use crate::database::Pool;
use crate::mail::Email;
use crate::passkey::{ChallengeType, StoredPasskey, WebAuthnChallenge, aaguid_from_raw_credential, name_from_user_agent};
use crate::routes::user::api::auth::me::MeResponse;
use crate::session::{Session, send_login_email};
use crate::user::{User, WebUser};
use crate::{die, err};

use crate::events::Event;
use actix_identity::Identity;
use actix_web::{HttpRequest, HttpResponse, Responder, web};
use anyhow::{Context, Result, anyhow};
use fang::AsyncQueue;
use gitarena_macros::route;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tracing::debug;
use utoipa::ToSchema;
use uuid::Uuid;
use webauthn_rs::prelude::{
    CreationChallengeResponse, DiscoverableAuthentication, DiscoverableKey, PasskeyRegistration, PublicKeyCredential, RegisterPublicKeyCredential, Webauthn,
};

#[utoipa::path(
    get,
    path = "/api/auth/passkey",
    responses(
        (status = 200, description = "List of registered passkeys"),
        (status = 401, description = "Not authenticated"),
    ),
    security(("cookieAuth" = [])),
    tag = "user"
)]
#[route("/api/auth/passkey", method = "GET", err = "json")]
pub(crate) async fn get_passkeys(web_user: WebUser, db_pool: web::Data<Pool>) -> Result<impl Responder> {
    let WebUser::Authenticated(user) = web_user else {
        die!(UNAUTHORIZED, "Not logged in");
    };

    let mut tx = db_pool.begin().await?;

    let rows = StoredPasskey::find_for_user(user.id, &mut tx).await?;

    tx.commit().await?;

    let items: Vec<PasskeyListItem> = rows.into_iter().map(|key| PasskeyListItem { id: key.id, name: key.name }).collect();

    Ok(HttpResponse::Ok().json(items))
}

#[derive(Serialize, ToSchema)]
pub(crate) struct PasskeyListItem {
    pub(crate) id: Uuid,
    pub(crate) name: String,
}

#[utoipa::path(
    delete,
    path = "/api/auth/passkey/{id}",
    params(("id" = Uuid, Path, description = "Passkey ID")),
    responses(
        (status = 204, description = "Passkey deleted"),
        (status = 401, description = "Not authenticated"),
        (status = 404, description = "Passkey not found"),
    ),
    security(("cookieAuth" = [])),
    tag = "user"
)]
#[route("/api/auth/passkey/{id}", method = "DELETE", err = "json")]
pub(crate) async fn delete_passkey(path: web::Path<Uuid>, web_user: WebUser, request: HttpRequest, db_pool: web::Data<Pool>) -> Result<impl Responder> {
    let WebUser::Authenticated(user) = web_user else {
        die!(UNAUTHORIZED, "Not logged in");
    };

    let passkey_id = path.into_inner();

    let mut tx = db_pool.begin().await?;

    let passkey: Option<StoredPasskey> = sqlx::query_as("delete from passkeys where id = $1 and user_id = $2 returning *")
        .bind(passkey_id)
        .bind(user.id)
        .fetch_optional(&mut *tx)
        .await?;

    if let Some(passkey) = passkey {
        Event::new(
            "passkey.removed",
            user.id,
            &request,
            (&user).into(),
            Some(json!({
                "name": passkey.name,
            })),
        )
        .save(&mut tx)
        .await?;

        tx.commit().await?;

        debug!(user.username, user.id = %user.id, %passkey_id, "Deleted passkey");

        Ok(HttpResponse::NoContent().finish())
    } else {
        die!(NOT_FOUND, "Passkey not found");
    }
}

#[utoipa::path(
    post,
    path = "/api/auth/passkey/register/start",
    responses(
        (status = 200, description = "Registration challenge"),
        (status = 401, description = "Not authenticated"),
    ),
    security(("cookieAuth" = [])),
    tag = "user"
)]
#[route("/api/auth/passkey/register/start", method = "POST", err = "json")]
pub(crate) async fn post_register_start(web_user: WebUser, webauthn: web::Data<Webauthn>, db_pool: web::Data<Pool>) -> Result<impl Responder> {
    let WebUser::Authenticated(user) = web_user else {
        die!(UNAUTHORIZED, "Not logged in");
    };

    let mut tx = db_pool.begin().await?;

    let exclude_credentials: Vec<_> = StoredPasskey::find_for_user(user.id, &mut tx)
        .await?
        .into_iter()
        .filter_map(|s| s.into_passkey().ok())
        .map(|p| p.cred_id().clone())
        .collect();

    // we use uuid v5 with the user id because the web authn spec requires a stable user identifier
    let user_handle = Uuid::new_v5(&Uuid::NAMESPACE_OID, user.id.as_bytes());

    let (mut ccr, skr) = webauthn
        .start_passkey_registration(user_handle, &user.username, &user.username, Some(exclude_credentials))
        .context("Failed to start passkey registration")?;

    let authenticator_selection = ccr.public_key.authenticator_selection.get_or_insert_default();
    authenticator_selection.resident_key = None;
    authenticator_selection.require_resident_key = true;

    let challenge_id = Uuid::new_v4();
    WebAuthnChallenge::insert_registration(challenge_id, user.id, &skr, &mut tx).await?;

    tx.commit().await?;

    Ok(HttpResponse::Ok().json(RegisterStartResponse { challenge_id, options: ccr }))
}

