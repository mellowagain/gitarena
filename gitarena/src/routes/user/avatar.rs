use crate::database::Pool;
use crate::storage::Storage;
use crate::user::{User, WebUser};
use crate::{die, err};
use actix_web::http::header::{CACHE_CONTROL, CONTENT_TYPE, ETAG};
use actix_web::{HttpResponse, Responder, web};
use anyhow::{Context, Result, bail};
use gitarena_macros::route;
use http::Method;
use object_store::ObjectStoreExt;
use object_store::signer::Signer;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use utoipa::ToSchema;
use uuid::Uuid;

#[utoipa::path(
    get,
    path = "/api/avatar/{user_id}",
    params(("user_id" = Uuid, Path, description = "User ID")),
    responses(
        (status = 200, description = "User avatar", content_type = "image/webp"),
        (status = 404, description = "User or avatar not found"),
        (status = 503, description = "Object storage unavailable"),
    ),
    tag = "user"
)]
#[route("/api/avatar/{user_id}", method = "GET", err = "text")]
pub(crate) async fn get_avatar(request: web::Path<AvatarRequest>, storage: web::Data<Storage>, db_pool: web::Data<Pool>) -> Result<impl Responder> {
    let Some(store) = storage.as_ref().as_ref() else {
        die!(SERVICE_UNAVAILABLE, "Object storage is not available");
    };

    let mut tx = db_pool.begin().await?;

    let user = User::find_using_id(request.user_id, &mut tx)
        .await
        .ok_or_else(|| err!(NOT_FOUND, "user not found"))?;

    tx.commit().await?;

    Ok(match store.get(&user.avatar_s3_key()).await {
        Ok(result) => {
            let mut response = HttpResponse::Ok();
            response
                .append_header((CONTENT_TYPE, "image/webp"))
                .append_header((CACHE_CONTROL, "public, max-age=604800"));

            if let Some(e_tag) = &result.meta.e_tag {
                response.append_header((ETAG, e_tag.as_str()));
            }

            response.streaming(result.into_stream())
        }
        Err(object_store::Error::NotFound { .. }) => die!(NOT_FOUND, "user did not upload avatar"),
        Err(err) => bail!(err),
    })
}

#[utoipa::path(
    post,
    path = "/api/avatar",
    responses(
        (status = 200, description = "Avatar upload URL", body = UploadAvatarResponse),
        (status = 401, description = "Authentication required"),
        (status = 403, description = "User disabled"),
        (status = 503, description = "Object storage unavailable"),
    ),
    security(("cookieAuth" = [])),
    tag = "user"
)]
#[route("/api/avatar", method = "POST", err = "json")]
pub(crate) async fn put_avatar(web_user: WebUser, storage: web::Data<Storage>) -> Result<impl Responder> {
    let Some(store) = storage.as_ref().as_ref() else {
        die!(SERVICE_UNAVAILABLE, "Object storage is not available");
    };

    let user = web_user.into_user()?;

    if user.disabled {
        die!(FORBIDDEN, "User is disabled");
    }

    let upload_url = store
        .signed_url(Method::PUT, &user.avatar_s3_key(), Duration::from_mins(15))
        .await
        .context("failed to generate upload url")?;

    Ok(HttpResponse::Ok().json(UploadAvatarResponse {
        upload_url: upload_url.to_string(),
    }))
}

#[utoipa::path(
    delete,
    path = "/api/avatar",
    responses(
        (status = 204, description = "Avatar deleted"),
        (status = 401, description = "Authentication required"),
        (status = 403, description = "User disabled"),
        (status = 503, description = "Object storage unavailable"),
    ),
    security(("cookieAuth" = [])),
    tag = "user"
)]
#[route("/api/avatar", method = "DELETE", err = "json")]
pub(crate) async fn delete_avatar(web_user: WebUser, storage: web::Data<Storage>) -> Result<impl Responder> {
    let Some(store) = storage.as_ref().as_ref() else {
        die!(SERVICE_UNAVAILABLE, "Object storage is not available");
    };

    let user = web_user.into_user()?;

    if user.disabled {
        die!(FORBIDDEN, "User is disabled");
    }

    store.delete(&user.avatar_s3_key()).await?;
    Ok(HttpResponse::NoContent().finish())
}

#[derive(Deserialize)]
pub(crate) struct AvatarRequest {
    /// User ID
    user_id: Uuid,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UploadAvatarResponse {
    /// S3 pre-signed URL to upload avatar to
    upload_url: String,
}
