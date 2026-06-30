use crate::database::Pool;
use crate::ssh::key::{KeyType, SshKey};
use crate::user::WebUser;
use crate::{die, err};

use crate::events::Event;
use actix_web::{HttpRequest, HttpResponse, Responder, web};
use anyhow::Context;
use anyhow::Result;
use chrono::serde::ts_seconds_option;
use chrono::{DateTime, Utc};
use gitarena_macros::route;
use russh::keys::ssh_encoding::Encode;
use russh::keys::{HashAlg, PublicKey};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::debug;
use utoipa::ToSchema;
use uuid::Uuid;

#[utoipa::path(
    put,
    path = "/api/ssh-key",
    request_body = AddKeyJsonRequest,
    responses(
        (status = 201, description = "SSH key added successfully", body = AddKeyJsonResponse),
        (status = 400, description = "Invalid or unsupported key, missing title"),
        (status = 401, description = "Authentication required"),
        (status = 409, description = "SSH key already exists"),
        (status = 422, description = "Key fingerprint calculation failed"),
    ),
    security(("cookieAuth" = [])),
    tag = "user"
)]
#[route("/api/ssh-key", method = "PUT", err = "json")]
pub(crate) async fn put_ssh_key(
    body: web::Json<AddKeyJsonRequest>,
    web_user: WebUser,
    request: HttpRequest,
    db_pool: web::Data<Pool>,
) -> Result<impl Responder> {
    let user = web_user.into_user()?;
    let mut tx = db_pool.begin().await?;

    if body.key.is_empty() {
        die!(BAD_REQUEST, "Key is not a valid argument");
    }

    let public_key = PublicKey::from_openssh(body.key.as_str()).map_err(|err| {
        debug!(?err, input = %body.key.as_str(), "failed to parse public key");
        err!(BAD_REQUEST, "Failed to parse SSH public key")
    })?;

    let comment_str;
    let key_title: &str = if !body.title.is_empty() {
        &body.title
    } else {
        let comment = public_key.comment();
        if comment.is_empty() {
            die!(BAD_REQUEST, "Key requires a title");
        }
        comment_str = comment.to_string();
        &comment_str
    };

    let Ok(algorithm) = KeyType::try_from(&public_key.algorithm()) else {
        die!(BAD_REQUEST, "Unsupported key algorithm");
    };

    let fingerprint = public_key.fingerprint(HashAlg::Sha256).to_string();
    let fingerprint = fingerprint
        .strip_prefix("SHA256:")
        .expect("fingerprint to_string on sha256 to include sha256 prefix");

    let (exists,): (bool,) = sqlx::query_as("select exists(select 1 from ssh_keys where fingerprint = $1 limit 1)")
        .bind(fingerprint)
        .fetch_one(&mut *tx)
        .await?;

    if exists {
        die!(CONFLICT, "SSH key already exists");
    }

    let key_data = public_key.key_data();

    let mut buffer = Vec::with_capacity(key_data.encoded_len()?);
    key_data.encode(&mut buffer).context("failed to encode key data")?;

    let key = sqlx::query_as::<_, SshKey>(
        "insert into ssh_keys (id, owner, title, fingerprint, algorithm, key, expires_at) values ($1, $2, $3, $4, $5, $6, $7) returning *",
    )
    .bind(Uuid::now_v7())
    .bind(user.id)
    .bind(key_title)
    .bind(fingerprint)
    .bind(algorithm)
    .bind(buffer)
    .bind(body.expiration_date)
    .fetch_one(&mut *tx)
    .await?;

    Event::new(
        "ssh_key.added",
        user.id,
        &request,
        (&user).into(),
        Some(json!({
            "title": key_title,
            "fingerprint": fingerprint
        })),
    )
    .save(&mut tx)
    .await?;

    tx.commit().await?;

    debug!(user.id = %user.id, key.id = %key.id, key.title, key.algorithm = ?algorithm, key.fingerprint = fingerprint, "New SSH key added by user",);

    Ok(HttpResponse::Created().json(AddKeyJsonResponse {
        id: key.id,
        fingerprint: fingerprint.to_string(),
    }))
}

#[derive(Deserialize, ToSchema)]
pub(crate) struct AddKeyJsonRequest {
    /// Display title for the key
    #[schema(min_length = 1)]
    title: String,
    /// Full OpenSSH public key string
    #[schema(min_length = 1)]
    key: String,
    /// Optional expiration date as a Unix timestamp (seconds since epoch)
    #[serde(default, with = "ts_seconds_option")]
    #[schema(value_type = Option<i64>, example = 1_735_689_600)]
    expiration_date: Option<DateTime<Utc>>,
}

#[derive(Serialize, ToSchema)]
pub(crate) struct AddKeyJsonResponse {
    /// Internal ID of the newly added key
    id: Uuid,
    /// MD5 fingerprint of the public key
    fingerprint: String,
}
