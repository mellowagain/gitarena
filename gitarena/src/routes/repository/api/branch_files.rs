use crate::err;
use crate::git::GIT_HASH_KIND;
use crate::git::history::last_commit_for_blob;
use crate::git::utils::{read_blob_content, repo_files_at_ref};
use crate::prelude::LibGit2SignatureExtensions;
use crate::repository::{Branch, Repository};

use std::cmp::Ordering;
use std::sync::Arc;

use actix_web::{HttpResponse, Responder, web};
use anyhow::Result;
use async_recursion::async_recursion;
use bstr::ByteSlice;
use git2::Repository as Git2Repository;
use gitarena_common::database::{Database, Pool};
use gitarena_macros::route;
use gix::ObjectId;
use gix::objs::Tree;
use gix::objs::tree::EntryKind;
use gix::odb::Store;
use gix::odb::pack::FindExt;
use serde::Serialize;
use sqlx::Transaction;
use utoipa::ToSchema;

const MAX_ENTRIES: usize = 10_000;

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) enum FileType {
    /// Directory
    Tree,
    /// Regular file
    Blob,
    /// Executable file
    BlobExecutable,
    /// Symbolic link
    Link,
    /// Git submodule
    Commit,
}

impl From<EntryKind> for FileType {
    fn from(kind: EntryKind) -> Self {
        match kind {
            EntryKind::Tree => FileType::Tree,
            EntryKind::Blob => FileType::Blob,
            EntryKind::BlobExecutable => FileType::BlobExecutable,
            EntryKind::Link => FileType::Link,
            EntryKind::Commit => FileType::Commit,
        }
    }
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FileCommitInfo {
    /// SHA1 hash of the commit
    sha1: String,
    /// Commit message
    message: String,
    /// Unix timestamp of the commit
    time: i64,
    /// Author display name
    author_name: String,
    /// Author email address
    author_email: String,
    /// GitArena user ID of the author, if registered
    author_uid: Option<i32>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FileEntry {
    /// Type of the entry
    file_type: FileType,
    /// Full path of the entry relative to the repository root
    file_name: String,
    /// Target commit OID for submodule entries
    #[serde(skip_serializing_if = "Option::is_none")]
    submodule_target_oid: Option<String>,
    /// Last commit that touched this entry
    commit: FileCommitInfo,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BranchFilesResponse {
    /// File entries, capped at 10'000
    files: Vec<FileEntry>,
}

#[async_recursion(?Send)]
async fn collect_entries(
    tree: Tree,
    path_prefix: &str,
    store: Arc<Store>,
    libgit2_repo: &Git2Repository,
    full_tree_name: &str,
    transaction: &mut Transaction<'_, Database>,
    files: &mut Vec<FileEntry>,
    remaining: &mut usize,
) -> Result<()> {
    let mut entries = tree.entries;

    entries.sort_by(|lhs, rhs| {
        let lk = lhs.mode.kind();
        let rk = rhs.mode.kind();

        let is_tree = |k: EntryKind| matches!(k, EntryKind::Tree);
        let is_submodule = |k: EntryKind| matches!(k, EntryKind::Commit);

        if is_tree(lk) && !is_tree(rk) {
            Ordering::Less
        } else if !is_tree(lk) && is_tree(rk) {
            Ordering::Greater
        } else if is_tree(lk) && is_tree(rk) {
            lhs.filename.cmp(&rhs.filename)
        } else if is_submodule(lk) && !is_submodule(rk) {
            Ordering::Less
        } else if !is_submodule(lk) && is_submodule(rk) {
            Ordering::Greater
        } else {
            lhs.filename.cmp(&rhs.filename)
        }
    });

    for entry in &entries {
        if *remaining == 0 {
            break;
        }

        let name = entry.filename.to_str().unwrap_or("Invalid file name");
        let full_path = if path_prefix.is_empty() {
            name.to_owned()
        } else {
            format!("{}/{}", path_prefix, name)
        };

        let oid = last_commit_for_blob(libgit2_repo, full_tree_name, &full_path)
            .await?
            .ok_or_else(|| err!(INTERNAL_SERVER_ERROR, "No last commit found for blob"))?;
        let commit = libgit2_repo.find_commit(oid)?;

        let (author_name, author_uid, author_email) = commit.author().try_disassemble(transaction).await;

        let file_type = FileType::from(entry.mode.kind());
        let is_dir = matches!(file_type, FileType::Tree);

        let submodule_target_oid = if matches!(entry.mode.kind(), EntryKind::Commit) {
            Some(
                read_blob_content(entry.oid.as_ref(), store.clone())
                    .await
                    .unwrap_or_else(|_| ObjectId::null(GIT_HASH_KIND).to_string()),
            )
        } else {
            None
        };

        *remaining -= 1;

        files.push(FileEntry {
            file_type,
            file_name: full_path.clone(),
            submodule_target_oid,
            commit: FileCommitInfo {
                sha1: format!("{oid}"),
                message: commit.message().unwrap_or_default().to_owned(),
                time: commit.time().seconds(),
                author_name,
                author_email,
                author_uid,
            },
        });

        if is_dir {
            let mut subtree_buffer = Vec::<u8>::new();
            let (subtree_ref, _) = store.to_handle_arc().find_tree(entry.oid.as_ref(), &mut subtree_buffer)?;
            let subtree = Tree::from(subtree_ref);

            collect_entries(subtree, &full_path, store.clone(), libgit2_repo, full_tree_name, transaction, files, remaining).await?;
        }
    }

    Ok(())
}

#[utoipa::path(
    get,
    path = "/api/repos/{username}/{repository}/branch/{tree}/files",
    params(
        ("username" = String, Path, description = "Repository owner username"),
        ("repository" = String, Path, description = "Repository name"),
        ("tree" = String, Path, description = "Branch name (short form like 'main', or full ref like 'refs/heads/main')"),
    ),
    responses(
        (status = 200, description = "Recursive file listing with last commit info for each entry", body = BranchFilesResponse),
        (status = 404, description = "Repository or branch not found"),
    ),
    security((), ("cookieAuth" = [])),
    tag = "repository"
)]
#[route("/api/repos/{username}/{repository}/branch/{tree}/files", method = "GET", err = "json")]
pub(crate) async fn branch_files(repo: Repository, branch: Branch, db_pool: web::Data<Pool>) -> Result<impl Responder> {
    let mut transaction = db_pool.begin().await?;

    let libgit2_repo = repo.libgit2(&mut transaction).await?;
    let gitoxide_repo = branch.gitoxide_repo;
    let full_tree_name = branch.reference.name.as_bstr().to_str()?;

    let mut buffer = Vec::<u8>::new();
    let store = gitoxide_repo.objects.store().clone();

    let tree_ref = repo_files_at_ref(&branch.reference, store.clone(), &gitoxide_repo, &mut buffer).await?;
    let tree = Tree::from(tree_ref);

    let mut files = Vec::<FileEntry>::new();
    let mut remaining = MAX_ENTRIES;

    collect_entries(tree, "", store, &libgit2_repo, full_tree_name, &mut transaction, &mut files, &mut remaining).await?;

    Ok(HttpResponse::Ok().json(BranchFilesResponse { files }))
}
