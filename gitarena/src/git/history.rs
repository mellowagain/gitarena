use anyhow::Result;
use async_recursion::async_recursion;
use git2::{DiffOptions, Oid, Repository as Git2Repository, Sort};
use std::collections::{HashMap, HashSet};
use tracing::instrument;

#[instrument(err, skip(repo))]
pub(crate) async fn last_commit_for_blob(repo: &Git2Repository, reference_name: &str, file_name: &str) -> Result<Option<Oid>> {
    let commits = commits_for_blob(repo, reference_name, file_name, Some(1)).await?;

    Ok(commits.first().copied())
}

#[instrument(err, skip(repo, paths))]
pub(crate) async fn batch_last_commits(repo: &Git2Repository, reference: &str, paths: HashSet<String>) -> Result<HashMap<String, Oid>> {
    let mut pending = paths;
    let mut results = HashMap::<String, Oid>::with_capacity(pending.len());

    let mut rev_walk = repo.revwalk()?;
    rev_walk.set_sorting(Sort::TIME)?;
    rev_walk.push_ref(reference)?;

    let mut diff_options = DiffOptions::new();
    diff_options.enable_fast_untracked_dirs(true);
    diff_options.skip_binary_check(true);

    for result in rev_walk {
        if pending.is_empty() {
            break;
        }

        let commit_oid = result?;
        let commit = repo.find_commit(commit_oid)?;
        let tree = commit.tree()?;

        let previous_tree = if commit.parent_count() > 0 {
            let previous_commit = commit.parent(0)?;
            Some(previous_commit.tree()?)
        } else {
            None
        };

        let diff = repo.diff_tree_to_tree(previous_tree.as_ref(), Some(&tree), Some(&mut diff_options))?;

        for delta in diff.deltas() {
            let Some(path) = delta.new_file().path() else { continue };

            let mut path_str = path.to_string_lossy().into_owned();

            loop {
                if pending.remove(&path_str) {
                    results.insert(path_str.clone(), commit_oid);
                }

                match path_str.rfind('/') {
                    Some(pos) => path_str = path_str[..pos].to_owned(),
                    None => break,
                }
            }
        }
    }

    Ok(results)
}

#[instrument(err, skip(repo))]
#[async_recursion(?Send)]
pub(crate) async fn last_commit_for_ref(repo: &Git2Repository, reference_name: &str) -> Result<Option<Oid>> {
    let reference = repo.find_reference(reference_name)?;

    if let Some(target) = reference.symbolic_target()? {
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

            if let Some(max) = max_results
                && results.len() >= max
            {
                break 'outer;
            }
        }
    }

    Ok(results)
}

/// `reference` can be either a full ref name or a OID string (ascii-hex-numeric, 40 digits)
/// Returns a page of commits that touched `path`, starting at `offset`, returning at most `limit` entries
#[instrument(err, skip(repo))]
pub(crate) async fn paged_commits_for_path(repo: &Git2Repository, reference: &str, path: &str, offset: usize, limit: usize) -> Result<Vec<Oid>> {
    let mut results = Vec::<Oid>::with_capacity(limit);
    let mut skipped = 0usize;

    let mut rev_walk = repo.revwalk()?;
    rev_walk.set_sorting(Sort::TIME)?;

    match Oid::from_str(reference) {
        Ok(oid) => rev_walk.push(oid)?,
        Err(_) => rev_walk.push_ref(reference)?,
    }

    let mut diff_options = DiffOptions::new();
    diff_options.enable_fast_untracked_dirs(true);
    diff_options.skip_binary_check(true);
    diff_options.pathspec(path);

    for result in rev_walk {
        let commit_oid = result?;
        let commit = repo.find_commit(commit_oid)?;
        let tree = commit.tree()?;

        let previous_tree = if commit.parent_count() > 0 {
            let previous_commit = commit.parent(0)?;
            Some(previous_commit.tree()?)
        } else {
            None
        };

        let diff = repo.diff_tree_to_tree(previous_tree.as_ref(), Some(&tree), Some(&mut diff_options))?;

        if diff.deltas().next().is_some() {
            if skipped < offset {
                skipped += 1;
                continue;
            }

            results.push(commit_oid);

            if results.len() >= limit {
                return Ok(results);
            }
        }
    }

    Ok(results)
}

/// `reference` can be either a full ref name or a OID string (ascii-hex-numeric, 40 digits)
/// Returns a page of commits starting at `offset`, returning at most `limit` entries
#[instrument(err, skip(repo))]
pub(crate) async fn paged_commits(repo: &Git2Repository, reference: &str, offset: usize, limit: usize, reverse: bool) -> Result<Vec<Oid>> {
    let mut results = Vec::<Oid>::with_capacity(limit);

    let mut rev_walk = repo.revwalk()?;

    let sort = if reverse { Sort::TIME | Sort::REVERSE } else { Sort::TIME };
    rev_walk.set_sorting(sort)?;

    match Oid::from_str(reference) {
        Ok(oid) => rev_walk.push(oid)?,
        Err(_) => rev_walk.push_ref(reference)?,
    }

    for result in rev_walk.skip(offset).take(limit) {
        results.push(result?);
    }

    Ok(results)
}

