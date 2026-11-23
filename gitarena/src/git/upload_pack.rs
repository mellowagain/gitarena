use crate::git::fetch::{Fetch, fetch, walk_commits, wanted_objects, write_pack};
use crate::git::io::reader::{read_data_lines, read_until_command};
use crate::git::io::writer::GitWriter;
use crate::git::ls_refs::ls_refs;

use std::collections::HashSet;

use anyhow::{Result, bail};
use git2::{Oid, Repository};
use gix::protocol::transport::packetline::PacketLineRef;
use gix::protocol::transport::packetline::async_io::StreamingPeekableIter;
use gix::protocol::transport::packetline::decode::{Stream, streaming};
use tracing::instrument;

#[instrument(err, skip(data, repo, hidden_refs))]
pub(crate) async fn execute_upload_pack_v2(data: &[u8], repo: &Repository, hidden_refs: &HashSet<String>) -> Result<Vec<u8>> {
    let mut readable_iter = StreamingPeekableIter::new(data, &[PacketLineRef::Flush], false);
    readable_iter.fail_on_err_lines(true);

    let git_body = read_data_lines(&mut readable_iter).await?;
    let (command, body) = read_until_command(git_body).await?;

    let output = match command.as_str() {
        "ls-refs" => ls_refs(body, repo, hidden_refs).await?,
        "fetch" => fetch(body, repo, true, hidden_refs).await?,
        _ => bail!("unknown git-upload-pack command: {command}"),
    };

    Ok(output.to_vec())
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
enum MultiAck {
    #[default]
    None,
    Basic,
    Detailed,
}

#[derive(Debug, Default)]
pub(crate) struct UploadPackV1 {
    stateless: bool,
    fetch: Fetch,
    multi_ack: MultiAck,
    no_done: bool,
    received_wants: bool,
    common: Vec<Oid>,
    last_common: Option<Oid>,
    got_common: bool,
    got_other: bool,
    sent_ready: bool,
    finished: bool,
}

impl UploadPackV1 {
    pub(crate) fn stateless() -> UploadPackV1 {
        UploadPackV1 {
            stateless: true,
            ..Default::default()
        }
    }

    pub(crate) fn finished(&self) -> bool {
        self.finished
    }

    #[instrument(err, skip(self, data, repo, hidden_refs))]
    pub(crate) async fn process(&mut self, data: &[u8], repo: &Repository, hidden_refs: &HashSet<String>) -> Result<Vec<u8>> {
        let mut writer = GitWriter::new();
        let mut offset = 0;

        while !self.finished && offset < data.len() {
            let (line, consumed) = match streaming(&data[offset..])? {
                Stream::Complete { line, bytes_consumed } => (line, bytes_consumed),
                Stream::Incomplete { .. } => bail!("incomplete pkt-line in upload-pack request"),
            };

            offset += consumed;

            match line {
                PacketLineRef::Data(line) => {
                    let line = str::from_utf8(line)?.trim_end_matches('\n');

                    if self.received_wants {
                        self.have_line(line, repo, hidden_refs, &mut writer).await?;
                    } else {
                        self.want_line(line)?;
                    }
                }
                PacketLineRef::Flush if !self.received_wants => self.end_wants(repo, hidden_refs, &mut writer).await?,
                PacketLineRef::Flush => {
                    self.end_haves(repo, hidden_refs, &mut writer).await?;

                    if self.stateless {
                        break;
                    }
                }
                PacketLineRef::Delimiter | PacketLineRef::ResponseEnd => bail!("unexpected protocol v2 packet in upload-pack request"),
            }
        }

        Ok(writer.serialize().await?.to_vec())
    }

    fn want_line(&mut self, line: &str) -> Result<()> {
        let mut parts = line.split(' ');

        match (parts.next(), parts.next()) {
            (Some("want"), Some(oid)) => self.fetch.want.push(oid.to_owned()),
            (Some("shallow"), Some(oid)) => {
                self.fetch.shallow.insert(Oid::from_str(oid)?);
            }
            (Some("deepen"), Some(depth)) => self.fetch.deepen = Some(depth.parse()?),
            _ => bail!("unexpected line in git-upload-pack want list: {line}"),
        }

        for capability in parts {
            match capability {
                "multi_ack" => self.multi_ack = MultiAck::Basic,
                "multi_ack_detailed" => self.multi_ack = MultiAck::Detailed,
                "no-done" => self.no_done = true,
                "side-band-64k" => self.fetch.sideband = true,
                "side-band" => {
                    self.fetch.sideband = true;
                    self.fetch.small_sideband = true;
                }
                _ => self.fetch.set_capability(capability),
            }
        }

        Ok(())
    }

    async fn end_wants(&mut self, repo: &Repository, hidden_refs: &HashSet<String>, writer: &mut GitWriter) -> Result<()> {
        self.received_wants = true;

        if self.fetch.want.is_empty() {
            self.finished = true;
            return Ok(());
        }

        if self.fetch.deepen.is_none() {
            return Ok(());
        }

        let tips = wanted_objects(repo, &self.fetch, hidden_refs)?.commits;
        let walk = walk_commits(repo, &tips, self.fetch.deepen, &self.fetch.shallow)?;

        for oid in walk
            .commits
            .iter()
            .filter(|oid| walk.boundary.contains(oid) && !self.fetch.shallow.contains(oid))
        {
            writer.write_text(format!("shallow {oid}")).await?;
        }

        for oid in walk
            .commits
            .iter()
            .filter(|oid| self.fetch.shallow.contains(oid) && !walk.boundary.contains(oid))
        {
            writer.write_text(format!("unshallow {oid}")).await?;
        }

        writer.flush().await?;
        Ok(())
    }

    async fn have_line(&mut self, line: &str, repo: &Repository, hidden_refs: &HashSet<String>, writer: &mut GitWriter) -> Result<()> {
        if line == "done" {
            match self.last_common {
                Some(oid) if self.multi_ack != MultiAck::None => {
                    writer.write_text(format!("ACK {oid}")).await?;
                }
                Some(_) => {}
                None => {
                    writer.write_text("NAK").await?;
                }
            }

            return self.send_pack(repo, hidden_refs, writer).await;
        }

        let Some(oid) = line.strip_prefix("have ") else {
            bail!("unexpected line in git-upload-pack negotiation: {line}");
        };

        let oid = Oid::from_str(oid)?;

        if !repo.odb()?.exists(oid) {
            self.got_other = true;

            if self.multi_ack != MultiAck::None && self.ok_to_give_up(repo)? {
                if self.multi_ack == MultiAck::Detailed {
                    self.sent_ready = true;
                    writer.write_text(format!("ACK {oid} ready")).await?;
                } else {
                    writer.write_text(format!("ACK {oid} continue")).await?;
                }
            }

            return Ok(());
        }

        self.got_common = true;
        self.last_common = Some(oid);

        if !self.common.contains(&oid) {
            self.common.push(oid);
        }

        let ack = match self.multi_ack {
            MultiAck::Detailed => Some(format!("ACK {oid} common")),
            MultiAck::Basic => Some(format!("ACK {oid} continue")),
            MultiAck::None if self.common.len() == 1 => Some(format!("ACK {oid}")),
            MultiAck::None => None,
        };

        if let Some(ack) = ack {
            writer.write_text(ack).await?;
        }

        Ok(())
    }

    async fn end_haves(&mut self, repo: &Repository, hidden_refs: &HashSet<String>, writer: &mut GitWriter) -> Result<()> {
        if self.multi_ack == MultiAck::Detailed
            && self.got_common
            && !self.got_other
            && let Some(oid) = self.last_common
            && self.ok_to_give_up(repo)?
        {
            self.sent_ready = true;
            writer.write_text(format!("ACK {oid} ready")).await?;
        }

        if self.common.is_empty() || self.multi_ack != MultiAck::None {
            writer.write_text("NAK").await?;
        }

        if self.no_done
            && self.sent_ready
            && let Some(oid) = self.last_common
        {
            writer.write_text(format!("ACK {oid}")).await?;
            return self.send_pack(repo, hidden_refs, writer).await;
        }

        self.got_common = false;
        self.got_other = false;

        Ok(())
    }

    fn ok_to_give_up(&self, repo: &Repository) -> Result<bool> {
        if self.common.is_empty() {
            return Ok(false);
        }

        for want in &self.fetch.want {
            let object = repo.find_object(Oid::from_str(want)?, None)?;

            let Ok(commit) = object.peel_to_commit() else {
                continue;
            };

            let mut reached = false;

            for &have in &self.common {
                if have == commit.id() || repo.graph_descendant_of(commit.id(), have)? {
                    reached = true;
                    break;
                }
            }

            if !reached {
                return Ok(false);
            }
        }

        Ok(true)
    }

    async fn send_pack(&mut self, repo: &Repository, hidden_refs: &HashSet<String>, writer: &mut GitWriter) -> Result<()> {
        self.finished = true;

        writer.append(write_pack(repo, &self.fetch, &self.common, hidden_refs).await?).await?;

        if self.fetch.sideband {
            writer.flush().await?;
        }

        Ok(())
    }
}
