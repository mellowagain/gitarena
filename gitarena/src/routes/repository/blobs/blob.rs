use crate::git::utils::{read_blob_content, repo_files_at_ref};
use crate::repository::{Branch, Repository};
use crate::routes::repository::blobs::BlobRequest;
use crate::{die, err};
use gitarena_common::database::Pool;

use std::sync::Arc;

use actix_web::http::header::{CONTENT_DISPOSITION, CONTENT_TYPE};
use actix_web::{HttpResponse, Responder, web};
use anyhow::Result;
use async_recursion::async_recursion;
use bstr::ByteSlice;
use gitarena_macros::route;
use gix::objs::tree::{EntryKind, EntryMode};
use gix::objs::{Tree, TreeRef};
use gix::odb::Store;
use gix::odb::pack::FindExt;

#[route("/{username}/{repository}/tree/{tree}/~blob/{blob:.*}", method = "GET", err = "text")]
pub(crate) async fn view_raw_blob(_repo: Repository, branch: Branch, uri: web::Path<BlobRequest>, db_pool: web::Data<Pool>) -> Result<impl Responder> {
    let transaction = db_pool.begin().await?;

    let gitoxide_repo = branch.gitoxide_repo;

    let mut buffer = Vec::<u8>::new();
    let mut blob_buffer = Vec::<u8>::new();

    let store = gitoxide_repo.objects.store().clone();

    let tree_ref = repo_files_at_ref(&branch.reference, store.clone(), &gitoxide_repo, &mut buffer).await?;
    let (_, content, _) = recursively_visit_blob_content(tree_ref, uri.blob.as_str(), store.clone(), &mut blob_buffer).await?;

    let mime = infer::get(content.as_bytes()).map_or("text/plain", |ty| ty.mime_type());

    transaction.commit().await?;

    Ok(HttpResponse::Ok()
        .insert_header((CONTENT_TYPE, mime))
        .insert_header((CONTENT_DISPOSITION, "inline"))
        .body(content))
}

#[async_recursion(?Send)]
async fn recursively_visit_blob_content<'a>(
    tree_ref: TreeRef<'a>,
    path: &str,
    store: Arc<Store>,
    buffer: &'a mut Vec<u8>,
) -> Result<(String, String, EntryMode)> {
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

        recursively_visit_blob_content(tree_ref, remaining, store, &mut buffer).await
    } else {
        if kind != EntryKind::Blob && kind != EntryKind::BlobExecutable {
            die!(BAD_REQUEST, "Only blobs can be viewed in blob view");
        }

        let file_name = entry.filename.to_str().unwrap_or("Invalid file name");

        Ok((file_name.to_owned(), read_blob_content(entry.oid.as_ref(), store).await?, entry.mode))
    }
}
