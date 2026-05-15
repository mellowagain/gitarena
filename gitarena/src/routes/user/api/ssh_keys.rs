use crate::database::Pool;
use crate::die;
use crate::ssh::key::SshKey;
use crate::user::WebUser;

use actix_web::{HttpResponse, Responder, web};
use anyhow::Result;
use gitarena_macros::route;
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
pub(crate) async fn delete_ssh_key(path: web::Path<Uuid>, web_user: WebUser, db_pool: web::Data<Pool>) -> Result<impl Responder> {
    let user = web_user.into_user()?;

    let key_id = path.into_inner();

    let mut transaction = db_pool.begin().await?;

    let rows = sqlx::query("delete from ssh_keys where id = $1 and owner = $2")
        .bind(key_id)
        .bind(user.id)
        .execute(&mut *transaction)
        .await?
        .rows_affected();

    if rows == 0 {
        die!(NOT_FOUND, "SSH key not found");
    }

    transaction.commit().await?;

    Ok(HttpResponse::NoContent().finish())
}
