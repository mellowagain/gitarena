use crate::database::Pool;
use crate::prelude::HttpRequestExtensions;
use crate::session::Session;
use crate::sso::SSO;
use crate::sso::sso_provider::SSOProvider;
use crate::sso::sso_provider_type::SSOProviderType;
use crate::user::{User, WebUser};
use crate::{die, err};

use std::str::FromStr;

use crate::events::Event;
use crate::mail::Email;
use crate::meili::MeiliClient;
use actix_identity::Identity;
use actix_web::http::header::LOCATION;
use actix_web::{HttpMessage, HttpRequest, HttpResponse, Responder, web};
use anyhow::{Context, Result};
use gitarena_macros::{from_config, route};
use oauth2::TokenResponse;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::debug;
use utoipa::ToSchema;

#[utoipa::path(
    get,
    path = "/api/sso",
    responses(
        (status = 200, description = "Which SSO providers are enabled on this instance", body = SSOProvidersResponse),
    ),
    tag = "user"
)]
#[route("/api/sso", method = "GET", err = "json")]
pub(crate) async fn get_sso_providers(db_pool: web::Data<Pool>) -> Result<impl Responder> {
    let (sso_github_enabled, sso_gitlab_enabled, sso_bitbucket_enabled) = from_config!(
        "sso.github.enabled" => bool,
        "sso.gitlab.enabled" => bool,
        "sso.bitbucket.enabled" => bool
    );

    Ok(HttpResponse::Ok().json(SSOProvidersResponse {
        github: sso_github_enabled,
        gitlab: sso_gitlab_enabled,
        bitbucket: sso_bitbucket_enabled,
    }))
}

/// Whether the specific SSO methods are enabled
#[derive(Serialize, ToSchema)]
pub(crate) struct SSOProvidersResponse {
    github: bool,
    gitlab: bool,
    bitbucket: bool,
}

#[route("/api/sso/{service}", method = "GET", err = "json")]
pub(crate) async fn initiate_sso(sso_request: web::Path<SSORequest>, web_user: WebUser, db_pool: web::Data<Pool>) -> Result<impl Responder> {
    if matches!(web_user, WebUser::Authenticated(_)) {
        die!(UNAUTHORIZED, "Already logged in");
    }

    let provider = SSOProviderType::from_str(sso_request.service.as_str()).map_err(|()| err!(BAD_REQUEST, "Unknown service"))?;
    let provider_impl = provider.get_implementation();

    // TODO: Save token in cache to check for CSRF
    let (url, _token) = SSOProvider::generate_auth_url(&*provider_impl, &provider, &db_pool).await?;

    Ok(HttpResponse::TemporaryRedirect().append_header((LOCATION, url.to_string())).finish())
}

#[route("/api/sso/{service}/callback", method = "GET", err = "json")]
pub(crate) async fn sso_callback(
    sso_request: web::Path<SSORequest>,
    id: Identity,
    request: HttpRequest,
    meili_client: web::Data<MeiliClient>,
    db_pool: web::Data<Pool>,
) -> Result<impl Responder> {
    if id.id().is_ok() {
        die!(UNAUTHORIZED, "Already logged in");
    }

    let provider = SSOProviderType::from_str(sso_request.service.as_str()).map_err(|()| err!(BAD_REQUEST, "Unknown service"))?;
    let provider_impl = provider.get_implementation();

    let query_string = request.q_string();
    let token_response = SSOProvider::exchange_response(&*provider_impl, &query_string, &provider, &db_pool).await?;

    if !SSOProvider::validate_scopes(&*provider_impl, token_response.scopes()) {
        die!(CONFLICT, "Not all required scopes have been granted");
    }

    let access_token = token_response.access_token();
    let token = access_token.secret();

    let mut transaction = db_pool.begin().await?;

    let provider_id = SSOProvider::get_provider_id(&*provider_impl, token.as_str()).await?;

    let sso = sqlx::query_as::<_, SSO>("select * from sso where provider = $1 and provider_id = $2 limit 1")
        .bind(&provider)
        .bind(provider_id.as_str())
        .fetch_optional(&mut *transaction)
        .await?;

    let user = match sso {
        Some(sso) => {
            // User link already exists -> Login user
            sqlx::query_as::<_, User>("select * from users where id = $1 limit 1")
                .bind(sso.user_id)
                .fetch_one(&mut *transaction)
                .await?
        }
        None => {
            // User link does not exist -> Create new user
            SSOProvider::create_user(&*provider_impl, token.as_str(), &request, &meili_client, &db_pool)
                .await
                .context("Failed to create new user using sso")?
        }
    };

    let primary_email = Email::find_primary_email(user.id, &mut transaction)
        .await?
        .ok_or_else(|| err!(UNAUTHORIZED, "No primary email"))?;

    if user.disabled {
        debug!(
            %provider,
            user.username,
            user.id = %user.id,
            "Received sso login request from disabled user"
        );

        die!(FORBIDDEN, "Account has been disabled. Please contact support.");
    }

    if !primary_email.is_allowed_login() {
        debug!(user.username, user.id = %user.id, "Received sso login request from unverified user");
        die!(FORBIDDEN, "Verify your email address before logging in.");
    }

    // We're now doing something *very* illegal: We're changing state in a GET request
    // For this reason we need additional protection in the form of CSRF tokens as "Same-Site: Lax" cookies
    // don't protect in this case against cross-site request forgery.

    let session = Session::new(&request, &user, &mut transaction).await?;
    Identity::login(&*request.extensions(), session.to_string())?;

    Event::new(
        "auth.login",
        user.id,
        &request,
        (&user).into(),
        Some(json!({
            "type": "sso",
            "provider": provider.to_string(),
        })),
    )
    .save(&mut transaction)
    .await?;

    debug!(
        %provider,
        user.username,
        user.id = %user.id,
        "User logged in through sso"
    );

    transaction.commit().await?;

    Ok(HttpResponse::Found().append_header((LOCATION, "/")).finish())
}

#[derive(Deserialize)]
pub(crate) struct SSORequest {
    service: String,
}
