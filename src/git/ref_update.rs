use crate::error::GAErrors::ParseError;
use crate::error::GAErrors;
use crate::extensions::normalize_oid_str;

use anyhow::Result;

pub(crate) async fn parse_line(raw_line: Vec<u8>) -> Result<RefUpdate> {
    let line = String::from_utf8(raw_line)?;
    let mut ref_update = RefUpdate::default();
    let mut split = line.split(|c: char| {
        c.is_whitespace() || c == '\x00'
    }).filter(|s| !s.is_empty());

    let old_ref = split.next().ok_or::<GAErrors>(ParseError("Ref update", line.clone()).into())?;
    let new_ref = split.next().ok_or::<GAErrors>(ParseError("Ref update", line.clone()).into())?;

    ref_update.old = normalize_oid_str(Some(old_ref.to_owned()));
    ref_update.new = normalize_oid_str(Some(new_ref.to_owned()));

    let target_ref = split.next().ok_or::<GAErrors>(ParseError("Ref update", line.clone()).into())?;

    if !target_ref.starts_with("refs/") {
        return Err(ParseError("Ref update", line.clone()).into());
    }

    ref_update.target_ref = target_ref.to_owned();

    while let Some(option) = split.next() {
        match option {
            "report-status" => ref_update.report_status = true,
            "report-status-v2" => ref_update.report_status_v2 = true,
            "side-band-64k" => ref_update.side_band_64k = true,
            _ => {
                match ref_update.push_options {
                    Some(ref mut options) => options.push(option.to_owned()),
                    None => {
                        let mut vec = Vec::<String>::new();
                        vec.push(option.to_owned());

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

#[derive(Debug)]
pub(crate) struct RefUpdate {
    pub(crate) old: Option<String>,
    pub(crate) new: Option<String>,
    pub(crate) target_ref: String,
    pub(crate) report_status: bool,
    pub(crate) report_status_v2: bool,
    pub(crate) side_band_64k: bool,
    pub(crate) push_options: Option<Vec<String>>
}

impl Default for RefUpdate {
    fn default() -> RefUpdate {
        RefUpdate {
            old: None,
            new: None,
            target_ref: String::new(),
            report_status: false,
            report_status_v2: false,
            side_band_64k: false,
            push_options: None
        }
    }
}

pub(crate) enum RefUpdateType {
    Create,
    Delete,
    Update
}

impl RefUpdateType {
    pub(crate) async fn determinate(old: &Option<String>, new: &Option<String>) -> Result<RefUpdateType> {
        match (old, new) {
            (None, None) => Err(ParseError("Ref update type determination", "(None, None)".to_owned()).into()),
            (None, Some(_)) => Ok(RefUpdateType::Create),
            (Some(_), None) => Ok(RefUpdateType::Delete),
            (Some(_), Some(_)) => Ok(RefUpdateType::Update)
        }
    }
}
