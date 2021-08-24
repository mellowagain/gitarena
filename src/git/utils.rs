use crate::error::GAErrors::GitError;

use anyhow::Result;
use async_recursion::async_recursion;
use git_object::immutable::Tree;
use git_odb::FindExt;
use git_pack::cache::DecodeEntry;
use git_ref::file::loose::Reference;
use git_ref::mutable::Target;
use git_repository::Repository;

#[async_recursion(?Send)]
pub(crate) async fn repo_files_at_ref<'a>(repo: &'a Repository, reference: &Reference, buffer: &'a mut Vec<u8>, cache: &mut impl DecodeEntry) -> Result<Tree<'a>> {
    match &reference.target {
        Target::Peeled(object_id) => {
            let tree_oid = repo.odb.find_existing_commit(object_id.as_ref(), buffer, cache)?.tree();
            let tree = repo.odb.find_existing_tree(tree_oid.as_ref(), buffer, cache)?;

            Ok(tree)
        }
        Target::Symbolic(target) => match repo.refs.loose_find(target.to_partial())? {
            Some(reference) => repo_files_at_ref(repo, &reference, buffer, cache).await,
            None => Err(GitError(500, Some("Repo symlink points to invalid target".to_owned())).into())
        }
    }
}

pub(crate) async fn repo_files_at_head<'a>(repo: &'a Repository, buffer: &'a mut Vec<u8>, cache: &mut impl DecodeEntry) -> Result<Tree<'a>> {
    let reference_option = repo.refs.loose_find("HEAD")?;
    let reference = reference_option.ok_or(GitError(401, Some("Unable to find HEAD".to_owned())))?;

    repo_files_at_ref(repo, &reference, buffer, cache).await
}
