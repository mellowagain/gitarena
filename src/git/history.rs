use anyhow::Result;
use async_recursion::async_recursion;
use git2::{Oid, Repository as Git2Repository};
use git_odb::FindExt;

pub(crate) async fn last_commit_for_blob(repo: &Git2Repository, reference_name: &str, file_name: &str) -> Result<Option<Oid>> {
    let commits = commits_for_blob(repo, reference_name, file_name).await?;

    Ok(commits.last().map(|c| *c))
}

#[async_recursion(?Send)]
pub(crate) async fn last_commit_for_ref(repo: &Git2Repository, reference_name: &str) -> Result<Option<Oid>> {
    let reference = repo.find_reference(reference_name)?;

    if let Some(target) = reference.symbolic_target() {
        return last_commit_for_ref(repo, target).await;
    }

    Ok(reference.target())
}

pub(crate) async fn commits_for_blob(repo: &Git2Repository, reference: &str, file_name: &str) -> Result<Vec<Oid>> {
    let mut results = Vec::<Oid>::new();

    let mut rev_walk = repo.revwalk()?;
    rev_walk.set_sorting(git2::Sort::TIME)?;
    rev_walk.push_ref(reference)?;

    for result in rev_walk {
        let commit_oid = result?;
        let commit = repo.find_commit(commit_oid)?;
        let tree = commit.tree()?;

        if tree.get_name(file_name).is_some() {
            results.push(commit_oid);
        }
    }

    Ok(results)
}
