use crate::ssh::key::SshKey;
use crate::user::User;
use actix_web::{Responder, web};
use gitarena_common::database::Pool;
use gitarena_macros::route;
use itertools::Itertools;

#[route("/{user}.keys", method = "GET", err = "text")]
pub(crate) async fn get_keys(user: User, db_pool: web::Data<Pool>) -> anyhow::Result<impl Responder> {
    let mut transaction = db_pool.begin().await?;

    let result = match SshKey::all_from_user(&user, &mut transaction).await {
        Some(keys) if keys.is_empty() => String::new(),
        Some(keys) => keys.into_iter().map(|key| key.as_string()).join("\n"),
        _ => String::new(),
    };

    transaction.commit().await?;

    Ok(result)
}
