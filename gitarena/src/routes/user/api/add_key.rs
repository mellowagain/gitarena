use crate::ssh::SshKey;
use crate::user::WebUser;
use crate::{die, err};
use gitarena_common::database::Pool;

use actix_web::{HttpResponse, Responder, web};
use anyhow::Context;
use anyhow::Result;
use chrono::serde::ts_seconds_option;
use chrono::{DateTime, Utc};
use gitarena_common::database::models::KeyType;
use gitarena_macros::route;
use openssh_keys::PublicKey;
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use utoipa::ToSchema;

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
pub(crate) async fn put_ssh_key(body: web::Json<AddKeyJsonRequest>, web_user: WebUser, db_pool: web::Data<Pool>) -> Result<impl Responder> {
    let user = web_user.into_user()?;
    let mut transaction = db_pool.begin().await?;

    if body.key.is_empty() {
        die!(BAD_REQUEST, "Key is not a valid argument");
    }

    let public_key = PublicKey::parse(body.key.as_str()).context("Failed to parse SSH public key")?;
    let algorithm = KeyType::try_from(public_key.keytype()).map_err(|_| err!(BAD_REQUEST, "Invalid or unsupported key type"))?;

    let key_title = if !body.title.is_empty() {
        &body.title
    } else if let Some(comment) = &public_key.comment {
        comment
    } else {
        die!(BAD_REQUEST, "Key requires a title");
    };

    let fingerprint = public_key.fingerprint_md5();

    if fingerprint.len() != 47 {
        warn!(length = fingerprint.len(), fingerprint, "Calculated md5 fingerprint is the wrong length",);
        die!(UNPROCESSABLE_ENTITY, "Calculated md5 fingerprint did not end up being 47 characters long");
    }

    let (exists,): (bool,) = sqlx::query_as("select exists(select 1 from ssh_keys where fingerprint = $1 limit 1)")
        .bind(fingerprint.as_str())
        .fetch_one(&mut *transaction)
        .await?;

    if exists {
        die!(CONFLICT, "SSH key already exists");
    }

    let key =
        sqlx::query_as::<_, SshKey>("insert into ssh_keys (owner, title, fingerprint, algorithm, key, expires_at) values ($1, $2, $3, $4, $5, $6) returning *")
            .bind(user.id)
            .bind(key_title)
            .bind(fingerprint.as_str())
            .bind(algorithm)
            .bind(public_key.data().as_slice())
            .bind(body.expiration_date)
            .fetch_one(&mut *transaction)
            .await?;

    transaction.commit().await?;

    debug!(user.id, key.id, key.title, key.fingerprint = fingerprint.as_str(), "New SSH key added by user",);

    Ok(HttpResponse::Created().json(AddKeyJsonResponse { id: key.id, fingerprint }))
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
    id: i32,
    /// MD5 fingerprint of the public key
    fingerprint: String,
}
