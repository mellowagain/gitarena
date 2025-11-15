use std::path::Path;

use anyhow::Result;
use async_recursion::async_recursion;
use tokio::fs;

// Adopted from https://stackoverflow.com/a/65192210 (turned it async using tokio::fs)
#[async_recursion(?Send)]
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
