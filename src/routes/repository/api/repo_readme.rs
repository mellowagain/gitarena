use crate::git::utils::{read_blob_content, repo_files_at_ref};
use crate::privileges::privilege;
use crate::repository::Repository;
use crate::routes::repository::GitTreeRequest;
use crate::user::{User, WebUser};
use crate::{die, err};

use actix_web::{HttpResponse, Responder, web};
use anyhow::Result;
use bstr::ByteSlice;
use git_repository::objs::Tree;
use git_repository::refs::file::find::existing::Error as GitoxideFindError;
use gitarena_macros::route;
use serde_json::json;
use sqlx::PgPool;

#[route("/api/repo/{username}/{repository}/tree/{tree:.*}/readme", method = "GET", err = "json")]
pub(crate) async fn readme(uri: web::Path<GitTreeRequest>, web_user: WebUser, db_pool: web::Data<PgPool>) -> Result<impl Responder> {
    let mut transaction = db_pool.begin().await?;

    let repo_owner = User::find_using_name(&uri.username, &mut transaction).await.ok_or_else(|| err!(NOT_FOUND, "Repository not found"))?;
    let repo = Repository::open(repo_owner, &uri.repository, &mut transaction).await.ok_or_else(|| err!(NOT_FOUND, "Repository not found"))?;

    if !privilege::check_access(&repo, web_user.as_ref(), &mut transaction).await? {
        die!(NOT_FOUND, "Not found");
    }

    let gitoxide_repo = repo.gitoxide(&mut transaction).await?;

    let loose_ref = match gitoxide_repo.refs.find_loose(uri.tree.as_str()) {
        Ok(loose_ref) => Ok(loose_ref),
        Err(GitoxideFindError::Find(err)) => Err(err),
        Err(GitoxideFindError::NotFound(_)) => die!(NOT_FOUND, "Tree not found")
    }?;

    let mut buffer = Vec::<u8>::new();
    let store = gitoxide_repo.objects.clone();

    let tree_ref = repo_files_at_ref(&loose_ref, store.clone(), &gitoxide_repo, &mut buffer).await?;
    let tree = Tree::from(tree_ref);

    let entry = tree.entries
        .iter()
        .find(|e| e.filename.to_lowercase().starts_with(b"readme"))
        .ok_or_else(|| err!(NOT_FOUND, "No readme file found"))?;

    let name = entry.filename.to_str().unwrap_or("Invalid file name");

    let content = read_blob_content(entry.oid.as_ref(), store).await?;

    Ok(HttpResponse::Ok().json(json!({
        "file_name": name,
        "content": content
    })))
}
