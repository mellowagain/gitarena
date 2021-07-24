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
