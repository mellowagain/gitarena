use crate::git::io::band::Band;
use crate::git::io::progress_writer::ProgressWriter;
use crate::git::io::writer::GitWriter;

use actix_web::web::Bytes;
use anyhow::{Context, Result};
use async_recursion::async_recursion;
use git2::{Buf, Commit, ErrorCode, ObjectType, Oid, PackBuilder, Repository as Git2Repository};
use tracing::instrument;
use tracing::warn;

#[instrument(err, skip(repo))]
pub(crate) async fn fetch(input: Vec<Vec<u8>>, repo: &Git2Repository, sideband: bool) -> Result<Bytes> {
    let mut options = Fetch {
        sideband,
        ..Default::default()
    };
    let mut writer = GitWriter::new();

    for raw_line in &input {
        let line = String::from_utf8(raw_line.clone())?;

        if line == "thin-pack" {
            options.thin_pack = true;
        }

        if line == "no-progress" {
            options.no_progress = true;
        }

        if line == "include-tag" {
            options.include_tag = true;
        }

        if line == "ofs-delta" {
            options.ofs_delta = true;
        }

        if let Some(stripped) = line.strip_prefix("have ") {
            options.have.push(stripped.to_owned());
        }

        if let Some(stripped) = line.strip_prefix("want ") {
            let mut parts = stripped.splitn(2, '\x00');
            let oid = parts.next().unwrap_or(stripped).trim();
            options.want.push(oid.to_owned());

            if let Some(caps) = parts.next()
                && caps.split(' ').any(|c| c == "side-band-64k")
            {
                options.sideband = true;
            }
        }

        /*if line.starts_with("shallow ") {
            options.shallow.push(line[8..].to_owned());
        }

        if line.starts_with("deepen ") {
            options.deepen = Some(line[7..].parse::<i32>()?);
        }

        if line == "deepen-relative" {
            options.deepen_relative = true;
        }

        if line.starts_with("deepen-since ") && options.deepen.is_none() {
            let timestamp_str = &line[13..];
            //parse timestamp_str to DateTime<Utc>
        }

        if line.starts_with("deepen-not ") && options.deepen.is_none() {
            options.deepen_not = Some(line[11..].to_owned());
        }*/

        if line == "done" {
            options.done = true;
            break;
        }
    }

    if options.done {
        if let Some(wants) = process_wants(repo, &options).await? {
            writer.append(wants).await?;
        }
    } else {
        let (acknowledgments, sent_ready) = process_haves(repo, &options).await?;

        if let Some(acknowledgments) = acknowledgments {
            writer.append(acknowledgments).await?;
        }

        if sent_ready {
            writer.delimiter().await?;

            if let Some(wants) = process_wants(repo, &options).await? {
                writer.append(wants).await?;
            }
        }
    }

    /*if let Some(mut shallows) = process_shallows(&repo, &options).await? {
        writer = writer.append(&mut shallows);
    }*/

    writer.flush().await?;
    writer.serialize().await
}

#[instrument(err, skip(repo))]
pub(crate) async fn process_haves(repo: &Git2Repository, options: &Fetch) -> Result<(Option<GitWriter>, bool)> {
    if options.have.is_empty() {
        return Ok((None, false));
    }

    let mut written_one = false;
    let mut writer = GitWriter::new();
    writer.write_text("acknowledgments").await?;

    for have in &options.have {
        let oid = match Oid::from_str(have.as_str()) {
            Ok(oid) => oid,
            Err(err) => {
                warn!(?err, "Invalid OID in have: {have}");
                continue;
            }
        };

        match repo.find_object(oid, None) {
            Ok(_) => {
                writer.write_text(format!("ACK {have}")).await?;
                written_one = true;
            }
            Err(err) if err.code() == ErrorCode::NotFound => { /* client has a commit we dont have, just ignore */ }
            Err(err) => warn!(?err, "Error looking up have object: {have}"),
        }
    }

    if written_one {
        writer.write_text("ready").await?;
    } else {
        writer.write_text("NAK").await?;
    }

    Ok((Some(writer), written_one))
}

