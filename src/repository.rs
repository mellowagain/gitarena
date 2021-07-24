use sqlx::{FromRow, Transaction, Postgres};
use anyhow::Result;
use crate::user::User;

#[derive(FromRow)]
pub(crate) struct Repository {
    pub(crate) id: i32,
    pub(crate) owner: i32,
    pub(crate) name: String, // Ascii only
    pub(crate) description: String,
}

impl Repository {
    pub(crate) async fn get_owner(&self, transaction: &mut Transaction<'_, Postgres>) -> Result<User> {
        Ok(sqlx::query_as::<_, User>("select * from users where id = $1 limit 1")
            .bind(self.owner)
            .fetch_one(transaction)
            .await?)
    }
}
