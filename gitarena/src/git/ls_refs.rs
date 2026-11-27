use crate::git::GitProtocol;
use crate::git::io::writer::GitWriter;

use core::result::Result as CoreResult;
use std::collections::HashSet;
use std::fmt::Write as _;

use actix_web::web::Bytes;
use anyhow::Result;
use git2::{Error as Git2Error, ErrorCode, ObjectType, Oid, Reference, Repository as Git2Repository};
use tracing::instrument;
use tracing::{error, warn};

// TODO: Combine ls_refs and ls_refs_all to be shared (currently some code is duplicated)

// Used by git-upload-pack ref discovery
#[instrument(err, skip(repo, hidden_refs))]
pub(crate) async fn ls_refs(input: Vec<Vec<u8>>, repo: &Git2Repository, hidden_refs: &HashSet<String>) -> Result<Bytes> {
    let mut options = LsRefs::default();
    let mut writer = GitWriter::new();

    for raw_line in &input {
        let line = String::from_utf8(raw_line.clone())?;

        if line == "peel" {
            options.peel = true;
        }

        if line == "symrefs" {
            options.symrefs = true;
        }

        if let Some(stripped) = line.strip_prefix("ref-prefix ") {
            options.prefixes.push(stripped.to_owned());
        }

        if line == "unborn" {
            options.unborn = true;
        }
    }

    // HEAD is a special case as `repo.references` does not list it
    if options.matches("HEAD")
        && let Some(output_line) = build_ref_line(repo.find_reference("HEAD"), repo, &options).await
    {
        writer.write_text(output_line).await?;
    }

    for result in repo.references()? {
        if let Ok(reference) = &result
            && let name = reference.name().unwrap_or_default()
            && (!options.matches(name) || hidden_refs.contains(name))
        {
            continue;
        }

        if let Some(output_line) = build_ref_line(result, repo, &options).await {
            writer.write_text(output_line).await?;
        }
    }

    writer.flush().await?;
    writer.serialize().await
}

#[instrument(skip(ref_result, repo))]
pub(crate) async fn build_ref_line(ref_result: CoreResult<Reference<'_>, Git2Error>, repo: &Git2Repository, options: &LsRefs) -> Option<String> {
    match ref_result {
        Ok(reference) => {
            let name = reference.name().unwrap_or_default();

            let mut line;

            if let Some(oid) = reference.target() {
                line = format!("{oid} {name}");
            } else if let Some(sym_target) = reference.symbolic_target().ok()? {
                match repo.find_reference(sym_target).ok() {
                    Some(sym_target_ref) => {
                        if let Some(sym_target_oid) = sym_target_ref.target() {
                            line = format!("{sym_target_oid} {name}");
                        } else if options.unborn && options.symrefs {
                            line = format!("unborn {name}");
                        } else {
                            return None;
                        }

                        if options.symrefs {
                            let _ = write!(line, " symref-target:{}", sym_target_ref.name().unwrap_or_default());
                        }
                    }
                    None if options.unborn && options.symrefs => line = format!("unborn {name} symref-target:{sym_target}"),
                    None => return None, // Reference points to a symbolic target that doesn't exist?
                }
            } else if options.unborn {
                line = format!("unborn {name}");
            } else {
                return None;
            }

            if options.peel
                && let Some(peel) = reference.target().and_then(|oid| peeled_tag_target(repo, oid))
            {
                let _ = write!(line, " peeled:{peel}");
            }

            Some(line)
        }
        Err(err) => {
            if err.code() != ErrorCode::NotFound {
                error!(?err, "Failed to find reference asked for by Git client");
            }

            None
        }
    }
}

pub(crate) async fn ls_refs_all(repo: &Git2Repository, service_header: Option<&str>, version: GitProtocol, hidden_refs: &HashSet<String>) -> Result<Bytes> {
    ls_refs_all_inner(repo, service_header, version, hidden_refs, false, |_| receive_pack_capabilities().to_string()).await
}

pub(crate) async fn ls_refs_all_upload_pack(
    repo: &Git2Repository,
    service_header: Option<&str>,
    version: GitProtocol,
    hidden_refs: &HashSet<String>,
) -> Result<Bytes> {
    let head_symref = repo.find_reference("HEAD")?.symbolic_target()?.map(ToString::to_string);

    ls_refs_all_inner(repo, service_header, version, hidden_refs, true, |_| {
        let symref = head_symref.as_deref().map(|target| format!(" symref=HEAD:{target}")).unwrap_or_default();
        upload_pack_v1_capabilities(&symref)
    })
    .await
}