#[derive(Serialize, ToSchema)]
pub(crate) struct RegisterStartResponse {
    pub(crate) challenge_id: Uuid,
    pub(crate) options: CreationChallengeResponse,
}

#[utoipa::path(
    post,
    path = "/api/auth/passkey/register/finish",
    request_body = RegisterFinishRequest,
    responses(
        (status = 201, description = "Passkey registered successfully"),
        (status = 400, description = "Invalid or expired challenge"),
        (status = 401, description = "Not authenticated"),
    ),
    security(("cookieAuth" = [])),
    tag = "user"
)]
#[route("/api/auth/passkey/register/finish", method = "POST", err = "json")]
pub(crate) async fn post_register_finish(
    body: web::Json<RegisterFinishRequest>,
    request: HttpRequest,
    web_user: WebUser,
    webauthn: web::Data<Webauthn>,
    db_pool: web::Data<Pool>,
) -> Result<impl Responder> {
    let WebUser::Authenticated(user) = web_user else {
        die!(UNAUTHORIZED, "Not logged in");
    };

    let mut tx = db_pool.begin().await?;

    let challenge = WebAuthnChallenge::take(body.challenge_id, ChallengeType::Registration, &mut tx)
        .await?
        .ok_or_else(|| err!(BAD_REQUEST, "Challenge not found"))?;

    if challenge.user_id != Some(user.id) {
        die!(UNAUTHORIZED, "Challenge does not belong to the authenticated user");
    }

    let reg_state: PasskeyRegistration = serde_json::from_value(challenge.state)?;

    let credential: RegisterPublicKeyCredential =
        serde_json::from_value(body.credential.clone()).map_err(|_| err!(BAD_REQUEST, "Invalid credential format"))?;

    let passkey = webauthn
        .finish_passkey_registration(&credential, &reg_state)
        .context("Passkey registration failed")?;

    let passkey_id = Uuid::now_v7();
    let credential_json = serde_json::to_value(&passkey)?;

    let ua = request.headers().get("user-agent").and_then(|v| v.to_str().ok()).unwrap_or("");
    let name = body
        .name
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .map(str::to_owned)
        .or_else(|| aaguid_from_raw_credential(&body.credential))
        .unwrap_or_else(|| name_from_user_agent(ua));

    sqlx::query("insert into passkeys (id, user_id, name, credential) values ($1, $2, $3, $4)")
        .bind(passkey_id)
        .bind(user.id)
        .bind(&name)
        .bind(credential_json)
        .execute(&mut *tx)
        .await?;

    Event::new(
        "passkey.added",
        user.id,
        &request,
        (&user).into(),
        Some(json!({
            "name": name,
        })),
    )
    .save(&mut tx)
    .await?;

    tx.commit().await?;

    debug!(user.username, user.id = %user.id, "Registered new passkey");

    Ok(HttpResponse::Created().finish())
}

#[derive(Deserialize, ToSchema)]
pub(crate) struct RegisterFinishRequest {
    pub(crate) challenge_id: Uuid,
    pub(crate) name: Option<String>,
    /// Type [`webauthn_rs_proto::attest::RegisterPublicKeyCredential`]
    // We don't use the typed struct here because we need to extract the attestation for the name manually before it gets serialized
    // as the serialization throws that value away
    pub(crate) credential: Value,
}

#[utoipa::path(
    post,
    path = "/api/auth/passkey/login/start",
    responses(
        (status = 200, description = "Authentication challenge"),
        (status = 409, description = "Already logged in"),
    ),
    tag = "user"
)]
#[route("/api/auth/passkey/login/start", method = "POST", err = "json")]
pub(crate) async fn post_login_start(web_user: WebUser, webauthn: web::Data<Webauthn>, db_pool: web::Data<Pool>) -> Result<impl Responder> {
    if matches!(web_user, WebUser::Authenticated(_)) {
        die!(CONFLICT, "Already logged in");
    }

    let mut tx = db_pool.begin().await?;

    let (rcr, auth_state) = webauthn
        .start_discoverable_authentication()
        .context("Failed to start discoverable authentication")?;

    let challenge_id = Uuid::new_v4();
    WebAuthnChallenge::insert_authentication(challenge_id, &auth_state, &mut tx).await?;

    tx.commit().await?;

    Ok(HttpResponse::Ok().json(LoginStartResponse {
        challenge_id,
        options: serde_json::to_value(&rcr)?,
    }))
}

