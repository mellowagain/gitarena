use crate::git::history::{
    all_branches, all_commits, all_tags, last_commit_for_blob, last_commit_for_ref,
};
use crate::git::utils::{read_blob_content, repo_files_at_ref};
use crate::git::GIT_HASH_KIND;
use crate::prelude::{ContextExtensions, LibGit2SignatureExtensions};
use crate::repository::{Branch, Repository};
use crate::routes::repository::blobs::BlobRequest;
use crate::templates::web::{GitCommit, RepoFile};
use crate::user::WebUser;
use crate::{die, err, render_template};

use std::cmp::Ordering;
use std::sync::Arc;

use actix_web::{web, Responder};
use anyhow::Result;
use async_recursion::async_recursion;
use bstr::ByteSlice;
use git_repository::objs::tree::EntryMode;
use git_repository::objs::{Tree, TreeRef};
use git_repository::odb::pack::FindExt;
use git_repository::odb::Store;
use git_repository::ObjectId;
use gitarena_macros::route;
use sqlx::PgPool;
use tera::Context;

#[route(
    "/{username}/{repository}/tree/{tree}/directory/{blob:.*}",
    method = "GET",
    err = "html"
)]
pub(crate) async fn view_dir(
    repo: Repository,
    branch: Branch,
    uri: web::Path<BlobRequest>,
    web_user: WebUser,
    db_pool: web::Data<PgPool>,
) -> Result<impl Responder> {
    let mut transaction = db_pool.begin().await?;

    let gitoxide_repo = branch.gitoxide_repo;
    let libgit2_repo = repo.libgit2(&mut transaction).await?;

    let full_tree_name = branch.reference.name.as_bstr().to_str()?;
    let mut context = Context::new();

    let mut tree_ref_buffer = Vec::<u8>::new();
    let mut tree_buffer = Vec::<u8>::new();
    let store = gitoxide_repo.objects.clone();

    let mut path = uri.blob.to_owned();
    path.push('/');

    let tree_ref = repo_files_at_ref(
        &branch.reference,
        store.clone(),
        &gitoxide_repo,
        &mut tree_ref_buffer,
    )
    .await?;
    let tree =
        recursively_visit_tree(tree_ref, path.as_str(), store.clone(), &mut tree_buffer).await?;

    let (issues_count,): (i64,) = sqlx::query_as(
        "select count(*) from issues where repo = $1 and closed = false and confidential = false",
    )
    .bind(repo.id)
    .fetch_one(&mut transaction)
    .await?;

    context.try_insert("repo", &repo)?;
    context.try_insert("repo_owner_name", uri.username.as_str())?;
    context.try_insert("issues_count", &issues_count)?;
    context.try_insert("merge_requests_count", &0_i32)?;
    context.try_insert("releases_count", &0_i32)?;
    context.try_insert("tree", uri.tree.as_str())?;
    context.try_insert("full_tree", full_tree_name)?;
    context.try_insert("branches", &all_branches(&libgit2_repo).await?)?;
    context.try_insert("tags", &all_tags(&libgit2_repo, None).await?)?;
    context.try_insert("name", uri.blob.as_str())?;
    context.insert_web_user(&web_user)?;

    // Should be generalized so we don't have this code twice but can re-use it in repo_view and here in directory

    let mut files = Vec::<RepoFile>::with_capacity(tree.entries.len().min(1000));

    for entry in tree.entries.iter().take(1000) {
        let name = entry.filename.to_str().unwrap_or("Invalid file name");
        let file_path = format!("{}/{}", uri.blob.as_str(), name);

        let oid = last_commit_for_blob(&libgit2_repo, full_tree_name, file_path.as_str())
            .await?
            .ok_or_else(|| {
                err!(
                    INTERNAL_SERVER_ERROR,
                    "No last commit found for blob (this should never happen)"
                )
            })?;
        let commit = libgit2_repo.find_commit(oid)?;

        let submodule_target_oid = if matches!(entry.mode, EntryMode::Commit) {
            Some(
                read_blob_content(entry.oid.as_ref(), store.clone())
                    .await
                    .unwrap_or_else(|_| ObjectId::null(GIT_HASH_KIND).to_string()),
            )
        } else {
            None
        };

        files.push(RepoFile {
            file_type: entry.mode as u16,
            file_name: name,
            submodule_target_oid,
            commit: GitCommit {
                oid: format!("{}", oid),
                message: commit.message().unwrap_or_default().to_owned(),
                time: commit.time().seconds(),
                date: None,
                author_name: String::new(),  // Unused for file listing
                author_uid: None,            // Unused for file listing
                author_email: String::new(), // Unused for file listing
            },
        });
    }

    files.sort_by(|lhs, rhs| {
        // 1. Directory
        // 2. Submodules
        // 3. Rest

        if lhs.file_type == EntryMode::Tree as u16 && rhs.file_type != EntryMode::Tree as u16 {
            Ordering::Less
        } else if lhs.file_type != EntryMode::Tree as u16 && rhs.file_type == EntryMode::Tree as u16
        {
            Ordering::Greater
        } else if lhs.file_type == EntryMode::Tree as u16 && rhs.file_type == EntryMode::Tree as u16
        {
            lhs.file_name.cmp(rhs.file_name)
        } else if lhs.file_type == EntryMode::Commit as u16
            && rhs.file_type != EntryMode::Commit as u16
        {
            Ordering::Less
        } else if lhs.file_type != EntryMode::Commit as u16
            && rhs.file_type == EntryMode::Commit as u16
        {
            Ordering::Greater
        } else {
            lhs.file_name.cmp(rhs.file_name)
        }
    });

    context.try_insert("files", &files)?;
    context.try_insert(
        "commits_count",
        &all_commits(&libgit2_repo, full_tree_name, 0).await?.len(),
    )?;

    let last_commit_oid = last_commit_for_ref(&libgit2_repo, full_tree_name)
        .await?
        .ok_or_else(|| err!(OK, "Repository is empty"))?;
    let last_commit = libgit2_repo.find_commit(last_commit_oid)?;

    // TODO: Additionally show last_commit.committer and if doesn't match with author
    let (author_name, author_uid, author_email) =
        last_commit.author().try_disassemble(&mut transaction).await;

    context.try_insert(
        "last_commit",
        &GitCommit {
            oid: format!("{}", last_commit_oid),
            message: last_commit.message().unwrap_or_default().to_owned(),
            time: last_commit.time().seconds(),
            date: None,
            author_name,
            author_uid,
            author_email,
        },
    )?;

    render_template!("repo/blob/directory.html", context, transaction)
}

#[async_recursion(?Send)]
async fn recursively_visit_tree<'a>(
    tree_ref: TreeRef<'a>,
    path: &str,
    store: Arc<Store>,
    buffer: &'a mut Vec<u8>,
) -> Result<Tree> {
    let tree = Tree::from(tree_ref);

    match path.split_once('/') {
        Some((search, remaining)) => {
            let entry = tree
                .entries
                .iter()
                .find(|e| e.filename == search)
                .ok_or_else(|| err!(NOT_FOUND, "Not found"))?;

            if entry.mode != EntryMode::Tree {
                die!(BAD_REQUEST, "Only trees can be viewed in tree view");
            }

            let tree_ref = store
                .to_handle_arc()
                .find_tree(entry.oid.as_ref(), buffer)
                .map(|(tree, _)| tree)?;
            let mut buffer = Vec::<u8>::new();

            recursively_visit_tree(tree_ref, remaining, store, &mut buffer).await
        }
        None => Ok(tree),
    }
}
