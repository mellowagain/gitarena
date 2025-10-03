use anyhow::Result;
use futures::TryStreamExt;
use gitarena_common::database::models::KeyType;
use gitarena_common::database::Database;
use gitarena_common::prelude::*;
use sqlx::{Executor, Row};

pub(crate) async fn print_all<'e, E: Executor<'e, Database = Database>>(executor: E) -> Result<()> {
    let mut stream = sqlx::query(
        "select algorithm, key from ssh_keys where expires_at is null or expires_at < now()",
    )
    .fetch(executor);

    while let Some(row) = stream.try_next().await? {
        let algorithm: KeyType = row.try_get("algorithm")?;
        let key: &[u8] = row.try_get("key")?;

        println!("{} {}", algorithm, base64::encode(key));
    }

    Ok(())
}
