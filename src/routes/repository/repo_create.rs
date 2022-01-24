use crate::render_template;
use crate::user::WebUser;

use actix_web::{Responder, web};
use anyhow::Result;
use gitarena_macros::route;
use sqlx::PgPool;
use tera::Context;

#[route("/new", method = "GET", err = "html")]
pub(crate) async fn new_repo(web_user: WebUser, db_pool: web::Data<PgPool>) -> Result<impl Responder> {
    let user = web_user.into_user()?;
    let mut transaction = db_pool.begin().await?;

    let mut context = Context::new();

    context.try_insert("user", &user)?;

    render_template!("repo/create.html", context, transaction)
}
