use crate::database::Pool;
use crate::die;
use crate::ssh::key::SshKey;
use crate::user::WebUser;

use crate::events::Event;
use actix_web::{HttpRequest, HttpResponse, Responder, web};
use anyhow::Result;
use gitarena_macros::route;
use serde_json::json;
use uuid::Uuid;

#[utoipa::path(
    get,
    path = "/api/ssh-keys",
    responses(
        (status = 200, description = "List of SSH keys for the authenticated user", body = Vec<SshKey>),
        (status = 401, description = "Authentication required"),
    ),
    security(("cookieAuth" = [])),
    tag = "user"
)]
#[route("/api/ssh-keys", method = "GET", err = "json")]
pub(crate) async fn get_ssh_keys(web_user: WebUser, db_pool: web::Data<Pool>) -> Result<impl Responder> {
    let user = web_user.into_user()?;
    let mut transaction = db_pool.begin().await?;

    let keys = SshKey::all_from_user(&user, &mut transaction).await.unwrap_or_default();

    transaction.commit().await?;

    Ok(HttpResponse::Ok().json(keys))
}

#[utoipa::path(
    delete,
    path = "/api/ssh-keys/{id}",
    params(("id" = Uuid, Path, description = "SSH key ID")),
    responses(
        (status = 204, description = "SSH key deleted"),
        (status = 401, description = "Authentication required"),
        (status = 404, description = "SSH key not found"),
    ),
    security(("cookieAuth" = [])),
    tag = "user"
)]
#[route("/api/ssh-keys/{id}", method = "DELETE", err = "json")]
pub(crate) async fn delete_ssh_key(path: web::Path<Uuid>, web_user: WebUser, request: HttpRequest, db_pool: web::Data<Pool>) -> Result<impl Responder> {
    let user = web_user.into_user()?;

    let key_id = path.into_inner();

    let mut transaction = db_pool.begin().await?;

    let key: Option<SshKey> = sqlx::query_as("delete from ssh_keys where id = $1 and owner = $2 returning *")
        .bind(key_id)
        .bind(user.id)
        .fetch_optional(&mut *transaction)
        .await?;

    if let Some(key) = key {
        Event::new(
            "ssh_key.removed",
            user.id,
            &request,
            (&user).into(),
            Some(json!({
                "title": key.title,
                "fingerprint": key.fingerprint
            })),
        )
        .save(&mut transaction)
        .await?;

        transaction.commit().await?;

        Ok(HttpResponse::NoContent().finish())
    } else {
        die!(NOT_FOUND, "SSH key not found");
    }
}
