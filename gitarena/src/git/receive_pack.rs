use crate::git::io::band::Band;
use crate::git::io::writer::GitWriter;
use crate::git::ref_update::RefUpdate;
use crate::prelude::*;
use crate::repository::Repository;
use crate::utils::oid;
use crate::{die, err};

use std::convert::TryInto;
use std::sync::Arc;

use anyhow::{Result, anyhow};
use bstr::BString;
use gitarena_common::database::Database;
use gix::actor::Signature;
use gix::date::parse::TimeBuf;
use gix::lock::acquire::Fail;
use gix::objs::{CommitRef, Kind, TagRef};
use gix::odb::Store;
use gix::odb::pack::FindExt;
use gix::refs::Target;
use gix::refs::transaction::{Change, LogChange, PreviousValue, RefEdit, RefLog};
use sqlx::{PgPool, Transaction};
use tracing::instrument;

#[instrument(err, skip(writer, store))]
pub(crate) async fn process_create_update(
    ref_update: &RefUpdate,
    repo: &Repository,
    store: Arc<Store>,
    db_pool: &PgPool,
    writer: &mut GitWriter,
) -> Result<()> {
    assert!(ref_update.new.is_some());

    let mut transaction = db_pool.begin().await?;
    let new_oid = oid::from_hex_str(ref_update.new.as_deref())?;

    // # Gitoxide zone
    // The pack file was written by libgit2 before so we can now just search for the entries
    // and then create a Gitoxide commit and write it to the reflog using a transaction
    {
        let mut buffer = Vec::<u8>::new();

        let (committer, message) = {
            let (data, _) = store
                .to_cache_arc()
                .find(new_oid.as_ref(), &mut buffer)
                .map_err(|_| anyhow!("Failed to find object {} in ODB after pack write", new_oid))?;

            match data.kind {
                Kind::Commit => {
                    let commit = CommitRef::from_bytes(data.data)?;
                    (commit.committer.to_owned()?, BString::from(commit.message))
                }
                Kind::Tag => {
                    let tag = TagRef::from_bytes(data.data)?;
                    let tagger = tag.tagger.ok_or_else(|| anyhow!("Pushed tag object has no tagger field"))?;
                    (tagger.to_owned()?, BString::from(tag.message))
                }
                _ => die!(BAD_REQUEST, "Unexpected payload data type"),
            }
        };

        let previous_value = if let Some(previous_oid_str) = &ref_update.old {
            let previous_oid = oid::from_hex_str(Some(previous_oid_str.as_str()))?;
            let previous_target = Target::Object(previous_oid);

            PreviousValue::ExistingMustMatch(previous_target)
        } else {
            PreviousValue::Any
        };

        let edits = vec![RefEdit {
            change: Change::Update {
                log: LogChange {
                    mode: RefLog::AndReference,
                    force_create_reflog: true,
                    message,
                },
                expected: previous_value,
                new: Target::Object(new_oid),
            },
            name: ref_update.target_ref.as_str().try_into()?,
            deref: true,
        }];

        let gitoxide_repo = repo.gitoxide(&mut transaction).await?;

        gitoxide_repo
            .refs
            .transaction()
            .prepare(edits, Fail::Immediately, Fail::Immediately)
            .map_err(|err| anyhow!("Failed to commit transaction: {}", err))?
            .commit(committer.to_ref(&mut TimeBuf::default()))?;
    }

    if ref_update.report_status || ref_update.report_status_v2 {
        writer.write_text_sideband_pktline(Band::Data, format!("ok {}", ref_update.target_ref)).await?;
    }

    Ok(())
}

#[instrument(err, skip(tx, writer))]
pub(crate) async fn process_delete(ref_update: &RefUpdate, repo: &Repository, tx: &mut Transaction<'_, Database>, writer: &mut GitWriter) -> Result<()> {
    assert!(ref_update.old.is_some());
    assert!(ref_update.new.is_none());

    let gitoxide_repo = repo.gitoxide(tx).await?;

    let object_id = oid::from_hex_str(ref_update.old.as_deref()).map_err(|_| err!(NOT_FOUND, "Ref does not exist"))?;

    let edits = vec![RefEdit {
        change: Change::Delete {
            expected: PreviousValue::MustExistAndMatch(Target::Object(object_id)),
            log: RefLog::AndReference,
        },
        name: ref_update.target_ref.as_str().try_into()?,
        deref: true,
    }];

    gitoxide_repo
        .refs
        .transaction()
        .prepare(edits, Fail::Immediately, Fail::Immediately)
        .map_err(|err| err!(INTERNAL_SERVER_ERROR, "Failed to commit transaction: {}", err))?
        .commit(Signature::gitarena_default().to_ref(&mut TimeBuf::default()))?;

    if ref_update.report_status || ref_update.report_status_v2 {
        writer.write_text_sideband_pktline(Band::Data, format!("ok {}", ref_update.target_ref)).await?;
    }

    Ok(())
}
