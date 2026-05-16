use crate::error::{ErrorHolder, HoldsError};

use std::convert::{TryFrom, TryInto};
use std::fmt::Debug;
use std::future::Future;
use std::result::Result as StdResult;
use std::str::FromStr;

use crate::database::Database;
use anyhow::{Context, Result, anyhow, bail};
use derive_more::Display;
use serde::{Deserialize, Serialize};
use sqlx::encode::Encode;
use sqlx::{FromRow, Postgres, Transaction, Type};
use tracing::instrument;
use tracing_unwrap::OptionExt;

/// Gets the value of a setting from the database.
///
/// If unset, returns None.
/// If the setting does not match provided type, returns Anyhow Err.
/// If the setting does not exist, returns SQL Err.
///
/// The later case should never happen if the programmer added their setting to schema.sql
#[instrument(err, skip(tx))]
pub(crate) async fn get_optional_setting<T>(key: &'static str, tx: &mut Transaction<'_, Database>) -> Result<Option<T>>
where
    T: TryFrom<Setting> + Send,
    <T as TryFrom<Setting>>::Error: HoldsError + Send + Sync + 'static,
{
    let setting = sqlx::query_as::<_, Setting>("select * from settings where key = $1 limit 1")
        .bind(key)
        .fetch_one(&mut **tx)
        .await
        .with_context(|| format!("Unable to read setting {key} from database"))?;

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
#[instrument(err, skip(tx))]
pub(crate) async fn get_setting<T>(key: &'static str, tx: &mut Transaction<'_, Database>) -> Result<T>
where
    T: TryFrom<Setting> + Send,
    <T as TryFrom<Setting>>::Error: HoldsError + Send + Sync + 'static,
{
    let setting = sqlx::query_as::<_, Setting>("select * from settings where key = $1 limit 1")
        .bind(key)
        .fetch_one(&mut **tx)
        .await
        .with_context(|| format!("Unable to read setting {key} from database"))?;

    let result: T = setting.try_into().map_err(|err: T::Error| err.into_inner())?;
    Ok(result)
}

#[instrument(err, skip(tx))]
pub(crate) async fn get_all_settings(tx: &mut Transaction<'_, Database>) -> Result<Vec<Setting>> {
    Ok(sqlx::query_as::<_, Setting>("select * from settings order by key").fetch_all(&mut **tx).await?)
}

// This function returns impl Future instead of relying on async fn to automatically convert it into doing just that
// Because async fn tries to unify lifetimes, we need to do this. More info: https://stackoverflow.com/a/68733302
#[instrument(err, skip(tx))]
#[allow(clippy::manual_async_fn)]
pub(crate) fn set_setting<'q, 'tx, T>(
    key: &'static str,
    value: T,
    tx: &'q mut Transaction<'tx, Database>,
) -> impl Future<Output = Result<()>> + 'q + use<'q, 'tx, T>
where
    T: TryFrom<Setting> + Encode<'q, Postgres> + Type<Postgres> + Debug + Send + 'q,
{
    async move {
        sqlx::query("update settings set value = $1 where key = $2")
            .bind(value)
            .bind(key)
            .execute(&mut **tx)
            .await?;

        Ok(())
    }
}

#[derive(FromRow, Debug, Deserialize, Serialize, Display)]
#[display("{key}")]
pub(crate) struct Setting {
    pub(crate) key: String,
    pub(crate) value: Option<String>,
    #[sqlx(rename = "type")]
    pub(crate) type_constraint: TypeConstraint,
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
                        let str = setting
                            .value
                            .as_ref()
                            .ok_or_else(|| anyhow!("Value for {} setting `{}` is not set", stringify!($type_constraint), setting))?;
                        <$type_>::from_str(str).map_err(|err| {
                            anyhow!(
                                "Expected valid value for {} on setting `{}` but instead received `{:?}`: {}",
                                stringify!($type_constraint),
                                setting.key.as_str(),
                                setting.value,
                                err
                            )
                        })
                    }
                    _ => bail!(
                        "Tried to cast setting `{}` into {} despite it being {}",
                        setting.key.as_str(),
                        stringify!($type_constraint),
                        setting.type_constraint
                    ),
                })()
                .map_err(|err| ErrorHolder(err))
            }
        }
    };
}

impl TryFrom<Setting> for bool {
    type Error = ErrorHolder;

    fn try_from(setting: Setting) -> StdResult<bool, Self::Error> {
        (|| match setting.type_constraint {
            TypeConstraint::Boolean => {
                let str = setting
                    .value
                    .ok_or_else(|| anyhow!("Value for Boolean setting `{}` is not set", setting.key.as_str()))?;

                match str.to_lowercase().as_str() {
                    "1" | "true" => Ok(true),
                    "0" | "false" => Ok(false),
                    _ => bail!(
                        "Expected valid value for boolean on setting `{}` but instead received `{}`",
                        setting.key.as_str(),
                        str.as_str()
                    ),
                }
            }
            _ => bail!(
                "Tried to cast setting `{}` into boolean despite it being {}",
                setting.key.as_str(),
                setting.type_constraint
            ),
        })()
        .map_err(ErrorHolder)
    }
}

impl TryFrom<Setting> for String {
    type Error = ErrorHolder;

    fn try_from(setting: Setting) -> StdResult<Self, Self::Error> {
        (|| match setting.type_constraint {
            TypeConstraint::String => Ok(setting
                .value
                .ok_or_else(|| anyhow!("Value for String setting `{}` is not set", setting.key.as_str()))?),
            _ => bail!(
                "Tried to cast setting `{}` into string despite it being {}",
                setting.key.as_str(),
                setting.type_constraint
            ),
        })()
        .map_err(ErrorHolder)
    }
}

generate_try_from!(Char, char);
generate_try_from!(Int, i32);
generate_try_from!(Int, i64);

#[derive(Type, Display, Debug, Ord, PartialOrd, Eq, PartialEq, Deserialize, Serialize)]
#[sqlx(type_name = "type_constraint", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub(crate) enum TypeConstraint {
    Boolean, // bool, bool
    Char,    // i8, char
    Int,     // i32/i64, int/bigint
    String,  // &str, varchar, char, text
    Bytes,   // &[u8], bytea // TODO: Implement Bytes when needed
}