#[instrument(err, skip(repo))]
pub(crate) async fn process_wants(repo: &Git2Repository, options: &Fetch) -> Result<Option<GitWriter>> {
    let mut writer = GitWriter::new();
    writer.write_text("packfile").await?;

    writer
        .write_text_sideband(Band::Progress, format!("Enumerating objects: {}, done.", options.want.len()))
        .await?;

    let mut progress_writer = ProgressWriter::new();

    let (buffer, object_count, written) = {
        let mut pack_builder = repo.packbuilder()?;

        pack_builder.set_threads(u32::try_from(num_cpus::get()).context("cpu threads available is larger than u32")?);
        pack_builder.set_progress_callback(progress_writer.pack_builder_callback())?;

        for wanted_obj in &options.want {
            match repo.find_object(Oid::from_str(wanted_obj.as_str())?, None) {
                Ok(object) => {
                    if let Some(kind) = object.kind() {
                        match kind {
                            ObjectType::Commit => {
                                // Can be simplified with if let guards: https://github.com/rust-lang/rust/issues/51114
                                if let Some(commit) = object.as_commit() {
                                    insert_commit_with_parents(commit, &mut pack_builder).await?;
                                }
                            }
                            ObjectType::Tree => pack_builder.insert_tree(object.id())?,
                            _ => pack_builder.insert_object(object.id(), Some(wanted_obj.as_str()))?,
                        }
                    } else {
                        pack_builder.insert_object(object.id(), Some(wanted_obj.as_str()))?;
                    }
                }
                Err(err) => {
                    warn!(?err, "Unable to find wanted object: {}", &wanted_obj);
                }
            }
        }

        let mut buf = Buf::new();
        pack_builder.write_buf(&mut buf)?;

        (buf, pack_builder.object_count(), pack_builder.written())
    };

    writer.append(progress_writer.to_writer().await?).await?;

    if options.sideband {
        writer.write_binary_sideband_chunked(Band::Data, buffer.as_ref()).await?;
    } else {
        writer.write_binary(buffer.as_ref()).await?;
    }

    let total = object_count;
    let total_delta = progress_writer.delta_total.unwrap_or_default() as usize;

    let reused = total.saturating_sub(written);
    let reused_delta = total_delta.saturating_sub(reused);

    writer
        .write_text_sideband(
            Band::Progress,
            format!("Total {total} (delta {total_delta}), reused {reused} (delta {reused_delta})"),
        )
        .await?;

    Ok(Some(writer))
}

#[instrument(err, skip(pack_builder))]
#[async_recursion(?Send)]
async fn insert_commit_with_parents(commit: &Commit<'_>, pack_builder: &mut PackBuilder<'_>) -> Result<()> {
    pack_builder.insert_commit(commit.id())?;

    for parent in commit.parents() {
        insert_commit_with_parents(&parent, pack_builder).await?;
    }

    Ok(())
}

/*pub(crate) async fn process_shallows(repo: &Git2Repository, options: &Fetch) -> Result<Option<GitWriter>> {
    if !repo.is_shallow() || options.shallow.is_empty() {
        return Ok(None);
    }

    let mut writer = GitWriter::new();
    writer = writer.write_text("shallow-info")?;

    // ...

    Ok(Some(writer))
}*/

#[derive(Debug, Default)]
pub(crate) struct Fetch {
    pub(crate) thin_pack: bool,
    pub(crate) no_progress: bool,
    pub(crate) include_tag: bool,
    pub(crate) ofs_delta: bool, // PACKv2
    pub(crate) done: bool,
    pub(crate) sideband: bool,
    pub(crate) have: Vec<String>,
    pub(crate) want: Vec<String>,
    /*pub(crate) shallow: Vec<String>,
    pub(crate) deepen: Option<i32>,
    pub(crate) deepen_relative: bool,
    pub(crate) deepen_since: Option<DateTime<Utc>>,
    pub(crate) deepen_not: Option<String>*/
}
