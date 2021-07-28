use crate::git::writer::GitWriter;

use actix_web::web::Bytes;
use anyhow::Result;
use git2::Repository as Git2Repository;

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

        for output_line in build_ref_prefix(prefix, repo, &options).await? {
            if output_line.is_empty() {
                writer.flush()?;
                continue;
            }

            writer.write_text(output_line)?;
        }
    }

    Ok(writer.flush()?.to_actix()?)
}

pub(crate) async fn build_ref_prefix(prefix: &String, repo: &Git2Repository, options: &LsRefs) -> Result<Vec<String>> {
    let mut output = Vec::<String>::new();

    for result in repo.references()? {
        match result {
            Ok(reference) => {
                let name = reference.name().unwrap_or_default();

                if !name.starts_with(prefix) {
                    continue;
                }

                let mut line;

                if let Some(oid) = reference.target() {
                    line = format!("{} {}", oid, name);
                } else {
                    if !options.unborn {
                        continue
                    }

                    line = format!("unborn {}", name);
                }

                if options.symrefs {
                    if let Some(sym_target) = reference.symbolic_target() {
                        line.push_str(&format!(" symref-target:{}", sym_target));
                    }
                }

                if options.peel {
                    if let Some(peel) = reference.target_peel() {
                        line.push_str(&format!(" peeled:{}", peel));
                    }
                }

                output.push(line);
            },
            Err(_) => output.push("".to_owned())
        }
    }

    Ok(output)
}

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
