use crate::git::io::band::Band;
use crate::git::io::writer::GitWriter;
use crate::git::ref_update::RefUpdate;
use crate::git::GIT_HASH_KIND;
use crate::prelude::*;
use crate::repository::Repository;
use crate::utils::oid;
use crate::{die, err};

use std::convert::TryInto;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use bstr::BString;
use git_repository::actor::Signature;
use git_repository::lock::acquire::Fail;
use git_repository::objs::{CommitRef, Kind};
use git_repository::odb::pack::data::{File as DataFile, ResolvedBase};
use git_repository::odb::pack::index::File as IndexFile;
use git_repository::odb::pack::{cache, FindExt};
use git_repository::odb::Store;
use git_repository::refs::transaction::{Change, LogChange, PreviousValue, RefEdit, RefLog};
use git_repository::refs::Target;
use sqlx::{Executor, PgPool, Postgres};
use tracing::instrument;

#[instrument(err, skip(writer, store))]
pub(crate) async fn process_create_update(
    ref_update: &RefUpdate,
    repo: &Repository,
    store: Arc<Store>,
    db_pool: &PgPool,
    writer: &mut GitWriter,
    index_path: Option<&PathBuf>,
    pack_path: Option<&PathBuf>,
    raw_pack: &[u8],
) -> Result<()> {
    assert!(ref_update.new.is_some());

    let mut transaction = db_pool.begin().await?;
    let new_oid = oid::from_hex_str(ref_update.new.as_deref())?;

    // # Gitoxide zone
    // This block decodes the entry from the pack file, creates a Gitoxide Commit and then writes it to the reflog using a transaction
    {
        let mut buffer = Vec::<u8>::new();

        let commit = match (index_path, pack_path) {
            (Some(index_path), Some(pack_path)) => {
                let index_file = IndexFile::at(index_path, GIT_HASH_KIND)?;

                let index = index_file
                    .lookup(new_oid.as_ref())
                    .ok_or_else(|| anyhow!("Failed to lookup new oid in index file"))?;
                let offset = index_file.pack_offset_at_index(index);

                let data_file = DataFile::at(pack_path, GIT_HASH_KIND)?;

                let entry = data_file.entry(offset);

                buffer.reserve(entry.decompressed_size as usize);

                let outcome = data_file.decode_entry(
                    entry,
                    &mut buffer,
                    |oid, vec| {
                        if let Some(index) = index_file.lookup(oid) {
                            let offset = index_file.pack_offset_at_index(index);
                            let entry = data_file.entry(offset);

                            Some(ResolvedBase::InPack(entry))
                        } else {
                            store.to_cache_arc().find(oid, vec).ok().map(|(data, _)| {
                                ResolvedBase::OutOfPack {
                                    kind: data.kind,
                                    end: data.data.len(),
                                }
                            })
                        }
                    },
                    &mut cache::Never,
                )?;

                match outcome.kind {
                    Kind::Commit => CommitRef::from_bytes(buffer.as_slice())?,
                    _ => die!(BAD_REQUEST, "Unexpected payload data type"),
                }
            }
            _ => {
                // This is a force push to an existing repository
                // TODO: Handle non existing refs as client errors instead of server errors
                store
                    .to_cache_arc()
                    .find_commit(new_oid.as_ref(), &mut buffer)
                    .map(|(data, _)| data)?
            }
        };

        let previous_value = if let Some(previous_oid_str) = &ref_update.old {
            let previous_oid = oid::from_hex_str(Some(previous_oid_str.as_str()))?;
            let previous_target = Target::Peeled(previous_oid);

            PreviousValue::ExistingMustMatch(previous_target)
        } else {
            PreviousValue::Any
        };

        let edits = vec![RefEdit {
            change: Change::Update {
                log: LogChange {
                    mode: RefLog::AndReference,
                    force_create_reflog: true,
                    message: BString::from(commit.message),
                },
                expected: previous_value,
                new: Target::Peeled(new_oid),
            },
            name: ref_update.target_ref.as_str().try_into()?,
            deref: true,
        }];

        let gitoxide_repo = repo.gitoxide(&mut transaction).await?;

        gitoxide_repo
            .refs
            .transaction()
            .prepare(edits, Fail::Immediately)
            .map_err(|err| anyhow!("Failed to commit transaction: {}", err))?
            .commit(&Signature::from(commit.committer))?;
    }

    // # libgit2 zone
    // This block writes the payload into the repo odb
    {
        let git2_repo = repo.libgit2(&mut transaction).await?;

        let odb = git2_repo.odb()?;
        let mut pack_writer = odb.packwriter()?;

        pack_writer.write_all(raw_pack)?;
        pack_writer.commit()?;
    }

    if ref_update.report_status || ref_update.report_status_v2 {
        writer
            .write_text_sideband_pktline(Band::Data, format!("ok {}", ref_update.target_ref))
            .await?;
    }

    Ok(())
}

#[instrument(err, skip(writer))]
pub(crate) async fn process_delete<'e, E: Executor<'e, Database = Postgres>>(
    ref_update: &RefUpdate,
    repo: &Repository,
    executor: E,
    writer: &mut GitWriter,
) -> Result<()> {
    assert!(ref_update.old.is_some());
    assert!(ref_update.new.is_none());

    let gitoxide_repo = repo.gitoxide(executor).await?;

    let object_id = oid::from_hex_str(ref_update.old.as_deref())
        .map_err(|_| err!(NOT_FOUND, "Ref does not exist"))?;

    let edits = vec![RefEdit {
        change: Change::Delete {
            expected: PreviousValue::MustExistAndMatch(Target::Peeled(object_id)),
            log: RefLog::AndReference,
        },
        name: ref_update.target_ref.as_str().try_into()?,
        deref: true,
    }];

    gitoxide_repo
        .refs
        .transaction()
        .prepare(edits, Fail::Immediately)
        .map_err(|err| {
            err!(
                INTERNAL_SERVER_ERROR,
                "Failed to commit transaction: {}",
                err
            )
        })?
        .commit(&Signature::gitarena_default())?;

    if ref_update.report_status || ref_update.report_status_v2 {
        writer
            .write_text_sideband_pktline(Band::Data, format!("ok {}", ref_update.target_ref))
            .await?;
    }

    Ok(())
}
