use std::borrow::Borrow;

use anyhow::Result;
use async_recursion::async_recursion;
use git_hash::oid;
use git_object::TreeRef;
use git_odb::FindExt;
use git_pack::cache::DecodeEntry;
use git_ref::file::loose::Reference;
use git_repository::refs::Target;
use git_repository::Repository;

#[async_recursion(?Send)]
pub(crate) async fn repo_files_at_ref<'a>(repo: &'a Repository, reference: &Reference, buffer: &'a mut Vec<u8>, cache: &mut impl DecodeEntry) -> Result<TreeRef<'a>> {
    match &reference.target {
        Target::Peeled(object_id) => {
            let tree_oid = repo.odb.find_existing_commit(object_id.as_ref(), buffer, cache)?.tree();
            let tree = repo.odb.find_existing_tree(tree_oid.as_ref(), buffer, cache)?;

            Ok(tree)
        }
        Target::Symbolic(target) => {
            let reference = repo.refs.find_loose(target.to_partial())?;

            repo_files_at_ref(repo, &reference, buffer, cache).await
        }
    }
}

pub(crate) async fn repo_files_at_head<'a>(repo: &'a Repository, buffer: &'a mut Vec<u8>, cache: &mut impl DecodeEntry) -> Result<TreeRef<'a>> {
    repo_files_at_ref(repo, &repo.refs.find_loose("HEAD")?, buffer, cache).await
}

pub(crate) async fn read_blob_content(repo: &Repository, oid: &oid, cache: &mut impl DecodeEntry) -> Result<String> {
    let mut buffer = Vec::<u8>::new();

    repo.odb.find_existing_blob(oid, &mut buffer, cache).map(|blob| {
        // Honestly no idea how but this works out to yield valid file content
        // TODO: Maybe Git odb has some header and padding attached to the blob? Need to investigate
        let content_vec: Vec<u8> = blob.data.iter()
            .map(|i| *i)
            .skip(2)
            .filter(|b| *b != 0)
            .collect();

        let content = &content_vec[..content_vec.len() - 2];
        let cow = String::from_utf8_lossy(content);
        let file_content: &str = cow.borrow();

        Ok(file_content.to_owned())
    })?
}