use crate::config::get_setting;
use crate::prelude::ContextExtensions;
use crate::user::WebUser;
use crate::{Ipc, die, render_template};
use gitarena_common::database::Pool;

use actix_web::{Responder, web};
use anyhow::Result;
use futures_locks::RwLock;
use gitarena_macros::route;

use tera::Context;

#[route("/new/import", method = "GET", err = "html")]
pub(crate) async fn import_repo(web_user: WebUser, ipc: web::Data<RwLock<Ipc>>, db_pool: web::Data<Pool>) -> Result<impl Responder> {
    let user = web_user.into_user()?;
    let mut transaction = db_pool.begin().await?;

    let enabled: bool = get_setting("repositories.importing_enabled", &mut transaction).await?;

    if !enabled || !ipc.read().await.is_connected() {
        die!(NOT_IMPLEMENTED, "Importing is disabled on this instance");
    }

    let mut context = Context::new();
    context.insert_user(&user)?;

    render_template!("repo/import.html", context, transaction)
}
