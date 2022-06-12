use crate::git::history::{all_branches, all_tags, last_commit_for_blob};
use crate::git::utils::{read_blob_content, repo_files_at_ref};
use crate::prelude::{ContextExtensions, LibGit2SignatureExtensions};
use crate::repository::{Branch, Repository};
use crate::routes::repository::blobs::BlobRequest;
use crate::templates::web::{GitCommit, RepoFile};
use crate::user::WebUser;
use crate::utils::cookie_file::{CookieExtensions, FileType};
use crate::{die, err, render_template};

use std::sync::Arc;

use actix_web::http::header::CONTENT_TYPE;
use actix_web::{HttpResponse, Responder, web};
use anyhow::Result;
use async_recursion::async_recursion;
use bstr::ByteSlice;
use git_repository::objs::tree::EntryMode;
use git_repository::objs::{Tree, TreeRef};
use git_repository::odb::pack::FindExt;
use git_repository::odb::Store;
use git_repository::refs::file::loose::Reference;
use git_repository::Repository as GitoxideRepository;
use gitarena_macros::route;
use magic::Cookie;
use sqlx::PgPool;
use tera::Context;
use tracing_unwrap::OptionExt;

#[route("/{username}/{repository}/tree/{tree}/blob/{blob:.*}", method = "GET", err = "html")]
pub(crate) async fn view_blob(repo: Repository, branch: Branch, uri: web::Path<BlobRequest>, web_user: WebUser, cookie: web::Data<Arc<Cookie>>, db_pool: web::Data<PgPool>) -> Result<impl Responder> {
    let mut transaction = db_pool.begin().await?;

    let gitoxide_repo = branch.gitoxide_repo;
    let libgit2_repo = repo.libgit2(&mut transaction).await?;

    let full_tree_name = branch.reference.name.as_bstr().to_str()?;

    let mut buffer = Vec::<u8>::new();
    let mut blob_buffer = Vec::<u8>::new();

    let store = gitoxide_repo.objects.clone();

    let tree_ref = repo_files_at_ref(&branch.reference, store.clone(), &gitoxide_repo, &mut buffer).await?;
    let (name, content, mode) = recursively_visit_blob_content(&branch.reference, tree_ref, uri.blob.as_str(), &gitoxide_repo, store.clone(), &mut blob_buffer).await?;

    let oid = last_commit_for_blob(&libgit2_repo, full_tree_name, uri.blob.as_str()).await?.unwrap_or_log();
    let commit = libgit2_repo.find_commit(oid)?;
    let (author_name, author_uid, author_email) = commit.author().try_disassemble(&mut transaction).await;

    let mut context = Context::new();

    context.try_insert("file", &RepoFile {
        file_type: mode as u16,
        file_name: name.as_str(),
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

    context.try_insert("name", name.as_str())?;
    context.try_insert("full_path", uri.blob.as_str())?;

    render_template!("repo/blob/blob.html", context, transaction)
}

#[route("/{username}/{repository}/tree/{tree}/~blob/{blob:.*}", method = "GET", err = "text")]
pub(crate) async fn view_raw_blob(_repo: Repository, branch: Branch, uri: web::Path<BlobRequest>, cookie: web::Data<Arc<Cookie>>, db_pool: web::Data<PgPool>) -> Result<impl Responder> {
    let transaction = db_pool.begin().await?;

    let gitoxide_repo = branch.gitoxide_repo;

    let mut buffer = Vec::<u8>::new();
    let mut blob_buffer = Vec::<u8>::new();

    let store = gitoxide_repo.objects.clone();

    let tree_ref = repo_files_at_ref(&branch.reference, store.clone(), &gitoxide_repo, &mut buffer).await?;
    let (_, content, _) = recursively_visit_blob_content(&branch.reference, tree_ref, uri.blob.as_str(), &gitoxide_repo, store.clone(), &mut blob_buffer).await?;

    let mime = if let Some(file_type) = infer::get(content.as_bytes()) {
        file_type.mime_type()
    } else {
        match cookie.probe(content.as_bytes())? {
            FileType::Text => "text/plain",
            _ => "application/octet-stream"
        }
    };

    transaction.commit().await?;

    Ok(HttpResponse::Ok().insert_header((CONTENT_TYPE, mime)).body(content))
}

#[async_recursion(?Send)]
async fn recursively_visit_blob_content<'a>(reference: &Reference, tree_ref: TreeRef<'a>, path: &str, repo: &'a GitoxideRepository, store: Arc<Store>, buffer: &'a mut Vec<u8>) -> Result<(String, String, EntryMode)> {
    let tree = Tree::from(tree_ref);
    let (search, remaining) = path.split_once('/').map_or_else(|| (path, None), |(a, b)| (a, Some(b)));

    let entry = tree.entries
        .iter()
        .find(|e| e.filename == search)
        .ok_or_else(|| err!(NOT_FOUND))?;

    match remaining {
        Some(remaining) => {
            if entry.mode != EntryMode::Tree {
                die!(NOT_FOUND);
            }

            let tree_ref = store.to_handle_arc().find_tree(entry.oid.as_ref(), buffer).map(|(tree, _)| tree)?;
            let mut buffer = Vec::<u8>::new();

            recursively_visit_blob_content(reference, tree_ref, remaining, repo, store, &mut buffer).await
        }
        None => {
            if entry.mode != EntryMode::Blob && entry.mode != EntryMode::BlobExecutable  {
                die!(BAD_REQUEST, "Only blobs can be viewed in blob view");
            }

            let file_name = entry.filename.to_str().unwrap_or("Invalid file name");

            Ok((file_name.to_owned(), read_blob_content(entry.oid.as_ref(), store).await?, entry.mode))
        }
    }
}