/// `reference` can be either a full ref name or a OID string (ascii-hex-numeric, 40 digits)
/// Returns at most `limit` commits or all commits if `limit == 0`
#[instrument(err, skip(repo))]
pub(crate) async fn all_commits(repo: &Git2Repository, reference: &str, limit: usize) -> Result<Vec<Oid>> {
    let mut results = Vec::<Oid>::with_capacity(limit);

    let mut rev_walk = repo.revwalk()?;
    rev_walk.set_sorting(Sort::TIME)?;

    match Oid::from_str(reference) {
        Ok(oid) => rev_walk.push(oid)?,
        Err(_) => rev_walk.push_ref(reference)?,
    }

    for result in rev_walk {
        let commit_oid = result?;

        results.push(commit_oid);

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
        let name = reference.name()?;

        results.push(name.replacen("refs/heads/", "", 1));
    }

    Ok(results)
}

#[instrument(err, skip(repo))]
pub(crate) async fn all_tags(repo: &Git2Repository, prefix: Option<&str>) -> Result<Vec<String>> {
    let tags = repo.tag_names(prefix)?;

    Ok(tags.iter().filter_map(|r| r.ok().flatten().map(ToOwned::to_owned)).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::{Signature, Time};
    use pretty_assertions::assert_eq;
    use tempfile::{TempDir, tempdir};

    fn repo_with_history() -> Result<(TempDir, Git2Repository, Vec<Oid>)> {
        let dir = tempdir()?;
        let repo = Git2Repository::init_bare(dir.path())?;
        let mut commits = Vec::new();

        {
            let mut builder = repo.treebuilder(None)?;

            for index in 0..6_i64 {
                let path = if index % 2 == 0 { "a.txt" } else { "b.txt" };
                builder.insert(path, repo.blob(format!("{index}\n").as_bytes())?, 0o100_644)?;

                let tree = repo.find_tree(builder.write()?)?;
                let signature = Signature::new("tester", "tester@example.com", &Time::new(1_700_000_000 + index * 60, 0))?;
                let parents = commits.last().map(|oid| repo.find_commit(*oid)).transpose()?;

                let oid = repo.commit(
                    Some("refs/heads/main"),
                    &signature,
                    &signature,
                    &format!("commit {index}"),
                    &tree,
                    parents.iter().collect::<Vec<_>>().as_slice(),
                )?;

                commits.push(oid);
            }
        }

        Ok((dir, repo, commits))
    }

    #[tokio::test]
    async fn paged_commits_pages_newest_first() -> Result<()> {
        let (_dir, repo, commits) = repo_with_history()?;

        let first = paged_commits(&repo, "refs/heads/main", 0, 2, false).await?;
        let second = paged_commits(&repo, "refs/heads/main", 2, 2, false).await?;
        let past_end = paged_commits(&repo, "refs/heads/main", 6, 2, false).await?;

        assert_eq!(first, vec![commits[5], commits[4]]);
        assert_eq!(second, vec![commits[3], commits[2]]);
        assert_eq!(past_end, Vec::<Oid>::new());
        Ok(())
    }

    #[tokio::test]
    async fn paged_commits_reverse_starts_with_oldest() -> Result<()> {
        let (_dir, repo, commits) = repo_with_history()?;

        let page = paged_commits(&repo, "refs/heads/main", 1, 3, true).await?;

        assert_eq!(page, vec![commits[1], commits[2], commits[3]]);
        Ok(())
    }

    #[tokio::test]
    async fn paged_commits_accepts_oid_as_reference() -> Result<()> {
        let (_dir, repo, commits) = repo_with_history()?;

        let page = paged_commits(&repo, &commits[2].to_string(), 0, 10, false).await?;

        assert_eq!(page, vec![commits[2], commits[1], commits[0]]);
        Ok(())
    }

    #[tokio::test]
    async fn paged_commits_for_path_only_returns_commits_touching_path() -> Result<()> {
        let (_dir, repo, commits) = repo_with_history()?;

        let a = paged_commits_for_path(&repo, "refs/heads/main", "a.txt", 0, 10).await?;
        let b = paged_commits_for_path(&repo, "refs/heads/main", "b.txt", 0, 10).await?;

        assert_eq!(a, vec![commits[4], commits[2], commits[0]]);
        assert_eq!(b, vec![commits[5], commits[3], commits[1]]);
        Ok(())
    }

    #[tokio::test]
    async fn paged_commits_for_path_respects_offset_and_limit() -> Result<()> {
        let (_dir, repo, commits) = repo_with_history()?;

        let page = paged_commits_for_path(&repo, "refs/heads/main", "a.txt", 1, 1).await?;
        let past_end = paged_commits_for_path(&repo, "refs/heads/main", "a.txt", 3, 1).await?;

        assert_eq!(page, vec![commits[2]]);
        assert_eq!(past_end, Vec::<Oid>::new());
        Ok(())
    }

    #[tokio::test]
    async fn paged_commits_for_unknown_reference_fails() -> Result<()> {
        let (_dir, repo, _) = repo_with_history()?;

        assert!(paged_commits(&repo, "refs/heads/missing", 0, 1, false).await.is_err());
        assert!(paged_commits_for_path(&repo, "refs/heads/missing", "a.txt", 0, 1).await.is_err());
        Ok(())
    }
}
