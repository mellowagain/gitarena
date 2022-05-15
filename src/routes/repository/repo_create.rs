use crate::prelude::ContextExtensions;
use crate::{Ipc, render_template};
use crate::user::WebUser;

use actix_web::{Responder, web};
use anyhow::Result;
use futures_locks::RwLock;
use gitarena_macros::route;
use sqlx::PgPool;
use tera::Context;

#[route("/new", method = "GET", err = "html")]
pub(crate) async fn new_repo(web_user: WebUser, ipc: web::Data<RwLock<Ipc>>, db_pool: web::Data<PgPool>) -> Result<impl Responder> {
    let user = web_user.into_user()?;
    let mut transaction = db_pool.begin().await?;

    let mut context = Context::new();
    context.insert_user(&user)?;

    context.try_insert("ipc_enabled", &ipc.read().await.is_connected())?;

    render_template!("repo/create.html", context, transaction)
}
