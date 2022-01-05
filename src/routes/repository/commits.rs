use crate::git::history::{all_branches, all_commits, all_tags};
use crate::prelude::*;
use crate::privileges::privilege;
use crate::repository::Repository;
use crate::routes::repository::GitTreeRequest;
use crate::templates::web::GitCommit;
use crate::user::{User, WebUser};
use crate::{die, err, render_template};

use actix_web::{HttpRequest, Responder, web};
use anyhow::Result;
use bstr::ByteSlice;
use git_repository::refs::file::find::existing::Error as GitoxideFindError;
use gitarena_macros::route;
use sqlx::PgPool;
use tera::Context;

#[route("/{username}/{repository}/tree/{tree:.*}/commits", method = "GET", err = "htmx+html")]
pub(crate) async fn commits(uri: web::Path<GitTreeRequest>, web_user: WebUser, request: HttpRequest, db_pool: web::Data<PgPool>) -> Result<impl Responder> {
    let mut transaction = db_pool.begin().await?;

    let repo_owner = User::find_using_name(&uri.username, &mut transaction).await.ok_or_else(|| err!(NOT_FOUND, "Repository not found"))?;
    let repo = Repository::open(repo_owner, &uri.repository, &mut transaction).await.ok_or_else(|| err!(NOT_FOUND, "Repository not found")  )?;

    if !privilege::check_access(&repo, web_user.as_ref(), &mut transaction).await? {
        die!(NOT_FOUND, "Not found");
    }

    let gitoxide_repo = repo.gitoxide(&mut transaction).await?;

    let loose_ref = match gitoxide_repo.refs.find_loose(uri.tree.as_str()) {
        Ok(loose_ref) => Ok(loose_ref),
        Err(GitoxideFindError::Find(err)) => Err(err),
        Err(GitoxideFindError::NotFound(_)) => die!(NOT_FOUND, "Not found")
    }?; // Handle 404

    let full_tree_name = loose_ref.name.as_bstr().to_str()?;

    let query_string = request.q_string();
    let after_oid = query_string.get("after");
    let before_oid = query_string.get("before");

    let mut context = Context::new();

    context.try_insert("repo_owner_name", uri.username.as_str())?;
    context.try_insert("repo", &repo)?;
    context.try_insert("tree", uri.tree.as_str())?;

    let libgit2_repo = repo.libgit2(&mut transaction).await?;

    context.try_insert("branches", &all_branches(&libgit2_repo).await?)?;
    context.try_insert("tags", &all_tags(&libgit2_repo, None).await?)?;

    let searching_ref = after_oid.unwrap_or(full_tree_name);

    let commit_ids = all_commits(&libgit2_repo, searching_ref, 20).await?;
    let mut commits = Vec::<GitCommit>::with_capacity(commit_ids.len());

    for oid in commit_ids {
        let commit = libgit2_repo.find_commit(oid)?;
        let (name, uid, email) = commit.author().try_disassemble(&mut transaction).await;

        let chrono_time = commit.time().try_as_chrono()?;
        let chrono_date = chrono_time.date();
        let chrono_time_only_date = chrono_date.and_hms(0, 0, 0);

        commits.push(GitCommit {
            oid: format!("{}", commit.id()),
            message: commit.message().unwrap_or_default().to_owned(),
            time: commit.time().seconds(),
            date: Some(chrono_time_only_date),
            author_name: name,
            author_uid: uid,
            author_email: email
        });
    }

    if commits.is_empty() {
        // TODO: Render empty repo skeleton template showing how to push files to this repository
        die!(NOT_FOUND, "Not found");
    }

    if after_oid.is_some() || before_oid.is_some() {
        commits.remove(0); // Remove the first result as it contains the requested OID
    }

    context.try_insert("commits", &commits)?;

    // Only send a partial result (only the components) if it's a request by htmx
    if request.get_header("hx-request").is_some() {
        return render_template!("repo/commit_list_component.html", context, transaction);
    }

    render_template!("repo/commits.html", context, transaction)
}
