use anyhow::Result;
use gitarena_common::database::Database;
use gitarena_common::database::models::KeyType;
use gitarena_common::prelude::*;
use sqlx::Executor;

pub(crate) async fn print_all<'e, E: Executor<'e, Database = Database>>(executor: E) -> Result<()> {
    let keys: Vec<(KeyType, Vec<u8>)> = sqlx::query_as("select algorithm, key from ssh_keys where expires_at is null or expires_at < now()")
        .fetch_all(executor)
        .await?;

    for (key_type, key) in keys {
        let encoded = base64::encode(key.as_slice());

        println!("{} {}", key_type, encoded);
    }

    Ok(())
}
