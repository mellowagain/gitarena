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
        bail!("Received target ref which does not start with \"refs/\", is this a partial ref instead of a FQN? Got: {target_ref}");
    }

    target_ref.clone_into(&mut ref_update.target_ref);

    for option in split.by_ref() {
        match option {
            "report-status" => ref_update.report_status = true,
            "report-status-v2" => ref_update.report_status_v2 = true,
            "side-band-64k" => ref_update.side_band_64k = true,
            _ => {
                if let Some(ref mut options) = ref_update.push_options {
                    options.push(option.to_owned());
                } else {
                    let vec = vec![option.to_owned()];
                    ref_update.push_options = Some(vec);
                }
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    const NULL: &str = "0000000000000000000000000000000000000000";
    const OLD: &str = "b52f683ce73e8be06428b8c6cf0eb421eae21772";
    const NEW: &str = "4b825dc642cb6eb9a060e54bf8d69288fbee4904";

    fn update(old: Option<&str>, new: Option<&str>) -> RefUpdate {
        RefUpdate {
            old: old.map(ToOwned::to_owned),
            new: new.map(ToOwned::to_owned),
            target_ref: "refs/heads/main".to_owned(),
            ..RefUpdate::default()
        }
    }

    #[tokio::test]
    async fn parse_line_reads_old_new_and_ref() -> Result<()> {
        let update = parse_line(format!("{OLD} {NEW} refs/heads/main").into_bytes()).await?;

        assert_eq!(update.old.as_deref(), Some(OLD));
        assert_eq!(update.new.as_deref(), Some(NEW));
        assert_eq!(update.target_ref, "refs/heads/main");
        assert!(!update.report_status);
        assert!(!update.report_status_v2);
        assert!(!update.side_band_64k);
        assert_eq!(update.push_options, None);
        Ok(())
    }

    #[tokio::test]
    async fn parse_line_normalizes_null_oids() -> Result<()> {
        let create = parse_line(format!("{NULL} {NEW} refs/heads/feature").into_bytes()).await?;
        let delete = parse_line(format!("{OLD} {NULL} refs/heads/feature").into_bytes()).await?;

        assert_eq!(create.old, None);
        assert_eq!(create.new.as_deref(), Some(NEW));
        assert_eq!(delete.old.as_deref(), Some(OLD));
        assert_eq!(delete.new, None);
        Ok(())
    }

    #[tokio::test]
    async fn parse_line_reads_capabilities_after_nul() -> Result<()> {
        let line = format!("{NULL} {NEW} refs/heads/main\0 report-status-v2 side-band-64k quiet object-format=sha1 agent=git/2.56.0-Linux");
        let update = parse_line(line.into_bytes()).await?;

        assert!(!update.report_status);
        assert!(update.report_status_v2);
        assert!(update.side_band_64k);
        assert_eq!(update.push_options, None);
        Ok(())
    }

    #[tokio::test]
    async fn parse_line_rejects_partial_ref_names() {
        assert!(parse_line(format!("{OLD} {NEW} main").into_bytes()).await.is_err());
    }

    #[tokio::test]
    async fn parse_line_rejects_truncated_lines() {
        assert!(parse_line(Vec::new()).await.is_err());
        assert!(parse_line(OLD.as_bytes().to_vec()).await.is_err());
        assert!(parse_line(format!("{OLD} {NEW}").into_bytes()).await.is_err());
    }

    #[tokio::test]
    async fn parse_line_rejects_invalid_utf8() {
        assert!(parse_line(vec![0xff, 0xfe]).await.is_err());
    }

    #[test]
    fn determinate_classifies_updates() {
        assert!(matches!(RefUpdateType::determinate(&None, &Some(NEW.to_owned())), Ok(RefUpdateType::Create)));
        assert!(matches!(RefUpdateType::determinate(&Some(OLD.to_owned()), &None), Ok(RefUpdateType::Delete)));
        assert!(matches!(
            RefUpdateType::determinate(&Some(OLD.to_owned()), &Some(NEW.to_owned())),
            Ok(RefUpdateType::Update)
        ));
        assert!(RefUpdateType::determinate(&None, &None).is_err());
    }

    #[test]
    fn is_only_deletions_requires_every_update_to_be_a_delete() -> Result<()> {
        assert!(is_only_deletions(&[update(Some(OLD), None), update(Some(NEW), None)])?);
        assert!(!is_only_deletions(&[update(Some(OLD), None), update(None, Some(NEW))])?);
        assert!(!is_only_deletions(&[update(Some(OLD), Some(NEW))])?);
        assert!(is_only_deletions(&[])?);
        assert!(is_only_deletions(&[update(None, None)]).is_err());
        Ok(())
    }

    #[test]
    fn propagate_capabilities_copies_first_line_flags() {
        let mut first = update(None, Some(NEW));
        first.report_status = true;
        first.side_band_64k = true;

        let mut second = update(Some(OLD), None);
        second.report_status_v2 = true;

        let mut updates = [first, second, update(Some(OLD), Some(NEW))];
        propagate_capabilities(&mut updates);

        for update in &updates {
            assert!(update.report_status);
            assert!(!update.report_status_v2);
            assert!(update.side_band_64k);
        }
    }
}
