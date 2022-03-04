use std::str::FromStr;
use std::sync::Arc;

use anyhow::Result;
use async_recursion::async_recursion;
use git2::{DiffOptions, Oid, Repository as Git2Repository, Sort};
use git_repository::odb::pack::FindExt;
use git_repository::odb::Store;
use git_repository::refs::Target;
use git_repository::traverse::commit::ancestors::State;
use git_repository::traverse::commit::{Ancestors, Parents, Sorting};
use git_repository::{ObjectId, Repository as GitoxideRepository};
use tracing::instrument;

#[instrument(err, skip(repo))]
pub(crate) async fn last_commit_for_blob(repo: &Git2Repository, reference_name: &str, file_name: &str) -> Result<Option<Oid>> {
    let commits = commits_for_blob(repo, reference_name, file_name, Some(1)).await?;

    Ok(commits.get(0).copied())
}

#[instrument(err, skip(repo))]
#[async_recursion(?Send)]
pub(crate) async fn last_commit_for_ref(repo: &Git2Repository, reference_name: &str) -> Result<Option<Oid>> {
    let reference = repo.find_reference(reference_name)?;

    if let Some(target) = reference.symbolic_target() {
        return last_commit_for_ref(repo, target).await;
    }

    Ok(reference.target())
}

#[instrument(err, skip(repo))]
pub(crate) async fn commits_for_blob(repo: &Git2Repository, reference: &str, file_name: &str, max_results: Option<usize>) -> Result<Vec<Oid>> {
    let mut results = Vec::<Oid>::new();

    if let Some(max) = max_results {
        results.reserve(max);
    }

    let mut rev_walk = repo.revwalk()?;
    rev_walk.set_sorting(Sort::TIME)?;
    rev_walk.push_ref(reference)?;

    'outer: for result in rev_walk {
        let commit_oid = result?;
        let commit = repo.find_commit(commit_oid)?;
        let tree = commit.tree()?;

        let previous_tree = if commit.parent_count() > 0 {
            let previous_commit = commit.parent(0)?;
            let previous_tree = previous_commit.tree()?;

            Some(previous_tree)
        } else {
            None
        };

        let mut diff_options = DiffOptions::new();
        diff_options.enable_fast_untracked_dirs(true);
        diff_options.skip_binary_check(true);
        diff_options.pathspec(file_name);

        let diff = repo.diff_tree_to_tree(previous_tree.as_ref(), Some(&tree), Some(&mut diff_options))?;

        for _ in diff.deltas() {
            results.push(commit_oid);

            if let Some(max) = max_results {
                if results.len() >= max {
                    break 'outer;
                }
            }
        }
    }

    Ok(results)
}

/// `reference` can be either a full ref name or a OID string (ascii-hex-numeric, 40 digits)
/// Returns at most `limit` commits or all commits if `limit == 0`
#[instrument(err, skip(store, state, repo))]
pub(crate) async fn all_commits(reference: &str, store: Arc<Store>, state: &mut State, repo: &GitoxideRepository, limit: usize) -> Result<Vec<ObjectId>> {
    let mut results = Vec::<ObjectId>::with_capacity(limit);
    let tip = ObjectId::from_str(reference).or_else(|_| resolve_into_id(repo, repo.refs.find_loose(reference)?.target))?;

    let cache = store.to_cache_arc();

    let ancestors = Ancestors::new(
        Some(tip), // Option<T> implements IntoIterator<T> so this is totally ok
        state,
        |oid, buffer| cache.find_commit_iter(oid, buffer).ok().map(|(commit, _)| commit)
    ).sorting(Sorting::ByCommitterDate).parents(Parents::First);

    for ancestor in ancestors {
        results.push(ancestor?);

        if limit > 0 && results.len() >= limit {
            break;
        }
    }

    Ok(results)
}

#[instrument(err, skip(repo))]
pub(crate) async fn all_branches(repo: &Git2Repository) -> Result<Vec<String>> {
    let mut results = Vec::<String>::new();

    for reference in repo.references()? {
        let reference = reference?;

        if let Some(name) = reference.name() {
            results.push(name.replacen("refs/heads/", "", 1));
        }

    }

    Ok(results)
}

#[instrument(err, skip(repo))]
pub(crate) async fn all_tags(repo: &Git2Repository, prefix: Option<&str>) -> Result<Vec<String>> {
    let tags = repo.tag_names(prefix)?;

    Ok(tags.iter()
        .filter_map(|o| o.map(|o| o.to_owned()))
        .collect())
}

/// Recursively resolves symbolic refs until hitting a peeled target
fn resolve_into_id(repo: &GitoxideRepository, target: Target) -> Result<ObjectId> {
    match target {
        Target::Peeled(oid) => Ok(oid),
        Target::Symbolic(target) => {
            let reference = repo.refs.find_loose(target.to_partial())?;
            resolve_into_id(repo, reference.target)
        }
    }
}
