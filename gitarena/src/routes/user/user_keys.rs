use crate::ssh::SshKey;
use crate::user::User;
use actix_web::{web, Responder};
use gitarena_macros::route;
use itertools::Itertools;
use sqlx::PgPool;

#[route("/{user}.keys", method = "GET", err = "text")]
pub(crate) async fn get_keys(
    user: User,
    db_pool: web::Data<PgPool>,
) -> anyhow::Result<impl Responder> {
    let mut transaction = db_pool.begin().await?;

    let result = match SshKey::all_from_user(&user, &mut transaction).await {
        Some(keys) if keys.is_empty() => String::new(),
        Some(keys) => keys.into_iter().map(|key| key.as_string()).join("\n"),
        _ => String::new(),
    };

    transaction.commit().await?;

    Ok(result)
}
