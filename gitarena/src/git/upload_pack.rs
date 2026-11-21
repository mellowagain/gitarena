use crate::git::fetch::fetch;
use crate::git::io::reader::{read_data_lines, read_until_command};
use crate::git::ls_refs::ls_refs;

use std::collections::HashSet;

use anyhow::{Result, bail};
use git2::Repository;
use gix::protocol::transport::packetline::PacketLineRef;
use gix::protocol::transport::packetline::async_io::StreamingPeekableIter;
use tracing::instrument;

#[instrument(err, skip(data, repo, hidden_refs))]
pub(crate) async fn execute_upload_pack_v2(data: &[u8], repo: &Repository, hidden_refs: &HashSet<String>) -> Result<Vec<u8>> {
    let mut readable_iter = StreamingPeekableIter::new(data, &[PacketLineRef::Flush], false);
    readable_iter.fail_on_err_lines(true);

    let git_body = read_data_lines(&mut readable_iter).await?;
    let (command, body) = read_until_command(git_body).await?;

    let output = match command.as_str() {
        "ls-refs" => ls_refs(body, repo, hidden_refs).await?,
        "fetch" => fetch(body, repo, true, hidden_refs).await?,
        _ => bail!("unknown git-upload-pack command: {command}"),
    };

    Ok(output.to_vec())
}

#[instrument(err, skip(data, repo, hidden_refs))]
pub(crate) async fn execute_upload_pack_v1(data: &[u8], repo: &Repository, hidden_refs: &HashSet<String>) -> Result<Vec<u8>> {
    let mut readable_iter = StreamingPeekableIter::new(data, &[], false);
    readable_iter.fail_on_err_lines(true);

    let git_body = read_data_lines(&mut readable_iter).await?;

    Ok(fetch(git_body, repo, false, hidden_refs).await?.to_vec())
}
