use crate::mail::Email;
use crate::prelude::HttpRequestExtensions;
use crate::session::Session;
use crate::sso::sso_provider::SSOProvider;
use crate::sso::sso_provider_type::SSOProviderType;
use crate::sso::SSO;
use crate::user::{User, WebUser};
use crate::{die, err};

use std::ops::Deref;
use std::str::FromStr;

use actix_identity::Identity;
use actix_web::http::header::LOCATION;
use actix_web::{web, HttpRequest, HttpResponse, Responder};
use anyhow::{Context, Result};
use gitarena_macros::route;
use log::debug;
use oauth2::TokenResponse;
use serde::Deserialize;
use sqlx::PgPool;

#[route("/sso/{service}", method = "GET", err = "html")]
pub(crate) async fn initiate_sso(
    sso_request: web::Path<SSORequest>,
    web_user: WebUser,
    db_pool: web::Data<PgPool>,
) -> Result<impl Responder> {
    if matches!(web_user, WebUser::Authenticated(_)) {
        die!(UNAUTHORIZED, "Already logged in");
    }

    let provider = SSOProviderType::from_str(sso_request.service.as_str())
        .map_err(|_| err!(BAD_REQUEST, "Unknown service"))?;
    let provider_impl = provider.get_implementation();

    // TODO: Save token in cache to check for CSRF
    let (url, _token) =
        SSOProvider::generate_auth_url(provider_impl.deref(), &provider, &db_pool).await?;

    Ok(HttpResponse::TemporaryRedirect()
        .append_header((LOCATION, url.to_string()))
        .finish())
}

#[route("/sso/{service}/callback", method = "GET", err = "html")]
pub(crate) async fn sso_callback(
    sso_request: web::Path<SSORequest>,
    id: Identity,
    request: HttpRequest,
    db_pool: web::Data<PgPool>,
) -> Result<impl Responder> {
    if id.identity().is_some() {
        die!(UNAUTHORIZED, "Already logged in");
    }

    let provider = SSOProviderType::from_str(sso_request.service.as_str())
        .map_err(|_| err!(BAD_REQUEST, "Unknown service"))?;
    let provider_impl = provider.get_implementation();

    let query_string = request.q_string();
    let token_response =
        SSOProvider::exchange_response(provider_impl.deref(), &query_string, &provider, &db_pool)
            .await?;

    if !SSOProvider::validate_scopes(provider_impl.deref(), token_response.scopes()) {
        die!(CONFLICT, "Not all required scopes have been granted");
    }

    let access_token = token_response.access_token();
    let token = access_token.secret();

    let mut transaction = db_pool.begin().await?;

    let provider_id = SSOProvider::get_provider_id(provider_impl.deref(), token.as_str()).await?;

    let sso: Option<SSO> = sqlx::query_as::<_, SSO>(
        "select * from sso where provider = $1 and provider_id = $2 limit 1",
    )
    .bind(&provider)
    .bind(provider_id.as_str())
    .fetch_optional(&mut transaction)
    .await?;

    let user = match sso {
        Some(sso) => {
            // User link already exists -> Login user
            sqlx::query_as::<_, User>("select * from users where id = $1 limit 1")
                .bind(sso.user_id)
                .fetch_one(&mut transaction)
                .await?
        }
        None => {
            // User link does not exist -> Create new user
            SSOProvider::create_user(provider_impl.deref(), token.as_str(), &db_pool)
                .await
                .context("Failed to create new user using sso")?
        }
    };

    let primary_email = Email::find_primary_email(&user, &mut transaction)
        .await?
        .ok_or_else(|| err!(UNAUTHORIZED, "No primary email"))?;

    if user.disabled || !primary_email.is_allowed_login() {
        debug!(
            "Received {} sso login request for disabled user {} (id {})",
            &provider, &user.username, &user.id
        );

        die!(
            FORBIDDEN,
            "Account has been disabled. Please contact support."
        );
    }

    // We're now doing something *very* illegal: We're changing state in a GET request
    // For this reason we need additional protection in the form of CSRF tokens as "Same-Site: Lax" cookies
    // don't protect in this case against cross-site request forgery.

    let session = Session::new(&request, &user, &mut transaction).await?;
    id.remember(session.to_string());

    debug!(
        "{} (id {}) logged in successfully using {} sso",
        &user.username, &user.id, &provider
    );

    transaction.commit().await?;

    Ok(HttpResponse::Found()
        .append_header((LOCATION, "/"))
        .finish())
}

#[derive(Deserialize)]
pub(crate) struct SSORequest {
    service: String,
}
