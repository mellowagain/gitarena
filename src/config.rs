use crate::error::{ErrorHolder, HoldsError};

use std::convert::{Infallible, TryFrom, TryInto};
use std::fmt::Debug;
use std::future::Future;
use std::process::exit;
use std::result::Result as StdResult;
use std::str::FromStr;

use anyhow::{anyhow, bail, Context, Result};
use derive_more::Display;
use log::info;
use serde::{Deserialize, Serialize};
use sqlx::encode::Encode;
use sqlx::postgres::PgDatabaseError;
use sqlx::{Executor, FromRow, Pool, Postgres, Type};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_unwrap::OptionExt;

/// Gets the value of a setting from the database.
///
/// If unset, returns None.
/// If the setting does not match provided type, returns Anyhow Err.
/// If the setting does not exist, returns SQL Err.
///
/// The later case should never happen if the programmer added their setting to schema.sql
pub(crate) async fn get_optional_setting<'e, T, E>(key: &'static str, executor: E) -> Result<Option<T>>
    where T: TryFrom<Setting> + Send,
          E: Executor<'e, Database = Postgres>,
          <T as TryFrom<Setting>>::Error: HoldsError + Send + Sync + 'static
{
    let setting = sqlx::query_as::<_, Setting>("select * from settings where key = $1 limit 1")
        .bind(key)
        .fetch_one(executor)
        .await
        .with_context(|| format!("Unable to read setting {} from database", key))?;

    if setting.is_set() {
        let result: T = setting.try_into().map_err(|err: T::Error| err.into_inner())?;
        Ok(Some(result))
    } else {
        Ok(None)
    }
}

/// Gets the value of a setting from the database.
///
/// If unset, returns GA Err.
/// If the setting does not match provided type, returns Anyhow Err.
/// If the setting does not exist, returns SQL Err.
///
/// The later case should never happen if the programmer added their setting to schema.sql
pub(crate) async fn get_setting<'e, T, E>(key: &'static str, executor: E) -> Result<T>
    where T: TryFrom<Setting> + Send,
          E: Executor<'e, Database = Postgres>,
          <T as TryFrom<Setting>>::Error: HoldsError + Send + Sync + 'static
{
    let setting = sqlx::query_as::<_, Setting>("select * from settings where key = $1 limit 1")
        .bind(key)
        .fetch_one(executor)
        .await
        .with_context(|| format!("Unable to read setting {} from database", key))?;

    let result: T = setting.try_into().map_err(|err: T::Error| err.into_inner())?;
    Ok(result)
}

pub(crate) async fn get_all_settings<'e, E: Executor<'e, Database = Postgres>>(executor: E) -> Result<Vec<Setting>> {
    Ok(sqlx::query_as::<_, Setting>("select * from settings order by key").fetch_all(executor).await?)
}

