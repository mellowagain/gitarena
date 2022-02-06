use crate::git::GitoxideCacheList;
use crate::git::history::{all_branches, all_tags, last_commit_for_blob};
use crate::git::utils::{read_blob_content, repo_files_at_ref};
use crate::prelude::{ContextExtensions, LibGit2SignatureExtensions};
use crate::privileges::privilege;
use crate::repository::Repository;
use crate::routes::repository::blobs::BlobRequest;
use crate::templates::web::{GitCommit, RepoFile};
use crate::user::{User, WebUser};
use crate::utils::cookie_file::{CookieExtensions, FileType};
use crate::{die, err, render_template};

use actix_web::http::header::CONTENT_TYPE;
use actix_web::{HttpResponse, Responder, web};
use anyhow::Result;
use bstr::{BString, ByteSlice};
use git_repository::objs::Tree;
use git_repository::refs::file::find::existing::Error as GitoxideFindError;
use gitarena_macros::route;
use magic::Cookie;
use sqlx::PgPool;
use std::sync::Arc;
use tera::Context;
use tracing_unwrap::OptionExt;

#[route("/{username}/{repository}/tree/{tree}/blob/{blob}", method = "GET", err = "html")]
pub(crate) async fn view_blob(uri: web::Path<BlobRequest>, web_user: WebUser, cookie: web::Data<Arc<Cookie>>, db_pool: web::Data<PgPool>) -> Result<impl Responder> {
    let mut transaction = db_pool.begin().await?;

    let repo_owner = User::find_using_name(&uri.username, &mut transaction).await.ok_or_else(|| err!(NOT_FOUND, "Repository not found"))?;
    let repo = Repository::open(repo_owner, &uri.repository, &mut transaction).await.ok_or_else(|| err!(NOT_FOUND, "Repository not found"))?;

    if !privilege::check_access(&repo, web_user.as_ref(), &mut transaction).await? {
        die!(NOT_FOUND, "Not found");
    }

    let gitoxide_repo = repo.gitoxide(&mut transaction).await?;
    let libgit2_repo = repo.libgit2(&mut transaction).await?;

    let loose_ref = match gitoxide_repo.refs.find_loose(uri.tree.as_str()) {
        Ok(loose_ref) => Ok(loose_ref),
        Err(GitoxideFindError::Find(err)) => Err(err),
        Err(GitoxideFindError::NotFound(_)) => die!(NOT_FOUND, "Not found")
    }?;

    let full_tree_name = loose_ref.name.as_bstr().to_str()?;

    let mut buffer = Vec::<u8>::new();
    let mut cache = GitoxideCacheList::default();

    let tree = repo_files_at_ref(&gitoxide_repo, &loose_ref, &mut buffer, &mut cache).await?;
    let tree = Tree::from(tree);

    // TODO: Check if directories work with these
    let entry = tree.entries
        .iter()
        .find(|e| e.filename == BString::from(uri.blob.as_str()))
        .ok_or_else(|| err!(NOT_FOUND, "Not found"))?;

    let name = entry.filename.to_str().unwrap_or("Invalid file name");

    let oid = last_commit_for_blob(&libgit2_repo, full_tree_name, name).await?.unwrap_or_log();
    let commit = libgit2_repo.find_commit(oid)?;
    let (author_name, author_uid, author_email) = commit.author().try_disassemble(&mut transaction).await;

    let mut context = Context::new();

    context.try_insert("file", &RepoFile {
        file_type: entry.mode as u16,
        file_name: name,
        submodule_target_oid: None,
        commit: GitCommit {
            oid: format!("{}", oid),
            message: commit.message().unwrap_or_default().to_owned(),
            time: commit.time().seconds(),
            date: None,
            author_name,
            author_uid,
            author_email
        }
    })?;

    let content = read_blob_content(&gitoxide_repo, entry.oid.as_ref(), &mut cache).await?;
    let size = content.len();
    let file_type = cookie.probe(content.as_bytes())?;

    context.try_insert("type", &file_type)?;
    context.try_insert("size", &size)?;

    // We only display text files which are less than 2 MB
    if matches!(file_type, FileType::Text) && size < 2_000_000 {
        context.try_insert("content", content.as_str())?;
    }

    context.insert_web_user(&web_user)?;
    context.try_insert("repo_owner_name", uri.username.as_str())?;
    context.try_insert("repo", &repo)?;

    context.try_insert("tree", uri.tree.as_str())?;
    context.try_insert("branches", &all_branches(&libgit2_repo).await?)?;
    context.try_insert("tags", &all_tags(&libgit2_repo, None).await?)?;

    context.try_insert("name", name)?;

    render_template!("repo/blob/blob.html", context, transaction)
}

#[route("/{username}/{repository}/tree/{tree}/~blob/{blob}", method = "GET", err = "html")]
pub(crate) async fn view_raw_blob(uri: web::Path<BlobRequest>, web_user: WebUser, cookie: web::Data<Arc<Cookie>>, db_pool: web::Data<PgPool>) -> Result<impl Responder> {
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
        Err(GitoxideFindError::NotFound(_)) => die!(NOT_FOUND, "Not found")
    }?;

    let mut buffer = Vec::<u8>::new();
    let mut cache = GitoxideCacheList::default();

    let tree = repo_files_at_ref(&gitoxide_repo, &loose_ref, &mut buffer, &mut cache).await?;
    let tree = Tree::from(tree);

    // TODO: Check if directories work with these
    let entry = tree.entries
        .iter()
        .find(|e| e.filename == BString::from(uri.blob.as_str()))
        .ok_or_else(|| err!(NOT_FOUND, "Not found"))?;

    let content = read_blob_content(&gitoxide_repo, entry.oid.as_ref(), &mut cache).await?;

    let mime = if let Some(file_type) = infer::get(content.as_bytes()) {
        file_type.mime_type()
    } else {
        match cookie.probe(content.as_bytes())? {
            FileType::Text => "text/plain",
            _ => "application/octet-stream"
        }
    };

    Ok(HttpResponse::Ok().insert_header((CONTENT_TYPE, mime)).body(content))
}
