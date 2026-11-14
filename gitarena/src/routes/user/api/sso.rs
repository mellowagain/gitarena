use crate::config::get_setting;
use crate::crypto::random_hex_string;
use crate::database::Pool;
use crate::events::Event;
use crate::meili::MeiliClient;
use crate::session::Session;
use crate::sso::provider::{SSO_PROVIDERS, SsoProvider, SsoProviderType};
use crate::user::{User, WebUser};
use crate::utils::identifiers::{is_namespace_taken, validate_namespace};
use crate::verification::send_verification_mail;
use crate::{die, sso};
use actix_identity::Identity;
use actix_web::http::header::LOCATION;
use actix_web::{HttpMessage, HttpRequest, HttpResponse, Responder, web};
use anyhow::Result;
use chrono::Local;
use fang::AsyncQueue;
use gitarena_macros::route;
use serde::Deserialize;
use serde_json::json;
use sqlx::types::Json;
use tracing::debug;
use uuid::Uuid;

#[utoipa::path(
    get,
    path = "/api/sso",
    summary = "List SSO Providers",
    responses(
        (status = 200, description = "Which SSO providers are enabled on this instance", body = Vec<String>),
    ),
    tag = "user"
)]
#[route("/api/sso", method = "GET", err = "json")]
pub(crate) async fn get_sso_providers() -> Result<impl Responder> {
    let providers = SSO_PROVIDERS
        .get()
        .map(|v| v.iter().map(SsoProviderType::name).collect::<Vec<_>>())
        .unwrap_or_default();
    Ok(HttpResponse::Ok().json(providers))
}

#[route("/api/sso/{service}", method = "GET", err = "json")]
pub(crate) async fn initiate_sso(service: web::Path<String>, web_user: WebUser, db_pool: web::Data<Pool>) -> Result<impl Responder> {
    if matches!(web_user, WebUser::Authenticated(_)) {
        die!(UNAUTHORIZED, "Already logged in");
    }

    let Some(provider) = sso::provider::get(&service) else {
        return Ok(HttpResponse::NotFound().finish());
    };

    let mut tx = db_pool.begin().await?;
    let auth_url_return = provider.auth_url().await?;

    sqlx::query("insert into sso_state (csrf_token, provider, state) values ($1, lower($2), $3)")
        .bind(auth_url_return.csrf_token)
        .bind(service.as_str())
        .bind(Json(auth_url_return.state))
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok(HttpResponse::TemporaryRedirect().append_header((LOCATION, auth_url_return.auth_url)).finish())
}

#[route("/api/sso/{service}/authorize", method = "GET", err = "json")]
pub(crate) async fn sso_callback(
    service: web::Path<String>,
    query: web::Query<CallbackQuery>,
    web_user: WebUser,
    request: HttpRequest,
    queue: web::Data<AsyncQueue>,
    meili_client: web::Data<MeiliClient>,
    db_pool: web::Data<Pool>,
) -> Result<impl Responder> {
    if matches!(web_user, WebUser::Authenticated(_)) {
        die!(UNAUTHORIZED, "Already logged in");
    }

    let Some(provider) = sso::provider::get(&service) else {
        return Ok(HttpResponse::NotFound().finish());
    };

    let mut tx = db_pool.begin().await?;

    let Json(state): Json<_> = sqlx::query_scalar("delete from sso_state where csrf_token = $1 and provider = lower($2) returning state")
        .bind(&query.state)
        .bind(service.as_str())
        .fetch_one(&mut *tx)
        .await?;

    let mut exchange_return = provider.exchange(query.code.clone(), state).await?;

    let user_id: Option<Uuid> = sqlx::query_scalar("select user_id from sso_connections where provider = lower($1) and provider_id = $2 limit 1")
        .bind(service.as_str())
        .bind(&exchange_return.subject_id)
        .fetch_optional(&mut *tx)
        .await?;

    let user = if let Some(user_id) = user_id
        && let Some(user) = User::find_using_id(user_id, &mut tx).await
    {
        user
    } else {
        if !get_setting::<bool>("allow_registrations", &mut tx).await? {
            die!(FORBIDDEN, "registrations are currently disabled");
        }

        validate_namespace(&exchange_return.username)?;

        if is_namespace_taken(&exchange_return.username, &mut tx).await? {
            exchange_return.username.push_str(&random_hex_string(6)?);
        }

        let user: User = sqlx::query_as("insert into users (id, username, password) values ($1, lower($2), $3) returning *")
            .bind(Uuid::now_v7())
            .bind(&exchange_return.username)
            .bind("sso-login")
            .fetch_one(&mut *tx)
            .await?;

        Event::new(
            "user.created",
            user.id,
            &request,
            (&user).into(),
            Some(json!({
                "origin": "sso",
                "sso_provider": service.to_lowercase(),
            })),
        )
        .save(&mut tx)
        .await?;

        let mut primary_flag = true;
        let verify_time = if exchange_return.emails_verified { Some(Local::now()) } else { None };

        for email_address in exchange_return.emails {
            if sqlx::query(
                "insert into emails (id, owner, email, \"primary\", commit, notification, public, verified_at) values ($1, $2, $3, $4, true, true, true, $5)",
            )
            .bind(Uuid::now_v7())
            .bind(user.id)
            .bind(&email_address)
            .bind(primary_flag)
            .bind(verify_time)
            .execute(&mut *tx)
            .await
            .is_err()
            {
                continue;
            }

            Event::new(
                "email.added",
                user.id,
                &request,
                (&user).into(),
                Some(json!({
                    "email": email_address,
                    "action": "account_create",
                    "source": "sso",
                    "sso_provider": service.to_lowercase(),
                })),
            )
            .save(&mut tx)
            .await?;

            if primary_flag {
                primary_flag = false;

                if verify_time.is_none() {
                    tx.commit().await?;

                    send_verification_mail(&user, email_address, &queue, &db_pool).await?;

                    tx = db_pool.begin().await?;
                }
            }
        }

        sqlx::query("insert into sso_connections (provider, provider_id, user_id) values (lower($1), $2, $3)")
            .bind(service.as_str())
            .bind(&exchange_return.subject_id)
            .bind(user.id)
            .execute(&mut *tx)
            .await?;

        user.index_meili(&meili_client).await;
        user
    };

    let session = Session::new(&request, &user, &mut tx).await?;

    Event::new(
        "auth.login",
        user.id,
        &request,
        (&user).into(),
        Some(json!({
            "type": "sso",
            "sso_provider": service.to_lowercase(),
        })),
    )
    .save(&mut tx)
    .await?;

    let frontend_url: String = get_setting("webauthn.origin", &mut tx).await?;

    tx.commit().await?;

    Identity::login(&request.extensions(), session.to_string())?;

    debug!(
        provider = service.as_str(),
        user.username,
        user.id = %user.id,
        "User logged in through sso"
    );

    Ok(HttpResponse::Found().append_header((LOCATION, frontend_url)).finish())
}

#[derive(Debug, Deserialize)]
struct CallbackQuery {
    code: String,
    state: String,
}
