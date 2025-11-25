use crate::git::GIT_HASH_KIND;
use crate::repository::Repository;

use std::io::BufReader;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;

use anyhow::{Result, anyhow};
use gitarena_common::database::Database;
use gix::odb::pack::Bundle;
use gix::odb::pack::bundle::write::Options as GitPackWriteOptions;
use gix::odb::pack::data::input::Mode as PackIterationMode;
use gix::odb::pack::index::Version as PackVersion;
use gix::progress;
use sqlx::Transaction;
use tempfile::{Builder, TempDir};
use tracing::instrument;

/// Returns path to index file, pack file and temporary dir.
/// Ensure that the third tuple argument, the temporary dir, is alive for the whole duration of your usage.
/// It being dropped results in the index and pack file to be deleted and thus the paths becoming invalid
#[instrument(err, skip(data, tx))]
pub(crate) async fn read(data: &[u8], repo: &Repository, tx: &mut Transaction<'_, Database>) -> Result<(Option<PathBuf>, Option<PathBuf>, TempDir)> {
    let temp_dir = Builder::new().prefix("gitarena_").tempdir()?;

    match write_to_fs(data, &temp_dir, repo, tx).await {
        Ok((index_path, pack_path)) => Ok((Some(index_path), Some(pack_path), temp_dir)),
        Err(err) => match err.to_string().as_str() {
            // Gitoxide does not export the error enum so this is a whacky workaround
            "Did not encounter a single base" => Ok((None, None, temp_dir)),
            _ => Err(err),
        },
    }
}

#[instrument(err, skip(data, tx))]
pub(crate) async fn write_to_fs(data: &[u8], temp_dir: &TempDir, repo: &Repository, tx: &mut Transaction<'_, Database>) -> Result<(PathBuf, PathBuf)> {
    let options = GitPackWriteOptions {
        thread_limit: Some(num_cpus::get()),
        iteration_mode: PackIterationMode::Verify,
        index_version: PackVersion::V2,
        object_hash: GIT_HASH_KIND,
    };

    let repo = repo.gitoxide(tx).await?;
    let objects = repo.objects.clone();

    let mut buf_reader = BufReader::new(data);

    let bundle = Bundle::write_to_directory(
        &mut buf_reader,
        Some(temp_dir.path()),
        &mut progress::Discard,
        &AtomicBool::new(false), // The Actix runtime (+ tokio) handles timeouts for us
        Some(Box::new(objects)),
        options,
    )?;

    let index_path = bundle.index_path.ok_or_else(|| anyhow!("Failed to unpack index file"))?;
    let data_path = bundle.data_path.ok_or_else(|| anyhow!("Failed to unpack data file"))?;

    Ok((index_path, data_path))
}
