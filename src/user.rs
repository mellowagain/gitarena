use anyhow::{anyhow, Context, Result};
use crate::PgPoolConnection;
use sqlx::pool::PoolConnection;
use sqlx::postgres::PgQueryAs;
use sqlx::{Connection, Executor, FromRow, PgConnection, PgPool, Transaction};

#[derive(FromRow)]
pub(crate) struct User {
    pub(crate) id: i32,
    pub(crate) username: String, // Ascii-only
    pub(crate) email: String,
    pub(crate) password: String,
    salt: String,
}

impl User {
    /// Creates a new user object. The user is not yet saved to the database.
    pub(crate) fn new(username: String, email: String, raw_password: String) -> Result<Self> {
        let (password, salt) = crate::crypto::hash_password(raw_password).context("Failed to hash password.")?;

        Ok(User {
            id: -1, // User has not yet been placed in the database.
            username,
            email,
            password,
            salt
        })
    }

    /// Saves this user to the database and populates the user id
    pub(crate) async fn save(&mut self, database_pool: &PgPool) -> Result<()> {
        let connection = database_pool.acquire().await.context("Unable to acquire connection.")?;
        let mut transaction: Transaction<PgPoolConnection> = connection.begin().await.context("Unable to start transaction.")?;

        transaction.execute(
            sqlx::query("insert into users (username, email, password, salt) values ($1, $2, $3, $4);")
                .bind(&self.username)
                .bind(&self.email)
                .bind(&self.password)
                .bind(&self.salt)
        ).await.context("Failed to insert user into database.")?;

        let (id,): (i64,) = sqlx::query_as("select currval(pg_get_serial_sequence('users', 'id'));")
            .fetch_one(&mut transaction)
            .await
            .context("Failed to acquire user id.")?;

        if id > std::i32::MAX as i64 {
            return Err(anyhow!("Returned user id ({}) does not fit into i32.", id));
        }

        self.id = id as i32;

        transaction.commit().await?;

        Ok(())
    }
}
