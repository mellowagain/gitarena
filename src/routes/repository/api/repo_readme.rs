use crate::err;
use crate::git::utils::{read_blob_content, repo_files_at_ref};
use crate::repository::{Branch, Repository};

use actix_web::{HttpResponse, Responder};
use anyhow::Result;
use bstr::ByteSlice;
use git_repository::objs::Tree;
use gitarena_macros::route;
use serde_json::json;

#[route("/api/repo/{username}/{repository}/tree/{tree:.*}/readme", method = "GET", err = "json")]
pub(crate) async fn readme(_repo: Repository, branch: Branch) -> Result<impl Responder> {
    let gitoxide_repo = branch.gitoxide_repo;

    let mut buffer = Vec::<u8>::new();
    let store = gitoxide_repo.objects.clone();

    let tree_ref = repo_files_at_ref(&branch.reference, store.clone(), &gitoxide_repo, &mut buffer).await?;
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
