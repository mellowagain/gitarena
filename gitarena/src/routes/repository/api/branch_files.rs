use crate::err;
use crate::git::GIT_HASH_KIND;
use crate::git::history::batch_last_commits;
use crate::git::utils::{read_blob_content, repo_files_at_ref};
use crate::prelude::LibGit2SignatureExtensions;
use crate::repository::{Branch, Repository};

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use actix_web::{HttpResponse, Responder, web};
use anyhow::{Result, anyhow};
use bstr::ByteSlice;
use git2::Oid;
use gitarena_common::database::Pool;
use gitarena_macros::route;
use gix::ObjectId;
use gix::objs::Tree;
use gix::objs::tree::EntryKind;
use gix::odb::Store;
use gix::odb::pack::FindExt;
use serde::Serialize;
use tracing::instrument;
use utoipa::ToSchema;

const MAX_ENTRIES: usize = 10_000;

#[derive(Debug, Serialize, ToSchema)]
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

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
struct CommitInfo {
    /// Commit message
    message: String,
    /// Unix timestamp of the commit
    time: i64,
    /// Author display name
    author_name: String,
    /// Author email address
    author_email: String,
    /// GitArena user ID of the author
    author_uid: Option<i32>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FileCommitInfo {
    /// SHA1 hash of the commit
    sha1: String,
    #[serde(flatten)]
    #[schema(inline)]
    info: CommitInfo,
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
    /// Whether the file listing was truncated due to exceeding the 10'000 entry limit
    truncated: bool,
}

#[derive(Debug)]
struct CollectedEntry {
    file_type: FileType,
    file_name: String,
    entry_oid: ObjectId,
}

#[instrument(level = "trace", skip(store, files), fields(files = files.len()))]
fn collect_entries(tree: Tree, path_prefix: &str, store: &Arc<Store>, files: &mut Vec<CollectedEntry>, remaining: &mut usize) -> Result<()> {
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
            format!("{path_prefix}/{name}")
        };

        let file_type = FileType::from(entry.mode.kind());
        let is_dir = matches!(file_type, FileType::Tree);

        *remaining -= 1;

        files.push(CollectedEntry {
            file_type,
            file_name: full_path.clone(),
            entry_oid: entry.oid,
        });

        if is_dir {
            let mut subtree_buffer = Vec::<u8>::new();
            let (subtree_ref, _) = store.to_handle_arc().find_tree(entry.oid.as_ref(), &mut subtree_buffer)?;
            let subtree = Tree::from(subtree_ref);

            collect_entries(subtree, &full_path, store, files, remaining)?;
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

    let mut collected = Vec::<CollectedEntry>::new();
    let mut remaining = MAX_ENTRIES;

    collect_entries(tree, "", &store, &mut collected, &mut remaining)?;

    let paths: HashSet<String> = collected.iter().map(|e| e.file_name.clone()).collect();
    let path_to_commit = batch_last_commits(&libgit2_repo, full_tree_name, paths).await?;

    let unique_oids: HashSet<Oid> = path_to_commit.values().copied().collect();
    let mut commit_cache: HashMap<Oid, CommitInfo> = HashMap::with_capacity(unique_oids.len());

    for oid in unique_oids {
        let commit = libgit2_repo.find_commit(oid)?;
        let (author_name, author_uid, author_email) = commit.author().try_disassemble(&mut transaction).await;

        commit_cache.insert(
            oid,
            CommitInfo {
                message: commit.message().unwrap_or_default().to_owned(),
                time: commit.time().seconds(),
                author_name,
                author_uid,
                author_email,
            },
        );
    }

    let mut files = Vec::<FileEntry>::with_capacity(collected.len());

    for entry in collected {
        let commit_oid = path_to_commit
            .get(&entry.file_name)
            .copied()
            .ok_or_else(|| err!(INTERNAL_SERVER_ERROR, "No last commit found for blob"))?;

        let info = commit_cache.get(&commit_oid).ok_or_else(|| anyhow!("Commit cache miss for {commit_oid}"))?;

        let submodule_target_oid = if matches!(entry.file_type, FileType::Commit) {
            Some(
                read_blob_content(entry.entry_oid.as_ref(), store.clone())
                    .await
                    .unwrap_or_else(|_| ObjectId::null(GIT_HASH_KIND).to_string()),
            )
        } else {
            None
        };

        files.push(FileEntry {
            file_type: entry.file_type,
            file_name: entry.file_name,
            submodule_target_oid,
            commit: FileCommitInfo {
                sha1: format!("{commit_oid}"),
                info: info.clone(),
            },
        });
    }

    Ok(HttpResponse::Ok().json(BranchFilesResponse {
        files,
        truncated: remaining == 0,
    }))
}
