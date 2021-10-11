use crate::error::GAErrors::TypeConstraintViolated;
use crate::error::GAErrors;

use core::result::Result as CoreResult;
use std::convert::{TryFrom, TryInto};
use std::error::Error as StdError;
use std::future::Future;
use std::str::FromStr;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use sqlx::encode::Encode;
use sqlx::{Executor, FromRow, Postgres, Type};
use tracing_unwrap::OptionExt;

/// Gets the value of a setting from the database.
///
/// If unset, returns None.
/// If the setting does not match provided type, returns GA Err.
/// If the setting does not exist, returns SQL Err.
///
/// The later case should never happen if the programmer added their setting to schema.sql
pub(crate) async fn get_optional_setting<'e, T, E>(key: &'static str, executor: E) -> Result<Option<T>>
    where T: TryFrom<Setting> + Send,
          E: Executor<'e, Database = Postgres>,
          <T as TryFrom<Setting>>::Error: StdError + Send + Sync + 'static
{
    let setting = sqlx::query_as::<_, Setting>("select * from users where key = $1 limit 1")
        .bind(key)
        .fetch_one(executor)
        .await
        .context("Unable to read setting from database")?;

    if setting.is_set() {
        let result: T = setting.try_into()?;
        Ok(Some(result))
    } else {
        Ok(None)
    }
}

/// Gets the value of a setting from the database.
///
/// If unset, returns GA Err.
/// If the setting does not match provided type, returns GA Err.
/// If the setting does not exist, returns SQL Err.
///
/// The later case should never happen if the programmer added their setting to schema.sql
pub(crate) async fn get_setting<'e, T, E>(key: &'static str, executor: E) -> Result<T>
    where T: TryFrom<Setting> + Send,
          E: Executor<'e, Database = Postgres>,
          <T as TryFrom<Setting>>::Error: StdError + Send + Sync + 'static
{
    let setting = sqlx::query_as::<_, Setting>("select * from users where key = $1 limit 1")
        .bind(key)
        .fetch_one(executor)
        .await
        .context("Unable to read setting from database")?;

    let result: T = setting.try_into()?;
    Ok(result)
}

// This function returns impl Future instead of relying on async fn to automatically convert it into doing just that
// Because async fn tries to unify lifetimes, we need to do this. More info: https://stackoverflow.com/a/68733302
pub(crate) fn set_setting<'e, 'q, T, E>(key: &'static str, value: T, executor: E) -> impl Future<Output = Result<()>> + 'q
    where T: TryFrom<Setting> + Encode<'q, Postgres> + Type<Postgres> + Send + 'q,
          E: Executor<'e, Database = Postgres> + 'q
{
    async move {
        sqlx::query("update settings set $1 = $2")
            .bind(key)
            .bind(value)
            .execute(executor)
            .await?;

        Ok(())
    }
}

pub(crate) async fn init<'e, E: Executor<'e, Database = Postgres>>(executor: E) -> Result<()> {
    match sqlx::query("select exists(select 1 from settings)").execute(executor).await {
        Ok(_) => { /* Database was valid */ },
        Err(err) => bail!(err)
    }

    // On error run create_tables

    Ok(())
}

// TODO: Use sqlx migrations
pub(crate) async fn create_tables<'e, E: Executor<'e, Database = Postgres>>(executor: E) -> Result<()> {
    const DATABASE_INIT_DATA: &str = include_str!("../schema.sql");

    sqlx::query(DATABASE_INIT_DATA)
        .execute(executor)
        .await
        .context("Failed to create initial database setup")?;

    Ok(())
}

#[derive(FromRow, Debug, Deserialize, Serialize)]
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
        self.value.map(|v| v.as_bytes())
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
            type Error = GAErrors;

            fn try_from(setting: Setting) -> CoreResult<$type_, Self::Error> {
                match setting.type_constraint {
                    TypeConstraint::$type_constraint => {
                        let str = setting.value.ok_or_else(|| TypeConstraintViolated("None"))?;
                        <$type_>::from_str(&str).map_err(|_| TypeConstraintViolated("value"))
                    },
                    _ => Err(TypeConstraintViolated(concat!("method: try_from<", stringify!($type_), ">")))
                }
            }
        }
    }
}

impl TryFrom<Setting> for bool {
    type Error = GAErrors;

    fn try_from(setting: Setting) -> CoreResult<bool, Self::Error> {
        match setting.type_constraint {
            TypeConstraint::Boolean => {
                let str = setting.value.ok_or_else(|| TypeConstraintViolated("None"))?;

                match str.to_lowercase().as_str() {
                    "1" | "true" => Ok(true),
                    "0" | "false" => Ok(false),
                    _ => Err(TypeConstraintViolated("value"))
                }
            }
            _ => Err(TypeConstraintViolated("method: try_from<bool>"))
        }
    }
}

impl TryFrom<Setting> for String {
    type Error = GAErrors;

    fn try_from(setting: Setting) -> CoreResult<Self, Self::Error> {
        match setting.type_constraint {
            TypeConstraint::String => Ok(setting.value.ok_or_else(|| TypeConstraintViolated("None"))?),
            _ => Err(TypeConstraintViolated("method: try_from<String>"))
        }
    }
}

generate_try_from!(Char, char);
generate_try_from!(Int, i32);
generate_try_from!(Int, i64);

#[derive(Type, Debug, Ord, PartialOrd, Eq, PartialEq, Deserialize, Serialize)]
#[sqlx(rename_all = "lowercase")]
#[serde(rename_all(serialize = "lowercase", deserialize = "lowercase"))]
pub(crate) enum TypeConstraint {
    Boolean,    // bool, bool
    Char,       // i8, char
    Int,        // i32/i64, int/bigint
    String,     // &str, varchar, char, text
    // TODO: Implement Bytes when needed
    //Bytes     // &[u8], bytea
}
