use crate::git::history::last_commit_for_blob;
use crate::git::utils::{read_raw_blob_content, repo_files_at_ref};
use crate::prelude::LibGit2SignatureExtensions;
use crate::repository::{Branch, Repository};
use crate::routes::repository::api::branch_files::{CommitInfo, FileCommitInfo};
use crate::{die, err};

use std::sync::Arc;

use actix_web::{HttpResponse, Responder, web};
use anyhow::Result;
use async_recursion::async_recursion;
use bstr::ByteSlice;
use gitarena_common::database::Pool;
use gitarena_macros::route;
use gix::objs::tree::EntryKind;
use gix::objs::{Tree, TreeRef};
use gix::odb::Store;
use gix::odb::pack::FindExt;
use infer::MatcherType;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

const MAX_TEXT_SIZE: usize = 5_000_000; // 5 MB

#[derive(Deserialize)]
struct FileContentRequest {
    namespace: String,
    repository: String,
    tree: String,
    file_path: String,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
struct FileContentResponse {
    /// File content as UTF-8. Null for binary files or files exceeding the 5 MB size limit
    content: Option<String>,
    /// File size in bytes
    size: usize,
    /// Whether the file was detected as binary (non-text) content
    is_binary: bool,
    /// Whether the content field was omitted because the file exceeded the 5 MB size limit
    is_truncated: bool,
    /// Last commit that touched this file
    commit: FileCommitInfo,
}

#[utoipa::path(
    get,
    path = "/api/repos/{namespace}/{repository}/branch/{tree}/files/{file_path}",
    params(
        ("namespace" = String, Path, description = "Repository namespace (user or organization)"),
        ("repository" = String, Path, description = "Repository name"),
        ("tree" = String, Path, description = "Branch name (short form like 'main', or full ref like 'refs/heads/main')"),
        ("file_path" = String, Path, description = "File path relative to the repository root. May contain slashes for files inside directories (e.g. 'src/main.rs' or 'a/b/c/file.txt')"),
    ),
    responses(
        (status = 200, description = "File content with size and last commit info", body = FileContentResponse),
        (status = 400, description = "Path points to a directory, not a file"),
        (status = 404, description = "Repository, branch, or file not found"),
    ),
    security((), ("cookieAuth" = [])),
    tag = "repository"
)]
#[route("/api/repos/{namespace}/{repository}/branch/{tree}/files/{file_path:.*}", method = "GET", err = "json")]
pub(crate) async fn file_content(repo: Repository, branch: Branch, uri: web::Path<FileContentRequest>, db_pool: web::Data<Pool>) -> Result<impl Responder> {
    let mut transaction = db_pool.begin().await?;

    let libgit2_repo = repo.libgit2(&mut transaction).await?;
    let gitoxide_repo = branch.gitoxide_repo;
    let full_tree_name = branch.reference.name.as_bstr().to_str()?;

    let mut buffer = Vec::<u8>::new();
    let mut blob_buffer = Vec::<u8>::new();

    let store = gitoxide_repo.objects.store().clone();

    let tree_ref = repo_files_at_ref(&branch.reference, store.clone(), &gitoxide_repo, &mut buffer).await?;
    let raw_bytes = recursively_find_blob(tree_ref, uri.file_path.as_str(), store, &mut blob_buffer).await?;

    let size = raw_bytes.len();
    let is_binary = infer::get(&raw_bytes).is_some_and(|ty| !matches!(ty.matcher_type(), MatcherType::Text | MatcherType::Doc));

    let (content, is_truncated) = if is_binary {
        (None, false)
    } else if size > MAX_TEXT_SIZE {
        (None, true)
    } else {
        (Some(String::from_utf8_lossy(&raw_bytes).into_owned()), false)
    };

    let commit_oid = last_commit_for_blob(&libgit2_repo, full_tree_name, uri.file_path.as_str())
        .await?
        .ok_or_else(|| err!(INTERNAL_SERVER_ERROR, "No commit found for file"))?;

    let commit = libgit2_repo.find_commit(commit_oid)?;
    let (author_name, author_uid, author_email) = commit.author().try_disassemble(&mut transaction).await;

    transaction.commit().await?;

    Ok(HttpResponse::Ok().json(FileContentResponse {
        content,
        size,
        is_binary,
        is_truncated,
        commit: FileCommitInfo {
            sha1: format!("{commit_oid}"),
            info: CommitInfo {
                message: commit.message().unwrap_or_default().to_owned(),
                time: commit.time().seconds(),
                author_name,
                author_uid,
                author_email,
            },
        },
    }))
}

#[async_recursion(?Send)]
async fn recursively_find_blob<'a>(tree_ref: TreeRef<'a>, path: &str, store: Arc<Store>, buffer: &'a mut Vec<u8>) -> Result<Vec<u8>> {
    let tree = Tree::from(tree_ref);
    let (search, remaining) = path.split_once('/').map_or_else(|| (path, None), |(a, b)| (a, Some(b)));

    let entry = tree.entries.iter().find(|e| e.filename == search).ok_or_else(|| err!(NOT_FOUND))?;
    let kind = entry.mode.kind();

    if let Some(remaining) = remaining {
        if kind != EntryKind::Tree {
            die!(NOT_FOUND);
        }

        let tree_ref = store.to_handle_arc().find_tree(entry.oid.as_ref(), buffer).map(|(tree, _)| tree)?;
        let mut buffer = Vec::<u8>::new();

        recursively_find_blob(tree_ref, remaining, store, &mut buffer).await
    } else {
        if kind != EntryKind::Blob && kind != EntryKind::BlobExecutable {
            die!(BAD_REQUEST, "Path is a directory, not a file");
        }

        read_raw_blob_content(entry.oid.as_ref(), store).await
    }
}
