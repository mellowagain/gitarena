use crate::git::io::writer::GitWriter;

use crate::git::io::band::Band;
use anyhow::{Result, anyhow};
use bstr::ByteSlice;
use gix::object::Kind;
use gix::object::tree::diff::ChangeDetached;
use gix::{ObjectId, Repository};
use kingfisher_rules::{Confidence, Rules, RulesDatabase, get_builtin_rules};
use kingfisher_scanner::{Finding, Scanner};
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;
use tokio::runtime::Handle;
use tokio::sync::OnceCell;
use tracing::{Span, debug, info, instrument};

pub(crate) static SECRET_SCANNER: OnceCell<Scanner> = OnceCell::const_new();

const MAX_REPORTED_FINDINGS: usize = 50;

#[instrument(err)]
pub(crate) async fn init() -> Result<Scanner> {
    debug!("loading secret scanning rules");

    let span = Span::current();

    let scanner = tokio::task::spawn_blocking(|| -> Result<Scanner> {
        let _guard = span.entered();
        let mut rules = get_builtin_rules(None)?;

        let ga_rules = Rules::from_paths_and_contents(
            [(
                Path::new("gitarena-kingfisher-rules.yml"),
                &include_bytes!("../../../gitarena-kingfisher-rules.yml")[..],
            )],
            Confidence::High,
        )?;

        rules.update(ga_rules);

        info!(amount = rules.rules.len(), "parsed secret scanning rules");

        let database = RulesDatabase::from_rule_collection(rules)?;
        Ok(Scanner::new(Arc::new(database)))
    })
    .await??;

    debug!("secret scanning rules compiled");
    Ok(scanner)
}

#[instrument(err, skip(repo, writer))]
pub(crate) fn scan_push(repo: &Repository, tips: &[ObjectId], side_band: bool, writer: &mut GitWriter) -> Result<bool> {
    let scanner = SECRET_SCANNER
        .get()
        .ok_or_else(|| anyhow!("secret scanning should only be used after the scanner has been initialized"))?;

    let mut commit_tips = Vec::new();

    for &tip in tips {
        let object = repo.find_object(tip)?.peel_tags_to_end()?;

        if object.kind == Kind::Commit {
            commit_tips.push(object.id);
        } else {
            debug!(%tip, kind = %object.kind, "skipping secret scanning for tag that does not point to a commit");
        }
    }

    let mut hidden = Vec::new();

    for reference in repo.references()?.all()? {
        if let Ok(commit) = reference?.peel_to_commit() {
            hidden.push(commit.id);
        }
    }

    let mut seen_blobs = HashSet::new();
    let mut scan_results: HashMap<ObjectId, Vec<Finding>> = HashMap::new();
    let mut lines = Vec::new();
    let mut found = 0;
    let mut truncated = false;

    'walk: for info in repo.rev_walk(commit_tips).with_hidden(hidden).all()? {
        let commit = info?.object()?;

        if commit.message_raw()?.contains_str("[skip secret scanning]") {
            continue;
        }

        let parent_tree = match commit.parent_ids().next() {
            Some(parent) => Some(repo.find_commit(parent)?.tree()?),
            None => None,
        };

        for change in repo.diff_tree_to_tree(parent_tree.as_ref(), &commit.tree()?, None)? {
            let (location, entry_mode, id) = match change {
                ChangeDetached::Addition { location, entry_mode, id, .. }
                | ChangeDetached::Modification { location, entry_mode, id, .. }
                | ChangeDetached::Rewrite { location, entry_mode, id, .. } => (location, entry_mode, id),
                ChangeDetached::Deletion { .. } => continue,
            };

            if !entry_mode.is_blob() || !seen_blobs.insert((id, location.clone())) {
                continue;
            }

            let findings = match scan_results.entry(id) {
                Entry::Occupied(entry) => entry.into_mut(),
                Entry::Vacant(entry) => entry.insert(scanner.scan_bytes(&repo.find_blob(id)?.data)?),
            };

            for finding in findings.iter() {
                if found == MAX_REPORTED_FINDINGS {
                    truncated = true;
                    break 'walk;
                }

                lines.push(finding.rule_name.clone());
                lines.push(format!("  secret: {}", mask_secret(&finding.secret)));
                lines.push(format!("  location: {location}:{}:{}", finding.location.line, finding.location.column));
                lines.push(format!("  confidence: {}", finding.confidence));

                if finding.is_base64_encoded {
                    lines.push("  note: secret is base64 encoded".to_string());
                }

                lines.push(" ".to_string());
                found += 1;
            }
        }
    }

    if found == 0 {
        return Ok(false);
    }

    if !side_band {
        return Ok(true);
    }

    let mut header = vec![
        " ".to_string(),
        "SECRETS DETECTED IN PUSH".to_string(),
        "If this is a false positive, change your commit message to include `[skip secret scanning]`.".to_string(),
    ];

    if truncated {
        header.push(format!("Only the first {MAX_REPORTED_FINDINGS} findings are shown."));
    }

    header.push(" ".to_string());

    Handle::current().block_on(async {
        for line in header.into_iter().chain(lines) {
            writer.write_text_sideband(Band::Progress, line).await?;
        }

        Ok::<(), anyhow::Error>(())
    })?;

    Ok(true)
}

fn mask_secret(secret: &str) -> String {
    let visible = (secret.chars().count() / 4).min(6);
    let prefix: String = secret.chars().take(visible).collect();

    format!("{prefix}***")
}