#[instrument(err, skip(repo, hidden_refs, capabilities))]
async fn ls_refs_all_inner(
    repo: &Git2Repository,
    service_header: Option<&str>,
    version: GitProtocol,
    hidden_refs: &HashSet<String>,
    upload_pack: bool,
    capabilities: impl Fn(&str) -> String,
) -> Result<Bytes> {
    let mut writer = GitWriter::new();

    if let Some(service) = service_header {
        writer.write_text(format!("# service={service}")).await?;
        writer.flush().await?;
    }

    if matches!(version, GitProtocol::V1) {
        writer.write_text("version 1").await?;
    }

    let mut refs = Vec::new();

    if upload_pack
        && let Ok(head) = repo.head()
        && let Some(oid) = head.target()
    {
        refs.push((oid, "HEAD".to_owned()));
    }

    for result in repo.references()? {
        match result {
            Ok(reference) => {
                if let Ok(name) = reference.name()
                    && !hidden_refs.contains(name)
                    && let Some(oid) = reference.target()
                {
                    refs.push((oid, name.to_owned()));
                }
            }
            Err(err) => {
                warn!(?err, repo_path = %repo.path().display(), "Failed to grab repository references");
            }
        }
    }

    for (index, (oid, name)) in refs.iter().enumerate() {
        let mut line = format!("{oid} {name}");

        // Git ignores capabilities written after the first line
        if index == 0 {
            line.push_str(&capabilities(name));
        }

        writer.write_text(line).await?;

        if upload_pack && let Some(peeled) = peeled_tag_target(repo, *oid) {
            writer.write_text(format!("{peeled} {name}^{{}}")).await?;
        }
    }

    if refs.is_empty() {
        writer
            .write_text(format!("0000000000000000000000000000000000000000 capabilities^{{}}{}", capabilities("")))
            .await?;
    }

    writer.flush().await?;
    writer.serialize().await
}

fn peeled_tag_target(repo: &Git2Repository, oid: Oid) -> Option<Oid> {
    let tag = repo.find_object(oid, Some(ObjectType::Tag)).ok()?;
    tag.peel(ObjectType::Any).ok().map(|object| object.id())
}

const fn receive_pack_capabilities() -> &'static str {
    concat!(
        "\x00report-status report-status-v2 delete-refs side-band-64k quiet push-options object-format=sha1 agent=git/gitarena-",
        env!("CARGO_PKG_VERSION")
    )
}

fn upload_pack_v1_capabilities(symref_cap: &str) -> String {
    format!(
        "\x00multi_ack thin-pack side-band side-band-64k ofs-delta shallow no-progress include-tag multi_ack_detailed no-done{symref_cap} object-format=sha1 agent=git/gitarena-{}",
        env!("CARGO_PKG_VERSION")
    )
}

#[derive(Debug, Default)]
pub(crate) struct LsRefs {
    pub(crate) peel: bool,
    pub(crate) symrefs: bool,
    pub(crate) prefixes: Vec<String>,
    pub(crate) unborn: bool,
}

