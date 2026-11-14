use std::fs::{create_dir_all, rename};
use std::io::ErrorKind;
use std::mem::DropGuard;
use std::path::Path;

use crate::queue::GLOBAL_QUEUE;
use crate::repository::task::DeletedRepoCleanupTask;

use anyhow::Result;
use async_recursion::async_recursion;
use fang::{AsyncQueueable, AsyncRunnable};
use tokio::fs;
use tokio::spawn;
use tracing::{error, instrument};
use uuid::Uuid;

// Adopted from https://stackoverflow.com/a/65192210 (turned it async using tokio::fs)
#[async_recursion(?Send)]
#[instrument(skip_all, fields(source = source.as_ref().display().to_string(), destination = destination.as_ref().display().to_string()))]
#[allow(clippy::multiple_bound_locations)]
pub(crate) async fn copy_dir_all<P: AsRef<Path>>(source: P, destination: P) -> Result<()> {
    fs::create_dir_all(destination.as_ref()).await?;

    let mut entries = fs::read_dir(source).await?;

    while let Some(entry) = entries.next_entry().await? {
        let file_destination = destination.as_ref().join(entry.file_name());

        if entry.file_type().await?.is_dir() {
            copy_dir_all(entry.path(), file_destination).await?;
        } else {
            fs::copy(entry.path(), file_destination).await?;
        }
    }

    Ok(())
}

pub(crate) fn remove_dir_on_drop(path: String, base_dir: &str) -> DropGuard<String, impl FnOnce(String) + use<>> {
    let trash_dir = format!("{base_dir}/.deleted");
    let trash_path = format!("{trash_dir}/{}", Uuid::now_v7());

    DropGuard::new(path, move |path| {
        if let Err(err) = create_dir_all(&trash_dir).and_then(|()| rename(&path, &trash_path)) {
            if err.kind() != ErrorKind::NotFound {
                error!(?err, %path, %trash_path, "failed to move directory to trash");
            }

            return;
        }

        spawn(async move {
            let Some(queue) = GLOBAL_QUEUE.get() else {
                error!(%trash_path, "cleanup task should only be scheduled after queue has been initialized");
                return;
            };

            let task = DeletedRepoCleanupTask {
                trash_path: Some(trash_path),
                s3_keys: Vec::new(),
            };

            if let Err(err) = queue.insert_task(&task as &dyn AsyncRunnable).await {
                error!(?err, trash_path = ?task.trash_path, "failed to schedule cleanup task");
            }
        });
    })
}
