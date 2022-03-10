use crate::prelude::ContextExtensions;
use crate::sse::{Broadcaster, Category};
use crate::user::WebUser;
use crate::{die, render_template};

use std::collections::HashMap;
use std::fs;

use actix_web::http::header::CONTENT_TYPE;
use actix_web::web::Data;
use actix_web::{HttpResponse, Responder};
use anyhow::Result;
use chrono::Local;
use futures_locks::RwLock;
use gitarena_macros::route;
use once_cell::sync::Lazy;
use serde::Deserialize;
use serde_json::Value;
use tera::Context;

#[route("/log", method = "GET", err = "html")]
pub(crate) async fn log(web_user: WebUser) -> Result<impl Responder> {
    let user = web_user.into_user()?;

    if !user.admin {
        die!(FORBIDDEN, "Not allowed");
    }

    static LOG_FILE: Lazy<String> = Lazy::new(get_log_file_path);

    let lines = fs::read_to_string(LOG_FILE.as_str()).map(|content| {
        let index = content.rfind("Successfully loaded 415 licenses from cache").map_or_else(|| 0, |i| i - 72);
        let new_log_file = &content[index..];

        let lines = new_log_file.lines();
        let mut log_lines = Vec::with_capacity(lines.size_hint().0);

        for line in lines {
            if let Ok(log_line) = serde_json::from_str::<LogLine>(line) {
                if let Some(message) = log_line.fields.get("message") {
                    if let Value::String(message) = message {
                        log_lines.push(format!("{} [{}] {}", log_line.timestamp, log_line.level, message));
                    }
                }
            }
        }

        log_lines
    }).unwrap_or_default();

    let mut context = Context::new();

    context.insert_user(&user)?;
    context.try_insert("lines", &lines)?;

    render_template!("admin/log.html", context)
}

#[route("/log/sse", method = "GET", err = "html")]
pub(crate) async fn log_sse(web_user: WebUser, broadcaster: Data<RwLock<Broadcaster>>) -> Result<impl Responder> {
    let user = web_user.into_user()?;

    if !user.admin {
        die!(FORBIDDEN, "Not allowed");
    }

    let tx = broadcaster.write().await.new_client(Category::AdminLog).await?;

    Ok(HttpResponse::Ok()
        .insert_header((CONTENT_TYPE, "text/event-stream"))
        .streaming(tx))
}

#[derive(Deserialize)]
struct LogLine<'a> {
    timestamp: &'a str,
    level: &'a str,
    #[serde(borrow = "'a")]
    fields: HashMap<&'a str, Value>
}

fn get_log_file_path() -> String {
    format!("logs/gitarena.log.{}", Local::now().format("%Y-%m-%d"))
}
