use crate::user::User;

use actix_web::HttpRequest;
use log::warn;
use sqlx::{Transaction, Postgres};

pub(crate) fn get_user_agent(request: &HttpRequest) -> Option<&str> {
    request.headers().get("user-agent")?.to_str().ok()
}

pub(crate) async fn get_user_by_identity(identity: Option<String>, transaction: &mut Transaction<'_, Postgres>) -> Option<User> {
    match identity {
        Some(id_str) => {
            let mut split = id_str.splitn(1, '$');
            let id = split.next().unwrap_or_else(|| {
                warn!("Unable to parse identity string `{}`", id_str);
                "unknown"
            });
            let session = split.next().unwrap_or_else(|| {
                warn!("Unable to parse identity string `{}`", id_str);
                "unknown"
            });

            sqlx::query_as::<_, User>("select * from users where id = $1 and session = $2 limit 1")
                .bind(id)
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
        legal &= input != format!("COM{}", i);
        legal &= input != format!("LPT{}", i);
    }

    legal
}
