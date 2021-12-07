use crate::error::GAErrors::HttpError;
use crate::render_template;
use crate::repository::Repository;
use crate::user::{User, WebUser};
use crate::prelude::HttpRequestExtensions;

use actix_web::{Responder, web};
use anyhow::Result;
use chrono::Duration;
use chrono_humanize::{Accuracy, HumanTime, Tense};
use git2::Version as LibGit2Version;
use gitarena_macros::route;
use heim::units::{Information, information, Time};
use sqlx::PgPool;
use tera::Context;
use actix_web::HttpRequest;


#[route("/explore", method = "GET")]
pub(crate) async fn explore(web_user: WebUser, db_pool: web::Data<PgPool>, request: HttpRequest) -> Result<impl Responder>  {
    
    let mut context = Context::new();
    let mut transaction = db_pool.begin().await?;
    let query_string = request.q_string();


    let (repos_count,): (i64,) = sqlx::query_as("select count(*) from 	repositories")
    .fetch_one(&mut transaction)
    .await?;

    context.try_insert("repository_count", &repos_count)?;

    let latest_repo_option: Option<Repository> = sqlx::query_as::<_, Repository>(format!("select * from repositories order by id desc"))
    .fetch_optional(&mut transaction)
    .await?;


    context.try_insert("repositories", &latest_repo_option)?;

    render_template!("explore/explore.html", context, transaction)
}