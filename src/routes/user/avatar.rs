use crate::mail::Email;
use crate::prelude::{AwcExtensions, HttpRequestExtensions};
use crate::user::WebUser;
use crate::{die, err};

use std::fs;
use std::io::Cursor;
use std::path::Path;
use std::time::SystemTime;

use actix_multipart::Multipart;
use actix_web::http::header::{CACHE_CONTROL, LAST_MODIFIED};
use actix_web::{HttpRequest, HttpResponse, Responder, web};
use anyhow::{Context, Result};
use awc::Client;
use awc::http::header::IF_MODIFIED_SINCE;
use chrono::{Duration, NaiveDateTime};
use futures::TryStreamExt;
use gitarena_macros::{from_config, route};
use image::ImageFormat;
use serde::Deserialize;
use sqlx::PgPool;

#[route("/api/avatar/{user_id}", method = "GET", err = "text")]
pub(crate) async fn get_avatar(avatar_request: web::Path<AvatarRequest>, request: HttpRequest, db_pool: web::Data<PgPool>) -> Result<impl Responder> {
    let (gravatar_enabled, avatars_dir): (bool, String) = from_config!(
        "avatars.gravatar" => bool,
        "avatars.dir" => String
    );

    let query_string = request.q_string();

    if !query_string.has("override") {
        let path_str = format!("{}/{}.jpg", avatars_dir, avatar_request.user_id);
        let path = Path::new(path_str.as_str());

        // User has set an avatar, return it
        if path.is_file() {
            return Ok(send_image(path, &request).await.context("Failed to read local image file")?);
        }
    }

    // User has not set an avatar, so if Gravatar integration is enabled return it
    if gravatar_enabled {
        let mut transaction = db_pool.begin().await?;

        let email = if let Some(email) = query_string.get("override") {
            email.to_owned()
        } else {
            Email::find_primary_email(avatar_request.user_id, &mut transaction)
                .await?
                .ok_or_else(|| err!(NOT_FOUND, "User not found"))?
                .email
        };

        return Ok(send_gravatar(email.as_str(), &request).await.context("Failed to request Gravatar image")?);
    }

    // Gravatar integration is not enabled, return fallback icon
    // TODO: Maybe generate own identicons? -> There are crates for this

    let path_str = format!("{}/default.jpg", avatars_dir);
    let path = Path::new(path_str.as_str());

    Ok(send_image(path, &request).await.context("Failed to read default avatar file")?)
}

#[route("/api/avatar", method = "PUT", err = "text")]
pub(crate) async fn put_avatar(web_user: WebUser, mut payload: Multipart, db_pool: web::Data<PgPool>) -> Result<impl Responder> {
    if matches!(web_user, WebUser::Anonymous) {
        die!(UNAUTHORIZED, "No logged in");
    }

    let user = web_user.into_user()?;

    if user.disabled {
        die!(FORBIDDEN, "User is disabled");
    }

    let avatars_dir: String = from_config!("avatars.dir" => String);

    let mut field = match payload.try_next().await {
        Ok(Some(field)) => field,
        Ok(None) => die!(BAD_REQUEST, "No multipart field found"),
        Err(err) => return Err(err.into())
    };

    let content_disposition = field.content_disposition();
    let file_name = content_disposition.get_filename().ok_or_else(|| err!(BAD_REQUEST, "No file name"))?;
    let extension = file_name.rsplit_once('.')
        .map(|(_, ext)| ext.to_owned())
        .ok_or_else(|| err!(BAD_REQUEST, "Invalid file name"))?;

    let mut bytes = web::BytesMut::new();

    while let Some(chunk) = field.try_next().await.context("Failed to read multipart data chunk")? {
        bytes.extend_from_slice(chunk.as_ref());
    }

    let frozen_bytes = bytes.freeze();

    web::block(move || -> Result<()> {
        let format = ImageFormat::from_extension(extension).ok_or_else(|| err!(BAD_REQUEST, "Unsupported image format"))?;

        let mut cursor = Cursor::new(frozen_bytes.as_ref());

        let mut img = image::load(&mut cursor, format)?;
        img = img.thumbnail_exact(500, 500); // TODO: Check whenever this removes metadata such as location (If not remove metadata)

        let path_str = format!("{}/{}.jpg", avatars_dir, user.id);
        let path = Path::new(path_str.as_str());

        img.save_with_format(path, ImageFormat::Jpeg)?;

        Ok(())
    }).await.context("Failed to save image")?.context("Failed to save image")?;

    Ok(HttpResponse::Created().finish())
}

async fn send_image<P: AsRef<Path>>(path: P, request: &HttpRequest) -> Result<HttpResponse> {
    let path = path.as_ref();

    let mut response = HttpResponse::Ok();
    response.content_type("image/jpeg");

    let meta_data = fs::metadata(path)?;

    if let Ok(modified_system_time) = meta_data.modified() {
        let modified_unix_time = modified_system_time.duration_since(SystemTime::UNIX_EPOCH)?;
        let naive_date_time = NaiveDateTime::from_timestamp(modified_unix_time.as_secs() as i64, modified_unix_time.subsec_nanos());

        // TODO: Convert time zone from local machine to GMT properly
        let format = naive_date_time.format("%a, %d %b %Y %H:%M:%S GMT").to_string();

        if let Some(if_modified_since) = request.get_header("if-modified-since") {
            let request_date_time = NaiveDateTime::parse_from_str(if_modified_since, "%a, %d %b %Y %H:%M:%S %Z")?;

            let duration = naive_date_time.signed_duration_since(request_date_time);

            // Image is still OK on client side cache
            if duration > Duration::seconds(0) {
                return Ok(HttpResponse::NotModified().append_header((LAST_MODIFIED, format)).finish());
            }
        }

        response.append_header((LAST_MODIFIED, format));
    }

    let file_content = fs::read(path)?;

    Ok(response.body(file_content))
}

/// Returns a streaming HttpResponse with the gravatar image
async fn send_gravatar(email: &str, request: &HttpRequest) -> Result<HttpResponse> {
    let md5hash = md5::compute(email);

    let url = format!("https://www.gravatar.com/avatar/{:x}?s=500&r=pg&d=identicon", md5hash);

    let mut client = Client::gitarena().get(url);

    if let Some(header_value) = request.get_header("if-modified-since") {
        client = client.append_header((IF_MODIFIED_SINCE, header_value));
    }

    let gateway_response = client.send().await.map_err(|err| err!(BAD_GATEWAY, "Failed to send request to Gravatar: {}", err))?;
    let mut response = HttpResponse::build(gateway_response.status());

    let headers = gateway_response.headers();

    if let Some(cache_control) = headers.get("cache-control") {
        response.append_header((CACHE_CONTROL, cache_control.to_str()?));
    }

    if let Some(last_modified) = headers.get("last-modified") {
        response.append_header((LAST_MODIFIED, last_modified.to_str()?));
    }

    Ok(response.streaming(gateway_response))
}

#[derive(Deserialize)]
pub(crate) struct AvatarRequest {
    user_id: i32
}
