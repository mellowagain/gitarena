use anyhow::Result;
use futures::TryStreamExt;
use gitarena_common::database::Database;
use gitarena_common::database::models::KeyType;
use gitarena_common::prelude::sqlx::Transaction;
use gitarena_common::prelude::*;
use sqlx::Row;

pub(crate) async fn print_all(tx: &mut Transaction<'_, Database>) -> Result<()> {
    let mut stream = sqlx::query("select algorithm, key, owner from ssh_keys where expires_at is null or expires_at < now()").fetch(&mut **tx);

    while let Some(row) = stream.try_next().await? {
        let algorithm: KeyType = row.try_get("algorithm")?;

        let key: &[u8] = row.try_get("key")?;
        let encoded = base64::encode(key);

        let owner_id: i32 = row.try_get("owner")?;

        println!("no-port-forwarding,no-X11-forwarding,no-agent-forwarding,no-pty {algorithm} {encoded} {owner_id}");
    }

    Ok(())
}
