use crate::utils::oid;

use anyhow::{Error, Result, anyhow, bail};
use tracing::instrument;

#[instrument(err)]
pub(crate) async fn parse_line(raw_line: Vec<u8>) -> Result<RefUpdate> {
    let line = String::from_utf8(raw_line)?;
    let mut ref_update = RefUpdate::default();
    let mut split = line.split(|c: char| c.is_whitespace() || c == '\x00').filter(|s| !s.is_empty());

    let old_ref = split
        .next()
        .ok_or_else::<Error, _>(|| anyhow!("Failed to parse ref update payload. Expected old ref, got: {}", line.clone()))?;
    let new_ref = split
        .next()
        .ok_or_else::<Error, _>(|| anyhow!("Failed to parse ref update payload. Expected new ref, got: {}", line.clone()))?;

    ref_update.old = oid::normalize_str(Some(old_ref)).map(ToOwned::to_owned);
    ref_update.new = oid::normalize_str(Some(new_ref)).map(ToOwned::to_owned);

    let target_ref = split
        .next()
        .ok_or_else::<Error, _>(|| anyhow!("Failed to parse ref update payload. Expected target ref, got: {}", line.clone()))?;

    if !target_ref.starts_with("refs/") {
        bail!("Received target ref which does not start with \"refs/\", is this a partial ref instead of a FQN? Got: {target_ref}",);
    }

    ref_update.target_ref = target_ref.to_owned();

    for option in split.by_ref() {
        match option {
            "report-status" => ref_update.report_status = true,
            "report-status-v2" => ref_update.report_status_v2 = true,
            "side-band-64k" => ref_update.side_band_64k = true,
            _ => match ref_update.push_options {
                Some(ref mut options) => options.push(option.to_owned()),
                None => {
                    let vec = vec![option.to_owned()];
                    ref_update.push_options = Some(vec);
                }
            },
        }
    }

    Ok(ref_update)
}

// capabilities are only sent by the client in the first line
pub(crate) fn propagate_capabilities(updates: &mut [RefUpdate]) {
    if updates.len() <= 1 {
        return;
    }

    let report_status = updates[0].report_status;
    let report_status_v2 = updates[0].report_status_v2;
    let side_band_64k = updates[0].side_band_64k;

    for update in updates.iter_mut().skip(1) {
        update.report_status = report_status;
        update.report_status_v2 = report_status_v2;
        update.side_band_64k = side_band_64k;
    }
}

pub(crate) fn is_only_deletions(updates: &[RefUpdate]) -> Result<bool> {
    for update in updates {
        match RefUpdateType::determinate(&update.old, &update.new)? {
            RefUpdateType::Delete => {}
            _ => return Ok(false),
        }
    }

    Ok(true)
}

#[derive(Debug, Default)]
pub(crate) struct RefUpdate {
    pub(crate) old: Option<String>,
    pub(crate) new: Option<String>,
    pub(crate) target_ref: String,
    pub(crate) report_status: bool,
    pub(crate) report_status_v2: bool,
    pub(crate) side_band_64k: bool,
    pub(crate) push_options: Option<Vec<String>>,
}

pub(crate) enum RefUpdateType {
    Create,
    Delete,
    Update,
}

impl RefUpdateType {
    #[allow(clippy::ref_option)]
    pub(crate) fn determinate(old: &Option<String>, new: &Option<String>) -> Result<RefUpdateType> {
        match (old, new) {
            (None, None) => {
                bail!("Unable to determinate ref update type, both old and new OID are None")
            }
            (None, Some(_)) => Ok(RefUpdateType::Create),
            (Some(_), None) => Ok(RefUpdateType::Delete),
            (Some(_), Some(_)) => Ok(RefUpdateType::Update),
        }
    }
}
