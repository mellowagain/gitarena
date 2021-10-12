use crate::error::GAErrors::HttpError;
use crate::extensions::{get_user_by_identity, repo_from_str};
use crate::git::utils::{read_blob_content, repo_files_at_ref};
use crate::privileges::privilege;
use crate::routes::repository::GitTreeRequest;

use actix_identity::Identity;
use actix_web::{HttpResponse, Responder, web};
use anyhow::Result;
use bstr::ByteSlice;
use git_object::Tree;
use git_pack::cache::lru::MemoryCappedHashmap;
use git_ref::file::find::existing::Error as GitoxideFindError;
use gitarena_macros::route;
use serde_json::json;
use sqlx::PgPool;

#[route("/api/repo/{username}/{repository}/tree/{tree}/readme", method="GET")]
pub(crate) async fn readme(uri: web::Path<GitTreeRequest>, id: Identity, db_pool: web::Data<PgPool>) -> Result<impl Responder> {
    let (repo, mut transaction) = repo_from_str(&uri.username, &uri.repository, db_pool.begin().await?).await?;
    let user = get_user_by_identity(id.identity(), &mut transaction).await;

    if !privilege::check_access(&repo, user.as_ref(), &mut transaction).await? {
        return Err(HttpError(404, "Not found".to_owned()).into());
    }

    let gitoxide_repo = repo.gitoxide(&mut transaction).await?;

    let loose_ref = match gitoxide_repo.refs.find_loose(uri.tree.as_str()) {
        Ok(loose_ref) => Ok(loose_ref),
        Err(GitoxideFindError::Find(err)) => Err(err),
        Err(GitoxideFindError::NotFound(_)) => return Err(HttpError(404, "Tree not found".to_owned()).into())
    }?;

    let mut buffer = Vec::<u8>::new();
    let mut cache = MemoryCappedHashmap::new(10000 * 1024); // 10 MB

    let tree = repo_files_at_ref(&gitoxide_repo, &loose_ref, &mut buffer, &mut cache).await?;
    let tree = Tree::from(tree);

    let entry = tree.entries
        .iter()
        .filter(|e| e.filename.to_lowercase().starts_with(b"readme"))
        .next()
        .ok_or(HttpError(404, "No README found".to_owned()))?;

    let name = match entry.filename.to_str() {
        Ok(name) => name,
        Err(_) => "Invalid file name"
    };

    let content = read_blob_content(&gitoxide_repo, entry.oid.as_ref(), &mut cache).await?;

    Ok(HttpResponse::Ok().json(json!({
        "file_name": name,
        "content": content
    })))
}
