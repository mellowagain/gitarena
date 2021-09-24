use crate::git::io::writer::GitWriter;

use core::result::Result as CoreResult;
use std::sync::Once;

use actix_web::web::Bytes;
use anyhow::Result;
use git2::{Error as Git2Error, ErrorCode, Reference, Repository as Git2Repository};
use log::{error, warn};
use tracing::instrument;

// TODO: Combine ls_refs and ls_refs_all to be shared (currently some code is duplicated)

// Used by git-upload-pack ref discovery
#[instrument(err, skip(repo))]
pub(crate) async fn ls_refs(input: Vec<Vec<u8>>, repo: &Git2Repository) -> Result<Bytes> {
    let mut options = LsRefs::default();
    let mut writer = GitWriter::new();

    for raw_line in input.iter() {
        let line = String::from_utf8(raw_line.to_vec())?;

        if line == "peel" {
            options.peel = true;
        }

        if line == "symrefs" {
            options.symrefs = true;
        }

        if line.starts_with("ref-prefix ") {
            options.prefixes.push(line[11..].to_owned());
        }

        if line == "unborn" {
            options.unborn = true;
        }
    }

    for prefix in &options.prefixes {
        if prefix.is_empty() {
            continue;
        }

        // HEAD is a special case as `repo.references_glob` does not find it but `repo.find_reference` does
        if prefix == "HEAD" {
            if let Some(output_line) = build_ref_line(repo.find_reference("HEAD"), repo, &options).await {
                writer.write_text(output_line).await?;
            }
        }

        for output_line in build_ref_list(prefix.as_str(), repo, &options).await? {
            if output_line.is_empty() {
                writer.flush().await?;
                continue;
            }

            writer.write_text(output_line).await?;
        }
    }

    writer.flush().await?;

    Ok(writer.serialize().await?)
}

pub(crate) async fn build_ref_list(prefix: &str, repo: &Git2Repository, options: &LsRefs) -> Result<Vec<String>> {
    let mut output = Vec::<String>::new();

    for result in repo.references_glob(format!("{}*", prefix).as_str())? {
        if let Some(ref_line) = build_ref_line(result, repo, options).await {
            output.push(ref_line);
        }
    }

    Ok(output)
}

#[instrument(skip(ref_result, repo))]
pub(crate) async fn build_ref_line(ref_result: CoreResult<Reference<'_>, Git2Error>, repo: &Git2Repository, options: &LsRefs) -> Option<String> {
    return match ref_result {
        Ok(reference) => {
            let name = reference.name().unwrap_or_default();

            let mut line;

            if let Some(oid) = reference.target() {
                line = format!("{} {}", oid, name);
            } else if let Some(sym_target) = reference.symbolic_target() {
                match repo.find_reference(sym_target).ok() {
                    Some(sym_target_ref) => {
                        if let Some(sym_target_oid) = sym_target_ref.target() {
                            line = format!("{} {} symref-target:{}", sym_target_oid, name, sym_target_ref.name().unwrap_or_default());
                        } else if options.unborn {
                            line = format!("unborn {} symref-target:{}", name, sym_target_ref.name().unwrap_or_default());
                        } else {
                            return None;
                        }
                    }
                    None => return None // Reference points to a symbolic target that doesn't exist?
                }
            } else if options.unborn {
                line = format!("unborn {}", name);
            } else {
                return None;
            }

            if options.peel {
                if let Some(peel) = reference.target_peel() {
                    line.push_str(&format!(" peeled:{}", peel));
                }
            }

            Some(line)
        },
        Err(e) => {
            if e.code() != ErrorCode::NotFound {
                error!("Failed to find reference asked for by Git client: {}", e);
            }

            None
        }
    }
}

// Used by git-receive-pack ref discovery
#[instrument(err, skip(repo))]
pub(crate) async fn ls_refs_all(repo: &Git2Repository) -> Result<Bytes> {
    let mut writer = GitWriter::new();

    writer.write_text("# service=git-receive-pack").await?;
    writer.flush().await?;

    let once = Once::new();

    for result in repo.references()? {
        match result {
            Ok(reference) => {
                if let Some(name) = reference.name() {
                    if let Some(oid) = reference.target() {
                        let mut line = format!("{} {}", oid, name);

                        // Git ignores capabilities written after the first line
                        once.call_once(|| {
                            line.push_str(receive_pack_capabilities());
                        });

                        writer.write_text(line).await?;
                    }
                }
            }
            Err(e) => {
                warn!("Failed to grab repository references for {}: {}", repo.path().display(), e);
            }
        }
    }

    // If we didn't tell the client our capabilities in the previous ref list, send a null ref with them
    if !once.is_completed() {
        writer.write_text(format!("0000000000000000000000000000000000000000 capabilities^{{}}{}", receive_pack_capabilities())).await?;
    }

    writer.flush().await?;

    Ok(writer.serialize().await?)
}

const fn receive_pack_capabilities() -> &'static str {
    concat!("\x00report-status report-status-v2 delete-refs side-band-64k quiet object-format=sha1 agent=git/gitarena-", env!("CARGO_PKG_VERSION"))
}

#[derive(Debug)]
pub(crate) struct LsRefs {
    pub(crate) peel: bool,
    pub(crate) symrefs: bool,
    pub(crate) prefixes: Vec<String>,
    pub(crate) unborn: bool
}

impl Default for LsRefs {
    fn default() -> LsRefs {
        LsRefs {
            peel: false,
            symrefs: false,
            prefixes: Vec::<String>::new(),
            unborn: false
        }
    }
}
