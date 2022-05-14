use crate::utils::oid;

use anyhow::{anyhow, bail, Error, Result};
use tracing::instrument;

#[instrument(err)]
pub(crate) async fn parse_line(raw_line: Vec<u8>) -> Result<RefUpdate> {
    let line = String::from_utf8(raw_line)?;
    let mut ref_update = RefUpdate::default();
    let mut split = line.split(|c: char| {
        c.is_whitespace() || c == '\x00'
    }).filter(|s| !s.is_empty());

    let old_ref = split.next().ok_or_else::<Error, _>(|| anyhow!("Failed to parse ref update payload. Expected old ref, got: {}", line.clone()))?;
    let new_ref = split.next().ok_or_else::<Error, _>(|| anyhow!("Failed to parse ref update payload. Expected new ref, got: {}", line.clone()))?;

    ref_update.old = oid::normalize_str(Some(old_ref)).map(|o| o.to_owned());
    ref_update.new = oid::normalize_str(Some(new_ref)).map(|o| o.to_owned());

    let target_ref = split.next().ok_or_else::<Error, _>(|| anyhow!("Failed to parse ref update payload. Expected target ref, got: {}", line.clone()))?;

    if !target_ref.starts_with("refs/") {
        bail!("Received target ref which does not start with \"refs/\", is this a partial ref instead of a FQN? Got: {}", target_ref);
    }

    ref_update.target_ref = target_ref.to_owned();

    for option in split.by_ref() {
        match option {
            "report-status" => ref_update.report_status = true,
            "report-status-v2" => ref_update.report_status_v2 = true,
            "side-band-64k" => ref_update.side_band_64k = true,
            _ => {
                match ref_update.push_options {
                    Some(ref mut options) => options.push(option.to_owned()),
                    None => {
                        let vec = vec![option.to_owned()];
                        ref_update.push_options = Some(vec);
                    }
                }
            }
        }
    }

    Ok(ref_update)
}

pub(crate) async fn is_only_deletions(updates: &[RefUpdate]) -> Result<bool> {
    for update in updates {
        match RefUpdateType::determinate(&update.old, &update.new).await? {
            RefUpdateType::Delete => continue,
            _ => return Ok(false)
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
    pub(crate) push_options: Option<Vec<String>>
}

pub(crate) enum RefUpdateType {
    Create,
    Delete,
    Update
}

impl RefUpdateType {
    pub(crate) async fn determinate(old: &Option<String>, new: &Option<String>) -> Result<RefUpdateType> {
        match (old, new) {
            (None, None) => bail!("Unable to determinate ref update type, both old and new OID are None"),
            (None, Some(_)) => Ok(RefUpdateType::Create),
            (Some(_), None) => Ok(RefUpdateType::Delete),
            (Some(_), Some(_)) => Ok(RefUpdateType::Update)
        }
    }
}
