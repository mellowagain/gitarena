use crate::error::GAErrors::PackUnpackError;
use crate::repository::Repository;

use std::io::BufReader;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;

use anyhow::Result;
use git_features::progress;
use git_pack::bundle::write::Options as GitPackWriteOptions;
use git_pack::data::input::{Mode as PackIterationMode};
use git_pack::index::Version as PackVersion;
use git_pack::{Bundle, cache, FindExt};
use tempfile::{Builder, TempDir};

/// Returns path to index file, pack file and temporary dir.
/// Ensure that the third tuple argument, the temporary dir, is alive for the whole duration of your usage.
/// It being dropped results in the index and pack file to be deleted and thus the paths becoming invalid
pub(crate) async fn read(data: &[u8], repo: &Repository, repo_owner: &str) -> Result<(Option<PathBuf>, Option<PathBuf>, TempDir)> {
    let temp_dir = Builder::new().prefix("gitarena_").tempdir()?;

    match write_to_fs(data, &temp_dir, repo, repo_owner).await {
        Ok((index_path, pack_path)) => Ok((Some(index_path), Some(pack_path), temp_dir)),
        Err(err) => match err.to_string().as_str() { // Gitoxide does not export the error enum so this is a whacky workaround
            "Did not encounter a single base" => Ok((None, None, temp_dir)),
            _ => Err(err)
        }
    }
}

pub(crate) async fn write_to_fs(data: &[u8], temp_dir: &TempDir, repo: &Repository, repo_owner: &str) -> Result<(PathBuf, PathBuf)> {
    let options = GitPackWriteOptions {
        thread_limit: Some(num_cpus::get()),
        iteration_mode: PackIterationMode::Verify,
        index_kind: PackVersion::V2
    };

    let repo = repo.gitoxide(repo_owner).await?;
    let buf_reader = BufReader::new(data);

    let bundle = Bundle::write_to_directory(
        buf_reader,
        Some(&temp_dir),
        progress::Discard,
        &AtomicBool::new(false), // The Actix runtime (+ tokio) handles timeouts for us
        Some(Box::new(move |oid, buffer| {
            repo.odb.find_existing(oid, buffer, &mut cache::Never).ok()
        })),
        options
    )?;

    let index_path = bundle.index_path.ok_or(PackUnpackError("index file"))?;
    let data_path = bundle.data_path.ok_or(PackUnpackError("data file"))?;

    Ok((index_path, data_path))
}