impl LsRefs {
    fn matches(&self, name: &str) -> bool {
        // if no prefixes are given all refs are returned
        self.prefixes.is_empty() || self.prefixes.iter().any(|prefix| name.starts_with(prefix.as_str()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Context;
    use git2::{Oid, Signature, Time};
    use pretty_assertions::assert_eq;
    use tempfile::{TempDir, tempdir};

    fn args(input: &[&str]) -> Vec<Vec<u8>> {
        input.iter().map(|line| line.as_bytes().to_vec()).collect()
    }

    fn pkt_lines(bytes: &[u8]) -> Result<Vec<String>> {
        let mut lines = Vec::new();
        let mut rest = bytes;

        while !rest.is_empty() {
            let length = usize::from_str_radix(str::from_utf8(&rest[..4])?, 16)?;

            if length == 0 {
                lines.push("0000".to_owned());
                rest = &rest[4..];
                continue;
            }

            lines.push(String::from_utf8(rest[4..length].to_vec())?.trim_end_matches('\n').to_owned());
            rest = &rest[length..];
        }

        Ok(lines)
    }

    fn empty_repo() -> Result<(TempDir, Git2Repository)> {
        let dir = tempdir()?;
        let repo = Git2Repository::init_bare(dir.path())?;
        repo.set_head("refs/heads/main")?;

        Ok((dir, repo))
    }

    fn repo_with_commit() -> Result<(TempDir, Git2Repository, Oid)> {
        let (dir, repo) = empty_repo()?;

        let commit = {
            let signature = Signature::new("tester", "tester@example.com", &Time::new(1_700_000_000, 0))?;
            let blob = repo.blob(b"hello\n")?;

            let mut builder = repo.treebuilder(None)?;
            builder.insert("README.md", blob, 0o100_644)?;
            let tree = repo.find_tree(builder.write()?)?;

            repo.commit(Some("refs/heads/main"), &signature, &signature, "initial", &tree, &[])?
        };

        repo.reference("refs/tags/v1", commit, false, "lightweight tag")?;

        Ok((dir, repo, commit))
    }

    #[test]
    fn empty_prefix_list_matches_everything() {
        let options = LsRefs::default();

        assert!(options.matches("HEAD"));
        assert!(options.matches("refs/heads/main"));
    }

    #[test]
    fn prefixes_match_by_string_prefix() {
        let options = LsRefs {
            prefixes: vec!["refs/heads/".to_owned(), "HEAD".to_owned()],
            ..LsRefs::default()
        };

        assert!(options.matches("HEAD"));
        assert!(options.matches("refs/heads/main"));
        assert!(!options.matches("refs/tags/v1"));
    }

    #[tokio::test]
    async fn without_prefixes_all_refs_are_listed() -> Result<()> {
        let (_dir, repo, commit) = repo_with_commit()?;

        let mut lines = pkt_lines(&ls_refs(args(&["command=ls-refs", "symrefs"]), &repo, &HashSet::new()).await?)?;
        lines.sort();

        assert_eq!(
            lines,
            vec![
                "0000".to_owned(),
                format!("{commit} HEAD symref-target:refs/heads/main"),
                format!("{commit} refs/heads/main"),
                format!("{commit} refs/tags/v1"),
            ]
        );
        Ok(())
    }

    #[tokio::test]
    async fn ref_prefix_filters_refs() -> Result<()> {
        let (_dir, repo, commit) = repo_with_commit()?;

        let lines = pkt_lines(&ls_refs(args(&["ref-prefix refs/tags/"]), &repo, &HashSet::new()).await?)?;

        assert_eq!(lines, vec![format!("{commit} refs/tags/v1"), "0000".to_owned()]);
        Ok(())
    }

    #[tokio::test]
    async fn head_prefix_lists_only_head() -> Result<()> {
        let (_dir, repo, commit) = repo_with_commit()?;

        let lines = pkt_lines(&ls_refs(args(&["ref-prefix HEAD"]), &repo, &HashSet::new()).await?)?;

        assert_eq!(lines, vec![format!("{commit} HEAD"), "0000".to_owned()]);
        Ok(())
    }

    #[tokio::test]
    async fn symrefs_adds_symref_target() -> Result<()> {
        let (_dir, repo, commit) = repo_with_commit()?;

        let lines = pkt_lines(&ls_refs(args(&["symrefs", "ref-prefix HEAD"]), &repo, &HashSet::new()).await?)?;

        assert_eq!(lines, vec![format!("{commit} HEAD symref-target:refs/heads/main"), "0000".to_owned()]);
        Ok(())
    }

    #[tokio::test]
    async fn unborn_head_is_only_listed_when_requested() -> Result<()> {
        let (_dir, repo) = empty_repo()?;

        let without = pkt_lines(&ls_refs(args(&["symrefs", "ref-prefix HEAD"]), &repo, &HashSet::new()).await?)?;
        let with = pkt_lines(&ls_refs(args(&["symrefs", "ref-prefix HEAD", "unborn"]), &repo, &HashSet::new()).await?)?;

        assert_eq!(without, vec!["0000".to_owned()]);
        assert_eq!(with, vec!["unborn HEAD symref-target:refs/heads/main".to_owned(), "0000".to_owned()]);
        Ok(())
    }

    #[tokio::test]
    async fn unborn_head_requires_symrefs() -> Result<()> {
        let (_dir, repo) = empty_repo()?;

        let lines = pkt_lines(&ls_refs(args(&["ref-prefix HEAD", "unborn"]), &repo, &HashSet::new()).await?)?;

        assert_eq!(lines, vec!["0000".to_owned()]);
        Ok(())
    }

    #[tokio::test]
    async fn ls_refs_all_on_empty_repo_advertises_capabilities_line() -> Result<()> {
        let (_dir, repo) = empty_repo()?;

        let lines = pkt_lines(&ls_refs_all(&repo, Some("git-receive-pack"), GitProtocol::V0, &HashSet::new()).await?)?;

        assert_eq!(lines.len(), 4);
        assert_eq!(lines[0], "# service=git-receive-pack");
        assert_eq!(lines[1], "0000");
        assert!(
            lines[2].starts_with("0000000000000000000000000000000000000000 capabilities^{}\0report-status"),
            "{}",
            lines[2]
        );
        assert_eq!(lines[3], "0000");
        Ok(())
    }

    #[tokio::test]
    async fn ls_refs_all_puts_capabilities_on_first_ref_only() -> Result<()> {
        let (_dir, repo, commit) = repo_with_commit()?;

        let lines = pkt_lines(&ls_refs_all_upload_pack(&repo, None, GitProtocol::V0, &HashSet::new()).await?)?;

        let (first_ref, capabilities) = lines[0].split_once('\0').context("first ref has no capabilities")?;
        let mut refs = lines.iter().map(|line| line.split('\0').next().unwrap_or_default()).collect::<Vec<_>>();
        refs.sort_unstable();

        assert_eq!(first_ref, format!("{commit} HEAD"));
        assert!(capabilities.contains("symref=HEAD:refs/heads/main"), "{capabilities}");
        assert!(!lines[1].contains('\0'));
        assert_eq!(
            refs,
            vec![
                "0000".to_owned(),
                format!("{commit} HEAD"),
                format!("{commit} refs/heads/main"),
                format!("{commit} refs/tags/v1")
            ]
        );
        Ok(())
    }
}
