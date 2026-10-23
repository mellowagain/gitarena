//! Lamport clocks shared with `git-bug`, stored in `.git/git-bug/clocks/`

use anyhow::{Context, Result};
use gix::Repository;
use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Read, Seek, Write};
use std::path::PathBuf;

/// Lamport clock incremented once per created bug
pub const BUGS_CREATE: &str = "bugs-create";
/// Lamport clock incremented once per bug commit
pub const BUGS_EDIT: &str = "bugs-edit";

/// Increment the clock `name` and return its new value
///
/// # Arguments
///
/// - `git_repo`: Handle to a [`Repository`]
/// - `name`: Clock name, e.g. [`BUGS_EDIT`]
pub fn increment(git_repo: &Repository, name: &str) -> Result<u64> {
    update(git_repo, name, |current| current + 1)
}

/// Advance the clock `name` to `value` if it is behind
///
/// # Arguments
///
/// - `git_repo`: Handle to a [`Repository`]
/// - `name`: Clock name, e.g. [`BUGS_EDIT`]
/// - `value`: Observed clock value
pub fn witness(git_repo: &Repository, name: &str, value: u64) -> Result<()> {
    // load_bug witnesses on every read, so don't take the lock unless the clock actually moves
    if read(git_repo, name)? >= value {
        return Ok(());
    }

    update(git_repo, name, |current| current.max(value))?;
    Ok(())
}

pub(crate) fn read(git_repo: &Repository, name: &str) -> Result<u64> {
    match fs::read_to_string(clock_path(git_repo, name)) {
        Ok(content) => parse(&content, name),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(0),
        Err(err) => Err(err).with_context(|| format!("failed to read clock {name}")),
    }
}

// same location and format as git-bug so both share the counter when used on the same repository
fn clock_path(git_repo: &Repository, name: &str) -> PathBuf {
    git_repo.git_dir().join("git-bug").join("clocks").join(name)
}

fn parse(content: &str, name: &str) -> Result<u64> {
    if content.is_empty() {
        return Ok(0);
    }

    content.trim().parse().with_context(|| format!("clock {name} is corrupt"))
}

fn update(git_repo: &Repository, name: &str, next: impl FnOnce(u64) -> u64) -> Result<u64> {
    let path = clock_path(git_repo, name);

    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).context("failed to create clocks directory")?;
    }

    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&path)
        .with_context(|| format!("failed to open clock {name}"))?;

    file.lock().with_context(|| format!("failed to lock clock {name}"))?;

    let mut content = String::new();
    file.read_to_string(&mut content)?;

    let value = next(parse(&content, name)?);

    file.set_len(0)?;
    file.rewind()?;
    write!(file, "{value}")?;

    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use tempfile::tempdir;

    #[test]
    fn increment_and_witness() -> Result<()> {
        let dir = tempdir()?;
        let repo = gix::init(dir.path())?;

        assert_eq!(read(&repo, BUGS_EDIT)?, 0);
        assert_eq!(increment(&repo, BUGS_EDIT)?, 1);
        assert_eq!(increment(&repo, BUGS_EDIT)?, 2);

        witness(&repo, BUGS_EDIT, 10)?;
        assert_eq!(read(&repo, BUGS_EDIT)?, 10);

        witness(&repo, BUGS_EDIT, 5)?;
        assert_eq!(increment(&repo, BUGS_EDIT)?, 11);

        assert_eq!(read(&repo, BUGS_CREATE)?, 0);

        let stored = fs::read_to_string(dir.path().join(".git/git-bug/clocks/bugs-edit"))?;
        assert_eq!(stored, "11");

        Ok(())
    }
}
