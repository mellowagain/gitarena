use std::borrow::Borrow;
use std::sync::Arc;

use anyhow::Result;
use async_recursion::async_recursion;
use gix::objs::TreeRef;
use gix::odb::Store;
use gix::odb::pack::FindExt;
use gix::refs::Target;
use gix::refs::file::loose::Reference;
use gix::{Repository, oid};
use tracing::instrument;

#[instrument(err, skip(store, repo))]
#[async_recursion(?Send)]
pub(crate) async fn repo_files_at_ref<'a>(reference: &Reference, store: Arc<Store>, repo: &'a Repository, buffer: &'a mut Vec<u8>) -> Result<TreeRef<'a>> {
    match &reference.target {
        Target::Object(object_id) => {
            let cache = store.to_cache_arc();

            let commit = cache.find_commit(object_id.as_ref(), buffer)?.0.tree();
            let (tree, _) = cache.find_tree(commit.as_ref(), buffer)?;

            Ok(tree)
        }
        Target::Symbolic(target) => {
            let reference = repo.refs.find_loose(target)?;

            repo_files_at_ref(&reference, store, repo, buffer).await
        }
    }
}

pub(crate) async fn repo_files_at_head<'a>(store: Arc<Store>, repo: &'a Repository, buffer: &'a mut Vec<u8>) -> Result<TreeRef<'a>> {
    let reference = repo.refs.find_loose("HEAD")?;

    repo_files_at_ref(&reference, store, repo, buffer).await
}

#[instrument(err, skip(store))]
pub(crate) async fn read_raw_blob_content(oid: &oid, store: Arc<Store>) -> Result<Vec<u8>> {
    let mut buffer = Vec::<u8>::new();
    store.to_cache_arc().find_blob(oid, &mut buffer)?;

    Ok(buffer)
}

#[instrument(err, skip(store))]
pub(crate) async fn read_blob_content(oid: &oid, store: Arc<Store>) -> Result<String> {
    let content = read_raw_blob_content(oid, store).await?;
    let cow = String::from_utf8_lossy(&content[..]);
    let file_content: &str = cow.borrow();

    Ok(file_content.to_owned())
}