// This function returns impl Future instead of relying on async fn to automatically convert it into doing just that
// Because async fn tries to unify lifetimes, we need to do this. More info: https://stackoverflow.com/a/68733302
pub(crate) fn set_setting<'e, 'q, T, E>(key: &'static str, value: T, executor: E) -> impl Future<Output = Result<()>> + 'q
    where T: TryFrom<Setting> + Encode<'q, Postgres> + Type<Postgres> + Send + 'q,
          E: Executor<'e, Database = Postgres> + 'q
{
    async move {
        sqlx::query("update settings set value = $1 where key = $2")
            .bind(value)
            .bind(key)
            .execute(executor)
            .await?;

        Ok(())
    }
}

pub(crate) async fn init(db_pool: &Pool<Postgres>, log_guard: WorkerGuard) -> Result<WorkerGuard> {
    let mut transaction = db_pool.begin().await?;

    if let Some(err) = sqlx::query("select exists(select 1 from settings limit 1)").execute(&mut transaction).await.err() {
        if let Some(db_err) = err.as_database_error() {
            let pg_err = db_err.downcast_ref::<PgDatabaseError>();

            // 42P01: relation settings does not exist
            // If we receive this error code we know the tables have not yet been generated,
            // so we insert our schema and if that succeeds we're ready to go
            if pg_err.code() == "42P01" {
                transaction.commit().await?;

                info!("Required database tables do not exist. Creating...");

                create_tables(db_pool, log_guard).await?;
            }
        }

        bail!(err);
    }

    Ok(log_guard)
}

// TODO: Use sqlx migrations
pub(crate) async fn create_tables(db_pool: &Pool<Postgres>, log_guard: WorkerGuard) -> Result<Infallible> {
    const DATABASE_INIT_DATA: &str = include_str!("../schema.sql");
    let mut connection = db_pool.acquire().await?;

    connection.execute(DATABASE_INIT_DATA)
        .await
        .context("Failed to create initial database setup")?;

    info!("Successfully created initial database tables");
    info!("Please change the config values in the `settings` table and restart GitArena");

    drop(connection); // Drop connection so when we close the pool below it doesn't hang
    db_pool.close().await; // Close the pool so the database flushes
    drop(log_guard); // Drop the log guard so the log file gets flushed

    // We had to drop everything above manually as exit below does not call destructors
    exit(0);
}

#[derive(FromRow, Debug, Deserialize, Serialize, Display)]
#[display(fmt = "{}", key)]
pub(crate) struct Setting {
    pub(crate) key: String,
    pub(crate) value: Option<String>,
    #[sqlx(rename = "type")]
    pub(crate) type_constraint: TypeConstraint
}

impl Setting {
    pub(crate) fn is_set(&self) -> bool {
        self.value.is_some()
    }

    pub(crate) fn is_unset(&self) -> bool {
        self.value.is_none()
    }

    pub(crate) fn as_bytes(&self) -> Option<&[u8]> {
        self.value.as_ref().map(String::as_bytes)
    }

    /// Panics if value is none. For safe option, see [as_bytes](as_bytes)
    pub(crate) fn as_bytes_unchecked(&self) -> &[u8] {
        self.as_bytes().unwrap_or_log()
    }
}

#[macro_export]
macro_rules! generate_try_from {
    ($type_constraint:ident, $type_:ty) => {
        impl TryFrom<Setting> for $type_ {
            type Error = ErrorHolder;

            fn try_from(setting: Setting) -> StdResult<$type_, Self::Error> {
                (|| match setting.type_constraint {
                    TypeConstraint::$type_constraint => {
                        let str = setting.value.as_ref().ok_or_else(|| anyhow!("Value for {} setting `{}` is not set", stringify!($type_constraint), setting))?;
                        <$type_>::from_str(str).map_err(|err| anyhow!("Expected valid value for {} on setting `{}` but instead received `{:?}`: {}", stringify!($type_constraint), setting.key.as_str(), setting.value, err))
                    },
                    _ => bail!("Tried to cast setting `{}` into {} despite it being {}", setting.key.as_str(), stringify!($type_constraint), setting.type_constraint)
                })().map_err(|err| ErrorHolder(err))
            }
        }
    }
}

impl TryFrom<Setting> for bool {
    type Error = ErrorHolder;

    fn try_from(setting: Setting) -> StdResult<bool, Self::Error> {
        (|| match setting.type_constraint {
            TypeConstraint::Boolean => {
                let str = setting.value.ok_or_else(|| anyhow!("Value for Boolean setting `{}` is not set", setting.key.as_str()))?;

                match str.to_lowercase().as_str() {
                    "1" | "true" => Ok(true),
                    "0" | "false" => Ok(false),
                    _ => bail!("Expected valid value for boolean on setting `{}` but instead received `{}`", setting.key.as_str(), str.as_str())
                }
            }
            _ => bail!("Tried to cast setting `{}` into boolean despite it being {}", setting.key.as_str(), setting.type_constraint)
        })().map_err(ErrorHolder)
    }
}

impl TryFrom<Setting> for String {
    type Error = ErrorHolder;

    fn try_from(setting: Setting) -> StdResult<Self, Self::Error> {
        (|| match setting.type_constraint {
            TypeConstraint::String => Ok(setting.value.ok_or_else(|| anyhow!("Value for String setting `{}` is not set", setting.key.as_str()))?),
            _ => bail!("Tried to cast setting `{}` into string despite it being {}", setting.key.as_str(), setting.type_constraint)
        })().map_err(ErrorHolder)
    }
}

generate_try_from!(Char, char);
generate_try_from!(Int, i32);
generate_try_from!(Int, i64);

#[derive(Type, Display, Debug, Ord, PartialOrd, Eq, PartialEq, Deserialize, Serialize)]
#[sqlx(rename = "type_constraint", rename_all = "lowercase")]
#[serde(rename_all(serialize = "lowercase", deserialize = "lowercase"))]
pub(crate) enum TypeConstraint {
    Boolean,    // bool, bool
    Char,       // i8, char
    Int,        // i32/i64, int/bigint
    String,     // &str, varchar, char, text
    Bytes       // &[u8], bytea // TODO: Implement Bytes when needed
}
