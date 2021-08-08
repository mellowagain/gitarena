use crate::error::GAErrors::PackUnpackError;

use std::io::BufReader;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;

use anyhow::Result;
use git_features::progress;
use git_pack::bundle::write::Options as GitPackWriteOptions;
use git_pack::Bundle;
use git_pack::data::input::Mode as PackIterationMode;
use git_pack::index::Version as PackVersion;
use tempfile::{Builder, TempDir};

/// Returns path to index file, pack file and temporary dir.
/// Ensure that the third tuple argument, the temporary dir, is alive for the whole duration of your usage.
/// It being dropped results in the index and pack file to be deleted and thus the paths becoming invalid
pub(crate) async fn read(data: &[u8]) -> Result<(PathBuf, PathBuf, TempDir)> {
    let temp_dir = Builder::new().prefix("gitarena_").tempdir()?;

    let (index_path, pack_path) = write_to_fs(data, &temp_dir).await?;

    Ok((index_path, pack_path, temp_dir))
}

pub(crate) async fn write_to_fs(data: &[u8], temp_dir: &TempDir) -> Result<(PathBuf, PathBuf)> {
    let options = GitPackWriteOptions {
        thread_limit: Some(num_cpus::get()),
        iteration_mode: PackIterationMode::Verify,
        index_kind: PackVersion::V2
    };

    let buf_reader = BufReader::new(data);

    // TODO: This fails on delta objects, probably need to define the thin pack base object lookup fn?
    // Error: Ref delta objects are not supported as there is no way to look them up. Resolve them beforehand.
    let bundle = Bundle::write_to_directory(
        buf_reader,
        Some(&temp_dir),
        progress::Discard,
        &AtomicBool::new(false), // The Actix runtime (+ tokio) handles timeouts for us
        None,
        options
    )?;

    let index_path = bundle.index_path.ok_or(PackUnpackError("index file"))?;
    let data_path = bundle.data_path.ok_or(PackUnpackError("data file"))?;

    Ok((index_path, data_path))
}
