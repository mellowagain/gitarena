use crate::error::GAErrors::HttpError;
use crate::extensions::{bstr_to_str, get_user_by_identity, repo_from_str};
use crate::git::history::{all_commits, last_commit_for_blob, last_commit_for_ref};
use crate::git::utils::{read_blob_content, repo_files_at_ref};
use crate::render_template;
use crate::repository::Repository;
use crate::routes::repository::GitRequest;
use crate::templates::web::{GitCommit, RepoFile, RepoReadme};

use std::borrow::Borrow;
use std::cmp::Ordering;

use actix_identity::Identity;
use actix_web::{Responder, web};
use anyhow::Result;
use bstr::ByteSlice;
use git_hash::ObjectId;
use git_object::tree::EntryMode;
use git_object::Tree;
use git_pack::cache::lru::MemoryCappedHashmap;
use git_ref::file::find::existing::Error as GitoxideFindError;
use gitarena_macros::route;
use log::warn;
use serde::Deserialize;
use sqlx::{PgPool, Postgres, Transaction};
use tera::Context;

async fn render(tree_option: Option<&str>, repo: Repository, username: &str, id: Identity, mut transaction: Transaction<'_, Postgres>) -> Result<impl Responder> {
    let tree_name = tree_option.unwrap_or(repo.default_branch.as_str());
    let user = get_user_by_identity(id.identity(), &mut transaction).await;

    // TODO: Check for repo access for other people than owner
    if repo.private {
        if !user.as_ref().is_some() || user.as_ref().unwrap().id != repo.owner {
            return Err(HttpError(404, "Not found".to_owned()).into());
        }
    }

    let mut context = Context::new();

    let libgit2_repo = repo.libgit2(username).await?;
    let gitoxide_repo = repo.gitoxide(username).await?;

    let loose_ref = match gitoxide_repo.refs.find_loose(tree_name) {
        Ok(loose_ref) => Ok(loose_ref),
        Err(GitoxideFindError::Find(err)) => Err(err),
        Err(GitoxideFindError::NotFound(_)) => return Err(HttpError(404, "Not found".to_owned()).into())
    }?; // Handle 404

    let full_tree_name = bstr_to_str(loose_ref.name.as_bstr())?;

    let mut buffer = Vec::<u8>::new();
    let mut cache = MemoryCappedHashmap::new(10000 * 1024); // 10 MB

    let tree = repo_files_at_ref(&gitoxide_repo, &loose_ref, &mut buffer, &mut cache).await?;
    let tree = Tree::from(tree);

    let mut files = Vec::<RepoFile>::new();
    files.reserve(tree.entries.len().min(1000));

    for entry in tree.entries.iter().take(1000) {
        let name = match entry.filename.to_str() {
            Ok(name) => name,
            Err(_) => "Invalid file name"
        };

        let oid = last_commit_for_blob(&libgit2_repo, full_tree_name, name).await?.unwrap();
        let commit = libgit2_repo.find_commit(oid)?;

        let submodule_target_oid = if matches!(entry.mode, EntryMode::Commit) {
            Some(read_blob_content(&gitoxide_repo, entry.oid.as_ref(), &mut cache).await.unwrap_or(ObjectId::null_sha1().to_sha1_hex_string()))
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
                author_name: "", // Unused for file listing
                author_uid: None // Unused for file listing
            }
        });

        if name.to_lowercase().starts_with("readme") {
            match read_blob_content(&gitoxide_repo, entry.oid.as_ref(), &mut cache).await {
                Ok(file_content) => context.try_insert("readme", &RepoReadme {
                    file_name: name,
                    content: file_content.as_str()
                })?,
                Err(_) => warn!("Couldn't read {} file content", name)
            }
        }
    }

    files.sort_by(|lhs, rhs| {
        // 1. Directory
        // 2. Submodules
        // 3. Rest

        if lhs.file_type == EntryMode::Tree as u16 && rhs.file_type != EntryMode::Tree as u16 {
            Ordering::Less
        } else if lhs.file_type != EntryMode::Tree as u16 && rhs.file_type == EntryMode::Tree as u16 {
            Ordering::Greater
        } else if lhs.file_type == EntryMode::Tree as u16 && rhs.file_type == EntryMode::Tree as u16 {
            lhs.file_name.cmp(&rhs.file_name)
        } else if lhs.file_type == EntryMode::Commit as u16 && rhs.file_type != EntryMode::Commit as u16 {
            Ordering::Less
        } else if lhs.file_type != EntryMode::Commit as u16 && rhs.file_type == EntryMode::Commit as u16 {
            Ordering::Greater
        } else {
            lhs.file_name.cmp(&rhs.file_name)
        }
    });

    context.try_insert("repo", &repo)?;
    context.try_insert("repo_owner_name", &username)?;
    context.try_insert("repo_size", &repo.repo_size(username).await?)?;
    context.try_insert("files", &files)?;
    context.try_insert("tree", tree_name)?;
    context.try_insert("full_tree", full_tree_name)?;
    context.try_insert("issues_count", &0)?;
    context.try_insert("merge_requests_count", &0)?;
    context.try_insert("releases_count", &0)?;
    context.try_insert("commits_count", &all_commits(&libgit2_repo, full_tree_name).await?.len())?;

    if let Some(user) = user.as_ref() {
        context.try_insert("user", user)?;
    }

    let last_commit_oid = last_commit_for_ref(&libgit2_repo, full_tree_name).await?.ok_or(HttpError(200, "Repository is empty".to_owned()))?;
    let last_commit = libgit2_repo.find_commit(last_commit_oid)?;

    let author_option: Option<(i32, String)> = sqlx::query_as("select id, username from users where lower(email) = lower($1)")
        .bind(last_commit.author().email().unwrap_or("Invalid e-mail address"))
        .fetch_optional(&mut transaction)
        .await?;

    let author_name;
    let author_uid;

    if let Some((user_id, username)) = author_option {
        author_uid = Some(user_id);
        author_name = username;
    } else {
        author_uid = None;
        author_name = last_commit.author().name().unwrap_or("Ghost").to_owned();
    }

    context.try_insert("last_commit", &GitCommit {
        oid: format!("{}", last_commit_oid),
        message: last_commit.message().unwrap_or_default().to_owned(),
        time: last_commit.time().seconds(),
        author_name: author_name.as_str(),
        author_uid
    })?;

    render_template!("repo/index.html", context, transaction)
}

#[route("/{username}/{repository}/tree/{tree}", method="GET")]
pub(crate) async fn view_repo_tree(uri: web::Path<RepoViewRequest>, id: Identity, db_pool: web::Data<PgPool>) -> Result<impl Responder> {
    let (repo, transaction) = repo_from_str(&uri.username, &uri.repository, db_pool.begin().await?).await?;

    render(Some(uri.tree.as_str()), repo, &uri.username, id, transaction).await
}

#[route("/{username}/{repository}", method="GET")]
pub(crate) async fn view_repo(uri: web::Path<GitRequest>, id: Identity, db_pool: web::Data<PgPool>) -> Result<impl Responder> {
    let (repo, transaction) = repo_from_str(&uri.username, &uri.repository, db_pool.begin().await?).await?;

    render(None, repo, &uri.username, id, transaction).await
}

#[derive(Deserialize)]
pub(crate) struct RepoViewRequest {
    pub(crate) username: String,
    pub(crate) repository: String,
    pub(crate) tree: String
}
