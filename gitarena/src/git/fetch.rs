use crate::die;
use crate::git::io::band::Band;
use crate::git::io::progress_writer::ProgressWriter;
use crate::git::io::writer::GitWriter;

use actix_web::web::Bytes;
use anyhow::{Context, Result};
use git2::{Buf, ErrorCode, ObjectType, Oid, Repository as Git2Repository, TreeWalkMode, TreeWalkResult};
use std::collections::{HashSet, VecDeque};
use tracing::instrument;
use tracing::warn;

#[instrument(err, skip(repo, hidden_refs))]
pub(crate) async fn fetch(input: Vec<Vec<u8>>, repo: &Git2Repository, sideband: bool, hidden_refs: &HashSet<String>) -> Result<Bytes> {
    let mut options = Fetch {
        sideband,
        ..Default::default()
    };
    let mut writer = GitWriter::new();

    for raw_line in &input {
        let line = String::from_utf8(raw_line.clone())?;

        options.set_capability(&line);

        if let Some(stripped) = line.strip_prefix("have ") {
            options.have.push(stripped.to_owned());
        }

        if let Some(stripped) = line.strip_prefix("want ") {
            let mut parts = stripped.split(' ');

            let oid = parts.next().unwrap_or(stripped);
            options.want.push(oid.to_owned());

            if parts.any(|c| c == "side-band-64k") {
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
        if let Some(wants) = process_wants(repo, &options, hidden_refs).await? {
            writer.append(wants).await?;
        }
    } else {
        let (acknowledgments, sent_ready) = process_haves(repo, &options).await?;

        if let Some(acknowledgments) = acknowledgments {
            writer.append(acknowledgments).await?;
        }

        if sent_ready {
            writer.delimiter().await?;

            if let Some(wants) = process_wants(repo, &options, hidden_refs).await? {
                writer.append(wants).await?;
            }
        }
    }

    /*if let Some(mut shallows) = process_shallows(&repo, &options).await? {
        writer = writer.append(&mut shallows);
    }*/

    if options.sideband {
        writer.flush().await?;
    }

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

#[instrument(err, skip(repo, hidden_refs))]
pub(crate) async fn process_wants(repo: &Git2Repository, options: &Fetch, hidden_refs: &HashSet<String>) -> Result<Option<GitWriter>> {
    let mut writer = GitWriter::new();
    writer.write_text("packfile").await?;

    let common = options.have.iter().filter_map(|have| Oid::from_str(have).ok()).collect::<Vec<_>>();
    writer.append(write_pack(repo, options, &common, hidden_refs).await?).await?;

    Ok(Some(writer))
}

#[instrument(err, skip(repo, hidden_refs))]
pub(crate) async fn write_pack(repo: &Git2Repository, options: &Fetch, common: &[Oid], hidden_refs: &HashSet<String>) -> Result<GitWriter> {
    let mut writer = GitWriter::new();

    if options.sideband && !options.no_progress {
        writer
            .write_text_sideband(Band::Progress, format!("Enumerating objects: {}, done.", options.want.len()))
            .await?;
    }

    let mut progress_writer = ProgressWriter::new();

    let (buffer, object_count, written) = {
        let mut pack_builder = repo.packbuilder()?;

        pack_builder.set_threads(u32::try_from(num_cpus::get()).context("cpu threads available is larger than u32")?);
        pack_builder.set_progress_callback(progress_writer.pack_builder_callback())?;

        let wanted = wanted_objects(repo, options, hidden_refs)?;

        for &oid in wanted.tags.iter().chain(&wanted.blobs) {
            pack_builder.insert_object(oid, None)?;
        }

        for &tree in &wanted.trees {
            pack_builder.insert_tree(tree)?;
        }

        let common_commits: Vec<_> = common.iter().copied().filter(|oid| repo.find_commit(*oid).is_ok()).collect();

        let known: HashSet<_> = walk_commits(repo, &common_commits, None, &options.shallow)?.commits.into_iter().collect();

        for commit in walk_commits(repo, &wanted.commits, options.deepen, &options.shallow)?.commits {
            if !known.contains(&commit) {
                pack_builder.insert_commit(commit)?;
            }
        }

        let mut buf = Buf::new();
        pack_builder.write_buf(&mut buf)?;

        (buf, pack_builder.object_count(), pack_builder.written())
    };

    if !options.sideband {
        writer.write_raw(buffer.as_ref()).await?;
        return Ok(writer);
    }

    let max_chunk = if options.small_sideband { 995 } else { 65515 };

    if !options.no_progress {
        writer.append(progress_writer.to_writer().await?).await?;
    }

    writer.write_binary_sideband_chunked(Band::Data, buffer.as_ref(), max_chunk).await?;

    if !options.no_progress {
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
    }

    Ok(writer)
}

#[instrument(err, skip(repo, hidden_refs))]
pub(crate) fn wanted_objects(repo: &Git2Repository, options: &Fetch, hidden_refs: &HashSet<String>) -> Result<WantedObjects> {
    let wants = options.want.iter().map(|want| Oid::from_str(want)).collect::<Result<Vec<_>, _>>()?;
    let reachable = reachable_objects(repo, &wants, hidden_refs)?;
    let mut wanted = WantedObjects::default();

    for oid in wants {
        if !reachable.contains(&oid) {
            die!(BAD_REQUEST, "wanted object {oid} is not reachable from any ref");
        }

        let mut object = repo.find_object(oid, None)?;

        while let Some(tag) = object.as_tag() {
            wanted.tags.push(tag.id());
            object = tag.target()?;
        }

        match object.kind() {
            Some(ObjectType::Commit) => wanted.commits.push(object.id()),
            Some(ObjectType::Tree) => wanted.trees.push(object.id()),
            _ => wanted.blobs.push(object.id()),
        }
    }

    Ok(wanted)
}

#[derive(Debug, Default)]
pub(crate) struct WantedObjects {
    pub(crate) tags: Vec<Oid>,
    pub(crate) commits: Vec<Oid>,
    pub(crate) trees: Vec<Oid>,
    pub(crate) blobs: Vec<Oid>,
}

#[derive(Debug, Default)]
pub(crate) struct CommitWalk {
    pub(crate) commits: Vec<Oid>,
    pub(crate) boundary: HashSet<Oid>,
}

#[instrument(err, skip(repo, shallow))]
pub(crate) fn walk_commits(repo: &Git2Repository, tips: &[Oid], deepen: Option<u32>, shallow: &HashSet<Oid>) -> Result<CommitWalk> {
    let mut walk = CommitWalk::default();
    let mut seen = HashSet::new();
    let mut queue = VecDeque::new();

    for &tip in tips {
        if seen.insert(tip) {
            queue.push_back((tip, 1));
        }
    }

    while let Some((oid, depth)) = queue.pop_front() {
        walk.commits.push(oid);

        let commit = repo.find_commit(oid)?;

        let descend = match deepen {
            Some(deepen) => depth < deepen,
            None => !shallow.contains(&oid),
        };

        if !descend {
            if deepen.is_some() && commit.parent_count() > 0 {
                walk.boundary.insert(oid);
            }

            continue;
        }

        for parent in commit.parent_ids() {
            if seen.insert(parent) {
                queue.push_back((parent, depth + 1));
            }
        }
    }

    Ok(walk)
}

#[instrument(err, skip(repo, hidden_refs))]
pub(crate) fn reachable_objects(repo: &Git2Repository, oids: &[Oid], hidden_refs: &HashSet<String>) -> Result<HashSet<Oid>> {
    let mut pending: HashSet<Oid> = oids.iter().copied().collect();
    let mut reachable = HashSet::new();
    let mut revwalk = repo.revwalk()?;

    for reference in repo.references()? {
        let reference = reference?;

        if reference.name().is_ok_and(|name| hidden_refs.contains(name)) {
            continue;
        }

        let peeled = match reference.peel(ObjectType::Any) {
            Ok(object) => object.id(),
            Err(err) if err.code() == ErrorCode::NotFound => continue,
            Err(err) => return Err(err.into()),
        };

        for oid in reference.target().into_iter().chain([peeled]) {
            if pending.remove(&oid) {
                reachable.insert(oid);
            }
        }

        if let Ok(commit) = reference.peel_to_commit() {
            revwalk.push(commit.id())?;
        }
    }

    if pending.is_empty() {
        return Ok(reachable);
    }

    let walk_trees = pending
        .iter()
        .any(|oid| repo.find_object(*oid, None).is_ok_and(|object| object.kind() != Some(ObjectType::Commit)));

    let mut seen_trees = HashSet::new();

    for oid in revwalk {
        let oid = oid?;

        if pending.remove(&oid) {
            reachable.insert(oid);
        }

        if walk_trees && !pending.is_empty() {
            let tree = repo.find_commit(oid)?.tree()?;

            if seen_trees.insert(tree.id()) {
                if pending.remove(&tree.id()) {
                    reachable.insert(tree.id());
                }

                tree.walk(TreeWalkMode::PreOrder, |_, entry| {
                    if pending.remove(&entry.id()) {
                        reachable.insert(entry.id());
                    }

                    if entry.kind() == Some(ObjectType::Tree) && !seen_trees.insert(entry.id()) {
                        TreeWalkResult::Skip
                    } else {
                        TreeWalkResult::Ok
                    }
                })?;
            }
        }

        if pending.is_empty() {
            break;
        }
    }

    Ok(reachable)
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
    pub(crate) shallow: HashSet<Oid>,
    pub(crate) deepen: Option<u32>,
    pub(crate) small_sideband: bool,
    /*pub(crate) deepen_relative: bool,
    pub(crate) deepen_since: Option<DateTime<Utc>>,
    pub(crate) deepen_not: Option<String>*/
}

impl Fetch {
    pub(crate) fn set_capability(&mut self, capability: &str) {
        match capability {
            "thin-pack" => self.thin_pack = true,
            "no-progress" => self.no_progress = true,
            "include-tag" => self.include_tag = true,
            "ofs-delta" => self.ofs_delta = true,
            _ => {}
        }
    }
}
