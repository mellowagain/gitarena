use crate::config::CONFIG;
use crate::error::GAErrors::{GitError, HttpError, ParseError};
use crate::repository::Repository;
use crate::user::User;

use core::result::Result as CoreResult;
use std::borrow::Borrow;
use std::io::Result as IoResult;
use std::time::Instant;

use actix_web::HttpRequest;
use anyhow::{Error, Result};
use bstr::{BStr, BString, ByteSlice};
use chrono::Utc;
use futures::Future;
use git2::ObjectType;
use git_hash::ObjectId;
use git_pack::data::entry::Header;
use git_actor::{Sign, Signature as GitoxideSignature, Time};
use git2::Signature as Git2Signature;
use log::warn;
use sqlx::{Executor, Postgres, Transaction};

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

pub(crate) fn bstr_to_str(input: &BStr) -> Result<&str> {
    Ok(input.to_str()?)
}

pub(crate) async fn get_user_by_identity<'e, E: Executor<'e, Database = Postgres>>(identity: Option<String>, executor: E) -> Option<User> {
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
                .fetch_one(executor)
                .await
                .ok()
        }
        None => None
    }
}

// TODO: Make this method take `E: Executor<'e, Database = Postgres>` instead of `Transaction`
pub(crate) async fn repo_from_str<S: AsRef<str>>(username: S, repository: S, mut transaction: Transaction<'_, Postgres>) -> Result<(Repository, Transaction<'_, Postgres>)> {
    let username_str = username.as_ref();
    let repo_str = repository.as_ref();

    let (user_id,): (i32,) = sqlx::query_as("select id from users where lower(username) = lower($1)")
        .bind(username_str)
        .fetch_optional(&mut transaction)
        .await?
        .ok_or(HttpError(404, "Not found".to_owned()))?;

    let repo: Repository = sqlx::query_as::<_, Repository>("select * from repositories where owner = $1 and lower(name) = lower($2)")
        .bind(&user_id)
        .bind(repo_str)
        .fetch_optional(&mut transaction)
        .await?
        .ok_or(HttpError(404, "Not found".to_owned()))?;

    Ok((repo, transaction))
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
pub(crate) fn default_signature() -> GitoxideSignature {
    let domain: &str = CONFIG.domain.borrow();
    let stripped = domain.replace("https://", "").replace("http://", "");

    let now = Utc::now();
    let naive = now.naive_utc();

   GitoxideSignature {
        name: BString::from("GitArena"),
        email: BString::from(format!("git@{}", stripped)),
        time: Time {
            time: naive.timestamp() as u32,
            offset: 0,
            sign: Sign::Plus
        }
    }
}

pub(crate) fn libgit2_to_gitoxide_signature(libgit2_signature: &Git2Signature) -> GitoxideSignature {
    let time = libgit2_signature.when();

    GitoxideSignature {
        name: BString::from(libgit2_signature.name_bytes()),
        email: BString::from(libgit2_signature.email_bytes()),
        time: Time {
            time: time.seconds() as u32,
            offset: time.offset_minutes() * 60,
            sign: if time.sign() == '-' {
                Sign::Minus
            } else {
                Sign::Plus
            }
        }
    }
}

// Returns the time the function took to execute in seconds
pub(crate) async fn time_function<T: Future, F: FnOnce() -> T>(func: F) -> u64 {
    let start = Instant::now();

    func().await;

    start.elapsed().as_secs()
}
