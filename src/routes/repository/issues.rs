use crate::issue::Issue;
use crate::prelude::ContextExtensions;
use crate::privileges::privilege;
use crate::repository::Repository;
use crate::routes::repository::GitRequest;
use crate::user::{User, WebUser};
use crate::{die, err, render_template};

use std::collections::HashMap;

use actix_web::{Responder, web};
use anyhow::Result;
use gitarena_macros::route;
use itertools::Itertools;
use sqlx::PgPool;
use tera::Context;

#[route("/{username}/{repository}/issues", method = "GET", err = "html")]
pub(crate) async fn all_issues(uri: web::Path<GitRequest>, web_user: WebUser, db_pool: web::Data<PgPool>) -> Result<impl Responder> {
    let mut transaction = db_pool.begin().await?;

    let repo_owner = User::find_using_name(&uri.username, &mut transaction).await.ok_or_else(|| err!(NOT_FOUND, "Repository not found"))?;
    let repo = Repository::open(repo_owner, &uri.repository, &mut transaction).await.ok_or_else(|| err!(NOT_FOUND, "Repository not found")  )?;

    if !privilege::check_access(&repo, web_user.as_ref(), &mut transaction).await? {
        die!(NOT_FOUND, "Not found");
    }

    let confidential = if web_user.as_ref().map_or_else(|| false, |user| user.id == repo.owner) {
        "1 = 1"
    } else {
        "confidential = false"
    };

    let issues = sqlx::query_as::<_, Issue>(format!("select * from issues where repo = $1 and {} order by id desc", confidential).as_str())
        .bind(&repo.id)
        .fetch_all(&mut transaction)
        .await?;

    // This is really ugly and needs to be changed
    // TODO: Is there a way to map the original Issue struct to include these infos?
    let mut usernames = HashMap::new();

    for issue in issues.iter() {
        let (username,): (String,) = sqlx::query_as("select username from users where id = $1 limit 1")
            .bind(&issue.author)
            .fetch_one(&mut transaction)
            .await?;

        usernames.insert(format!("u{}", issue.author), username);

        // TODO: Serialize milestone and label strings here as well
        if !issue.assignees.is_empty() {
            // This workaround can be removed once Vec can be passed directly: https://github.com/launchbadge/sqlx/issues/875
            let haystack = &issue.assignees.iter().join(",");
            let query = format!("select id, username from users where id in ({})", haystack);

            let db_usernames: Vec<(i32, String)> = sqlx::query_as(query.as_str())
                .fetch_all(&mut transaction)
                .await?;

            for (id, username) in db_usernames.into_iter() {
                usernames.insert(format!("u{}", id), username);
            }
        }
    }

    let mut context = Context::new();

    context.try_insert("usernames", &usernames)?;

    context.try_insert("repo", &repo)?;
    context.try_insert("repo_owner_name", uri.username.as_str())?;

    context.try_insert("issues", &issues)?;
    context.insert_web_user(&web_user)?;

    // TODO: Change this to be infinite scrolling like commit list and explore?
    render_template!("repo/issues.html", context, transaction)
}