#[derive(Serialize, ToSchema)]
pub(crate) struct LoginStartResponse {
    pub(crate) challenge_id: Uuid,
    pub(crate) options: Value,
}

#[utoipa::path(
    post,
    path = "/api/auth/passkey/login/finish",
    request_body = LoginFinishRequest,
    responses(
        (status = 200, description = "Login successful", body = MeResponse),
        (status = 400, description = "Invalid or expired challenge"),
        (status = 401, description = "Authentication failed"),
        (status = 403, description = "Account disabled"),
        (status = 404, description = "Could not find user with the passkey"),
        (status = 409, description = "Already logged in"),
    ),
    tag = "user"
)]
#[route("/api/auth/passkey/login/finish", method = "POST", err = "json")]
pub(crate) async fn post_login_finish(
    body: web::Json<LoginFinishRequest>,
    web_user: WebUser,
    request: HttpRequest,
    id: Identity,
    webauthn: web::Data<Webauthn>,
    queue: web::Data<AsyncQueue>,
    db_pool: web::Data<Pool>,
) -> Result<impl Responder> {
    if matches!(web_user, WebUser::Authenticated(_)) {
        die!(CONFLICT, "Already logged in");
    }

    let mut tx = db_pool.begin().await?;

    let challenge = WebAuthnChallenge::take(body.challenge_id, ChallengeType::Authentication, &mut tx)
        .await?
        .ok_or_else(|| err!(BAD_REQUEST, "Challenge not found"))?;

    let auth_state: DiscoverableAuthentication = serde_json::from_value(challenge.state)?;

    let (_, cred_id_bytes) = webauthn
        .identify_discoverable_authentication(&body.credential)
        .context("Failed to identify passkey")?;

    let cred_id_b64 = base64_url_encode(cred_id_bytes);

    let user_id = StoredPasskey::find_user_id_by_cred_id(&cred_id_b64, &mut tx)
        .await?
        .ok_or_else(|| err!(NOT_FOUND, "Unknown passkey credential"))?;

    let stored_passkeys = StoredPasskey::find_for_user(user_id, &mut tx).await?;
    let discoverable_keys: Vec<DiscoverableKey> = stored_passkeys.into_iter().filter_map(|s| s.into_passkey().ok()).map(Into::into).collect();

    let auth_result = webauthn
        .finish_discoverable_authentication(&body.credential, auth_state, &discoverable_keys)
        .map_err(|e| anyhow!("Passkey authentication failed: {e:?}"))?;

    let new_counter = i64::from(auth_result.counter());

    let name: String = sqlx::query_scalar(
        "update passkeys \
         set credential = jsonb_set(credential, '{cred,counter}', to_jsonb($1::bigint)) \
         where user_id = $2 and credential->'cred'->>'cred_id' = $3 \
         returning name",
    )
    .bind(new_counter)
    .bind(user_id)
    .bind(&cred_id_b64)
    .fetch_one(&mut *tx)
    .await?;

    let user: User = sqlx::query_as::<_, User>("select * from users where id = $1 limit 1")
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| err!(NOT_FOUND, "User not found for passkey"))?;

    let primary_email = Email::find_primary_email(user.id, &mut tx)
        .await?
        .ok_or_else(|| err!(UNAUTHORIZED, "No primary email"))?;

    if user.disabled {
        die!(FORBIDDEN, "Account has been disabled. Please contact support.");
    }

    if !primary_email.is_allowed_login() {
        die!(FORBIDDEN, "Verify your email address before logging in.");
    }

    let session = Session::new(&request, &user, &mut tx).await?;
    id.remember(session.to_string());

    Event::new(
        "auth.login",
        user.id,
        &request,
        (&user).into(),
        Some(json!({
            "type": "passkey",
            "passkey": name
        })),
    )
    .save(&mut tx)
    .await?;

    debug!(user.username, user.id = %user.id, "User logged in via passkey");

    tx.commit().await?;

    send_login_email(&user, "Passkey", &request, &queue, &db_pool).await?;

    Ok(HttpResponse::Ok().json(MeResponse {
        id: user.id,
        username: user.username,
        admin: user.admin,
    }))
}

#[derive(Deserialize, ToSchema)]
pub(crate) struct LoginFinishRequest {
    pub(crate) challenge_id: Uuid,
    pub(crate) credential: PublicKeyCredential,
}

fn base64_url_encode(bytes: &[u8]) -> String {
    base64::encode_config(bytes, base64::URL_SAFE_NO_PAD)
}
