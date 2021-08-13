use crate::config::CONFIG;
use crate::error::GAErrors::{GitError, ParseError};
use crate::user::User;

use core::result::Result as CoreResult;
use std::borrow::Borrow;
use std::fs;
use std::io::Result as IoResult;
use std::path::Path;

use actix_web::HttpRequest;
use anyhow::{Context, Error, Result};
use bstr::BString;
use chrono::Utc;
use git2::ObjectType;
use git_hash::ObjectId;
use git_pack::data::entry::Header;
use git_repository::actor::{Signature, Time, Sign};
use log::warn;
use sqlx::{Transaction, Postgres};

/// Parses "key=value" into a key value tuple
pub(crate) fn parse_key_value(input: &str) -> Result<(&str, &str)> {
    let mut split = input.splitn(2, "=");
    let key = split.next().ok_or(ParseError("key values", input.to_owned()))?;
    let value = split.next().ok_or(ParseError("key values", input.to_owned()))?;

    Ok((key, value))
}

pub(crate) fn get_header<'a>(request: &'a HttpRequest, header: &'a str) -> Option<&'a str> {
    request.headers().get(header)?.to_str().ok()
}

pub(crate) async fn get_user_by_identity(identity: Option<String>, transaction: &mut Transaction<'_, Postgres>) -> Option<User> {
    match identity {
        Some(id_str) => {
            let mut split = id_str.splitn(2, '$');

            let id = split.next().unwrap_or_else(|| {
                warn!("Unable to parse id from identity string `{}`", id_str);
                "-1"
            }).parse::<i32>().unwrap_or(-1);

            let session = split.next().unwrap_or_else(|| {
                warn!("Unable to parse session from identity string `{}`", id_str);
                "unknown"
            });

            sqlx::query_as::<_, User>("select * from users where id = $1 and session = $2 limit 1")
                .bind(&id)
                .bind(session)
                .fetch_one(transaction)
                .await
                .ok()
        }
        None => None
    }
}

/// Checks if the character is alphanumeric (`a-z, 0-9`), a dash (`-`) or a underscore (`_`)
#[inline]
pub(crate) fn is_identifier(c: &char) -> bool {
    c.is_ascii_alphanumeric() || c == &'-' || c == &'_'
}

/// Checks for illegal file and directory names on Windows.
/// This function assumes that the input has already been checked with [`is_identifier`][0].
///
/// [0]: crate::extensions::is_identifier
pub(crate) async fn is_fs_legal(input: &String) -> bool {
    let mut legal = input != "CON";
    legal &= input != "PRN";
    legal &= input != "AUX";
    legal &= input != "NUL";
    legal &= input != "LST";

    for i in 0..=9 {
        legal &= input != &format!("COM{}", i);
        legal &= input != &format!("LPT{}", i);
    }

    legal
}

/// Flattens `std::io::Result<std::result::Result<O, E>>` into `anyhow::Result<O>`
pub(crate) fn flatten_io_result<O, E: Into<Error>>(result: IoResult<CoreResult<O, E>>) -> Result<O> {
    match result {
        Ok(Ok(ok)) => Ok(ok),
        Ok(Err(err)) => Err(err.into()),
        Err(err) => Err(err.into())
    }
}

/// Flattens `std::result::Result<std::result::Result<O, E>, E>` into `anyhow::Result<O>`
pub(crate) fn flatten_result<O, E: Into<Error>>(result: CoreResult<CoreResult<O, E>, E>) -> Result<O> {
    match result {
        Ok(Ok(ok)) => Ok(ok),
        Ok(Err(err)) => Err(err.into()),
        Err(err) => Err(err.into())
    }
}

pub(crate) fn create_dir_if_not_exists(path: &Path) -> Result<()> {
    if !path.is_dir() {
        // Check if path is a file and not a directory
        if path.exists() {
            fs::remove_file(path).context("Unable to delete file")?;
        }

        return fs::create_dir_all(path).context("Unable to create directory");
    }

    Ok(())
}

pub(crate) fn normalize_oid_str(oid_str: Option<String>) -> Option<String> {
    match oid_str.as_deref() {
        Some("0000000000000000000000000000000000000000") => None,
        Some(_) => oid_str,
        None => None,
    }
}

pub(crate) fn str_to_oid(oid_option: &Option<String>) -> Result<ObjectId> {
    Ok(match oid_option {
        Some(oid_str) => {
            ObjectId::from_hex(oid_str.as_bytes())?
        }
        None => ObjectId::null_sha1()
    })
}

pub(crate) fn gitoxide_to_libgit2_type(header: &Header) -> Result<ObjectType> {
    Ok(match header {
        Header::Commit => ObjectType::Commit,
        Header::Tree => ObjectType::Tree,
        Header::Blob => ObjectType::Blob,
        Header::Tag => ObjectType::Tag,
        Header::RefDelta { .. } | Header::OfsDelta { .. } => return Err(GitError(501, Some("Delta objects are not yet implemented".to_owned())).into()),
    })
}

// TODO: Maybe make this configurable using the config file at some point?
pub(crate) fn default_signature() -> Signature {
    let domain: &str = CONFIG.domain.borrow();
    let stripped = domain.replace("https://", "").replace("http://", "");

    let now = Utc::now();
    let naive = now.naive_utc();

    Signature {
        name: BString::from("GitArena"),
        email: BString::from(format!("git@{}", stripped)),
        time: Time {
            time: naive.timestamp() as u32,
            offset: 0,
            sign: Sign::Plus
        }
    }
}

pub(crate) mod traits {
    use git_repository::actor::Signature as MutableSignature;
    use git_repository::actor::immutable::Signature as ImmutableSignature;
    use bstr::BString;

    pub(crate) trait GitoxideSignatureExtension {
        fn to_mut(&self) -> MutableSignature;
    }

    impl GitoxideSignatureExtension for ImmutableSignature<'_> {
        fn to_mut(&self) -> MutableSignature {
            MutableSignature {
                name: BString::from(*&self.name),
                email: BString::from(*&self.email),
                time: *&self.time
            }
        }
    }
}
