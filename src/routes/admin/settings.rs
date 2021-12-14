use crate::config::{Setting, TypeConstraint};
use crate::error::GAErrors::HttpError;
use crate::prelude::HttpRequestExtensions;
use crate::user::WebUser;
use crate::{config, render_template};

use std::collections::HashMap;
use std::sync::Once;

use actix_web::{HttpRequest, HttpResponse, Responder, web};
use anyhow::{Context as _, Result};
use gitarena_macros::route;
use multimap::MultiMap;
use sqlx::PgPool;
use tera::Context;

#[route("/settings", method = "GET")]
pub(crate) async fn get_settings(web_user: WebUser, db_pool: web::Data<PgPool>) -> Result<impl Responder> {
    let user = web_user.into_user()?;

    if !user.admin {
        return Err(HttpError(403, "Not allowed".to_owned()).into());
    }

    let mut context = Context::new();

    context.try_insert("user", &user)?;

    let mut transaction = db_pool.begin().await?;

    let settings = config::get_all_settings(&mut transaction)
        .await
        .context("Failed to fetch settings")?
        .into_iter()
        .map(|setting| {
            let key = setting.key.as_str();
            let parent_key = key.split_once('.').map_or_else(|| key, |(key, _)| key).to_owned();

            (parent_key, setting)
        })
        .collect::<MultiMap<String, Setting>>();

    context.try_insert("settings", &settings)?;

    render_template!("admin/settings.html", context, transaction)
}

#[route("/settings", method = "PATCH")]
pub(crate) async fn patch_settings(data: web::Form<HashMap<String, String>>, web_user: WebUser, request: HttpRequest, db_pool: web::Data<PgPool>) -> Result<impl Responder> {
    let user = web_user.into_user()?;

    if !user.admin {
        return Err(HttpError(403, "Not allowed".to_owned()).into());
    }

    let mut transaction = db_pool.begin().await?;
    let once = Once::new();

    for (key, value) in data.iter() {
        let setting = sqlx::query_as::<_, Setting>("select * from settings where key = $1 limit 1")
            .bind(key.as_str())
            .fetch_one(&mut transaction)
            .await
            .map_err(|_| HttpError(400, "Setting not found".to_owned()))?;

        let valid = match setting.type_constraint {
            TypeConstraint::Boolean => value.parse::<bool>().is_ok(),
            TypeConstraint::Char => value.parse::<char>().is_ok(),
            TypeConstraint::Int => value.parse::<i32>().is_ok(),
            TypeConstraint::String | TypeConstraint::Bytes => true
        };

        if !valid {
            return Err(HttpError(400, format!("Value for {} does not follow type constraint", key)).into());
        }

        // This does on purpose not use config::set_setting as that method requires a key: &'static str
        // aka it is meant to only be used within the program itself with known, safe values
        sqlx::query("update settings set value = $1 where key = $2")
            .bind(value)
            .bind(key)
            .execute(&mut transaction)
            .await?;

        once.call_once(|| {});
    }

    // htmx does not set booleans to `false` and does not send a form data for some reason
    // As a workaround detect the triggered element and set it to false
    if !once.is_completed() {
        let setting = match request.get_header("hx-trigger-name") {
            Some(setting) => setting,
            None => return Err(HttpError(400, "Setting not found".to_owned()).into())
        };

        sqlx::query("update settings set value = false where key = $1")
            .bind(setting)
            .execute(&mut transaction)
            .await?;

        once.call_once(|| {});
    }

    transaction.commit().await?;

    if once.is_completed() {
        Ok(HttpResponse::NoContent().finish())
    } else {
        Err(HttpError(400, "No data provided".to_owned()).into())
    }
}
