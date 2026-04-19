use anyhow::{Context, Result};
use gix::hash::Kind;
use once_cell::sync::Lazy;
use std::process::Command;
use tracing::{error, info, info_span};

pub(crate) mod basic_auth;
pub(crate) mod capabilities;
pub(crate) mod fetch;
pub(crate) mod history;
pub(crate) mod hooks;
pub(crate) mod io;
pub(crate) mod ls_refs;
pub(crate) mod receive_pack;
pub(crate) mod ref_update;
pub(crate) mod utils;
pub(crate) mod write;

pub(crate) const GIT_HASH_KIND: Kind = Kind::Sha1;

pub(crate) static GIT_CLI_AVAILABLE: Lazy<bool> = Lazy::new(|| {
    let span = info_span!("git_cli");
    let _guard = span.enter();

    match Command::new("git").arg("--version").output().map(|output| String::from_utf8(output.stdout)) {
        Ok(Ok(stdout)) => {
            info!("found {stdout}");
            true
        }
        Ok(Err(err)) => {
            error!(?err, "failed to parse git output");
            false
        }
        Err(err) => {
            error!(?err, "failed to execute git");
            false
        }
    }
});
