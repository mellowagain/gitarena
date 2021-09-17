use anyhow::Result;
use async_recursion::async_recursion;
use git2::{DiffOptions, Oid, Repository as Git2Repository};

pub(crate) async fn last_commit_for_blob(repo: &Git2Repository, reference_name: &str, file_name: &str) -> Result<Option<Oid>> {
    let commits = commits_for_blob(repo, reference_name, file_name, Some(1)).await?;

    Ok(commits.iter().next().map(|c| *c))
}

#[async_recursion(?Send)]
pub(crate) async fn last_commit_for_ref(repo: &Git2Repository, reference_name: &str) -> Result<Option<Oid>> {
    let reference = repo.find_reference(reference_name)?;

    if let Some(target) = reference.symbolic_target() {
        return last_commit_for_ref(repo, target).await;
    }

    Ok(reference.target())
}

pub(crate) async fn commits_for_blob(repo: &Git2Repository, reference: &str, file_name: &str, max_results: Option<usize>) -> Result<Vec<Oid>> {
    let mut results = Vec::<Oid>::new();

    if let Some(max) = max_results {
        results.reserve(max);
    }

    let mut rev_walk = repo.revwalk()?;
    rev_walk.set_sorting(git2::Sort::TIME)?;
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

pub(crate) async fn all_commits(repo: &Git2Repository, reference: &str) -> Result<Vec<Oid>> {
    let mut results = Vec::<Oid>::new();

    let mut rev_walk = repo.revwalk()?;
    rev_walk.set_sorting(git2::Sort::TIME)?;
    rev_walk.push_ref(reference)?;

    for result in rev_walk {
        let commit_oid = result?;

        results.push(commit_oid);
    }

    Ok(results)
}

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

pub(crate) async fn all_tags(repo: &Git2Repository, prefix: Option<&str>) -> Result<Vec<String>> {
    let tags = repo.tag_names(prefix)?;

    Ok(tags.iter()
        .filter_map(|o| o.map(|o| o.to_owned()))
        .collect())
}
