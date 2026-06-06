use crate::database::Pool;
use crate::mail::get_root_email;
use crate::privileges::privilege;
use crate::release::Release;
use crate::release::assets::{Arch, Kind, Libc, Os, ReleaseAssets};
use crate::repository::Repository;
use crate::storage::Storage;
use crate::user::WebUser;
use crate::{die, err};
use actix_web::http::header::LOCATION;
use actix_web::{HttpResponse, Responder, web};
use anyhow::{Context, Result};
use git2::Signature;
use gitarena_macros::route;
use http::Method;
use object_store::ObjectStoreExt;
use object_store::signer::Signer;
use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::time::Duration;
use tracing::{error, instrument};
use utoipa::ToSchema;
use uuid::Uuid;

#[utoipa::path(
    get,
    path = "/api/repos/{namespace}/{repository}/releases",
    params(
        ("namespace" = String, Path, description = "Repository namespace (user or organization)"),
        ("repository" = String, Path, description = "Repository name"),
    ),
    responses(
        (status = 200, description = "List of releases", body = Vec<ReleaseResponse>),
        (status = 404, description = "Repository not found or access denied"),
    ),
    security((), ("cookieAuth" = [])),
    tag = "repository"
)]
#[route("/api/repos/{namespace}/{repository}/releases", method = "GET", err = "json")]
pub(crate) async fn list_releases(repo: Repository, db_pool: web::Data<Pool>) -> Result<impl Responder> {
    let mut tx = db_pool.begin().await?;

    let rows: Vec<Release> = sqlx::query_as("select * from releases where repo_id = $1 order by id desc")
        .bind(repo.id)
        .fetch_all(&mut *tx)
        .await?;

    let latest_id: Option<Uuid> = sqlx::query_scalar("select id from releases where repo_id = $1 and pre_release = false order by id desc limit 1")
        .bind(repo.id)
        .fetch_optional(&mut *tx)
        .await?;

    let libgit2_repo = repo.libgit2(&mut tx).await?;

    let mut releases = Vec::with_capacity(rows.len());

    for release in rows {
        let (commit, commit_message) = tag_commit_info(&libgit2_repo, &release.tag);
        let assets = release.assets(&mut tx).await?;

        releases.push(ReleaseResponse {
            latest: latest_id == Some(release.id),
            release,
            commit,
            commit_message,
            assets,
        });
    }

    tx.commit().await?;

    Ok(HttpResponse::Ok().json(releases))
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReleaseResponse {
    #[serde(flatten)]
    release: Release,
    latest: bool,
    commit: Option<String>,
    commit_message: Option<String>,
    assets: Vec<ReleaseAssets>,
}

#[utoipa::path(
    get,
    path = "/api/repos/{namespace}/{repository}/releases/latest",
    params(
        ("namespace" = String, Path, description = "Repository namespace (user or organization)"),
        ("repository" = String, Path, description = "Repository name"),
    ),
    responses(
        (status = 200, description = "Latest non-pre-release", body = ReleaseResponse),
        (status = 404, description = "No release found"),
    ),
    security((), ("cookieAuth" = [])),
    tag = "repository"
)]
#[route("/api/repos/{namespace}/{repository}/releases/latest", method = "GET", err = "json")]
pub(crate) async fn latest_release(repo: Repository, db_pool: web::Data<Pool>) -> Result<impl Responder> {
    let mut tx = db_pool.begin().await?;

    let release: Option<Release> = sqlx::query_as("select * from releases where repo_id = $1 and pre_release = false order by id desc limit 1")
        .bind(repo.id)
        .fetch_optional(&mut *tx)
        .await?;

    let Some(release) = release else {
        die!(NOT_FOUND, "No release found");
    };

    let libgit2_repo = repo.libgit2(&mut tx).await?;
    let (commit, commit_message) = tag_commit_info(&libgit2_repo, &release.tag);

    let raw_assets: Vec<ReleaseAssets> = sqlx::query_as("select * from release_assets where release_id = $1 and available = true")
        .bind(release.id)
        .fetch_all(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok(HttpResponse::Ok().json(ReleaseResponse {
        latest: true,
        release,
        commit,
        commit_message,
        assets: raw_assets,
    }))
}

#[utoipa::path(
    get,
    path = "/api/repos/{namespace}/{repository}/releases/{release_id}",
    params(
        ("namespace" = String, Path, description = "Repository namespace (user or organization)"),
        ("repository" = String, Path, description = "Repository name"),
        ("release_id" = Uuid, Path, description = "Release ID"),
    ),
    responses(
        (status = 200, description = "Release details", body = ReleaseResponse),
        (status = 404, description = "Release not found"),
    ),
    security((), ("cookieAuth" = [])),
    tag = "repository"
)]
#[route("/api/repos/{namespace}/{repository}/releases/{release_id}", method = "GET", err = "json")]
pub(crate) async fn get_release(repo: Repository, path: web::Path<(String, String, Uuid)>, db_pool: web::Data<Pool>) -> Result<impl Responder> {
    let (_, _, release_id) = path.into_inner();
    let mut tx = db_pool.begin().await?;

    let release: Option<Release> = sqlx::query_as("select * from releases where id = $1 and repo_id = $2")
        .bind(release_id)
        .bind(repo.id)
        .fetch_optional(&mut *tx)
        .await?;

    let Some(release) = release else {
        die!(NOT_FOUND, "Release not found");
    };

    let latest_id: Option<Uuid> = sqlx::query_scalar("select id from releases where repo_id = $1 and pre_release = false order by id desc limit 1")
        .bind(repo.id)
        .fetch_optional(&mut *tx)
        .await?;

    let libgit2_repo = repo.libgit2(&mut tx).await?;

    let assets = release.assets(&mut tx).await?;

    tx.commit().await?;

    let (commit, commit_message) = tag_commit_info(&libgit2_repo, &release.tag);

    Ok(HttpResponse::Ok().json(ReleaseResponse {
        latest: latest_id == Some(release.id),
        release,
        commit,
        commit_message,
        assets,
    }))
}

#[utoipa::path(
    post,
    path = "/api/repos/{namespace}/{repository}/releases",
    params(
        ("namespace" = String, Path, description = "Repository namespace (user or organization)"),
        ("repository" = String, Path, description = "Repository name"),
    ),
    request_body = CreateReleaseRequest,
    responses(
        (status = 200, description = "Created release", body = ReleaseResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Authentication required"),
        (status = 403, description = "Insufficient permissions"),
    ),
    security(("cookieAuth" = [])),
    tag = "repository"
)]
#[route("/api/repos/{namespace}/{repository}/releases", method = "POST", err = "json")]
pub(crate) async fn create_release(
    repo: Repository,
    web_user: WebUser,
    body: web::Json<CreateReleaseRequest>,
    db_pool: web::Data<Pool>,
) -> Result<impl Responder> {
    let user = web_user.into_user()?;

    if repo.archived_at.is_some() {
        die!(FORBIDDEN, "Repository is archived and read-only");
    }

    let mut tx = db_pool.begin().await?;

    if !privilege::check_push(&repo, Some(&user), &mut tx).await? {
        die!(FORBIDDEN, "Insufficient permissions");
    }

    let libgit2_repo = repo.libgit2(&mut tx).await?;

    if libgit2_repo.find_reference(&format!("refs/tags/{}", body.tag)).is_err() {
        let head = libgit2_repo
            .find_reference(&format!("refs/heads/{}", repo.default_branch))
            .map_err(|_| err!(BAD_REQUEST, "Default branch does not exist"))?;

        let commit = head.peel_to_commit().map_err(|_| err!(BAD_REQUEST, "Repository has no commits"))?;

        let signature = Signature::now("GitArena", &get_root_email(&mut tx).await?).context("failed to create git signature for root mailbox")?;
        let description = body.description.as_deref().unwrap_or(&body.title);

        libgit2_repo
            .tag(&body.tag, commit.as_object(), &signature, description, false)
            .map_err(|_| err!(BAD_REQUEST, "Failed to create git tag"))?;
    }

    let release_id = Uuid::now_v7();
    let pre_release = body.pre_release.unwrap_or(false);

    sqlx::query("insert into releases (id, repo_id, title, description, author, tag, pre_release) values ($1, $2, $3, $4, $5, $6, $7)")
        .bind(release_id)
        .bind(repo.id)
        .bind(&body.title)
        .bind(&body.description)
        .bind(user.id)
        .bind(&body.tag)
        .bind(pre_release)
        .execute(&mut *tx)
        .await?;

    let latest_id: Option<Uuid> = sqlx::query_scalar("select id from releases where repo_id = $1 and pre_release = false order by id desc limit 1")
        .bind(repo.id)
        .fetch_optional(&mut *tx)
        .await?;

    tx.commit().await?;

    let (commit, commit_message) = tag_commit_info(&libgit2_repo, &body.tag);
    let body = body.into_inner();

    Ok(HttpResponse::Ok().json(ReleaseResponse {
        latest: latest_id == Some(release_id),
        release: Release {
            id: release_id,
            repo_id: repo.id,
            title: body.title,
            description: body.description,
            author: user.id,
            tag: body.tag,
            pre_release,
        },
        commit,
        commit_message,
        assets: Vec::new(),
    }))
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateReleaseRequest {
    tag: String,
    title: String,
    description: Option<String>,
    pre_release: Option<bool>,
}

#[utoipa::path(
    patch,
    path = "/api/repos/{namespace}/{repository}/releases/{release_id}",
    params(
        ("namespace" = String, Path, description = "Repository namespace (user or organization)"),
        ("repository" = String, Path, description = "Repository name"),
        ("release_id" = Uuid, Path, description = "Release ID"),
    ),
    request_body = UpdateReleaseRequest,
    responses(
        (status = 200, description = "Updated release", body = ReleaseResponse),
        (status = 401, description = "Authentication required"),
        (status = 403, description = "Insufficient permissions"),
        (status = 404, description = "Release not found"),
    ),
    security(("cookieAuth" = [])),
    tag = "repository"
)]
#[route("/api/repos/{namespace}/{repository}/releases/{release_id}", method = "PATCH", err = "json")]
pub(crate) async fn update_release(
    repo: Repository,
    web_user: WebUser,
    path: web::Path<(String, String, Uuid)>,
    body: web::Json<UpdateReleaseRequest>,
    db_pool: web::Data<Pool>,
) -> Result<impl Responder> {
    let (_, _, release_id) = path.into_inner();
    let user = web_user.into_user()?;

    if repo.archived_at.is_some() {
        die!(FORBIDDEN, "Repository is archived and read-only");
    }

    let mut tx = db_pool.begin().await?;

    if !privilege::check_push(&repo, Some(&user), &mut tx).await? {
        die!(FORBIDDEN, "Insufficient permissions");
    }

    let exists: bool = sqlx::query_scalar("select exists(select 1 from releases where id = $1 and repo_id = $2)")
        .bind(release_id)
        .bind(repo.id)
        .fetch_one(&mut *tx)
        .await?;

    if !exists {
        die!(NOT_FOUND, "Release not found");
    }

    if let Some(title) = &body.title {
        sqlx::query("update releases set title = $1 where id = $2")
            .bind(title)
            .bind(release_id)
            .execute(&mut *tx)
            .await?;
    }

    if let Some(description) = &body.description {
        sqlx::query("update releases set description = $1 where id = $2")
            .bind(description)
            .bind(release_id)
            .execute(&mut *tx)
            .await?;
    }

    if let Some(pre_release) = body.pre_release {
        sqlx::query("update releases set pre_release = $1 where id = $2")
            .bind(pre_release)
            .bind(release_id)
            .execute(&mut *tx)
            .await?;
    }

    let release: Release = sqlx::query_as("select * from releases where id = $1")
        .bind(release_id)
        .fetch_one(&mut *tx)
        .await?;

    let latest_id: Option<Uuid> = sqlx::query_scalar("select id from releases where repo_id = $1 and pre_release = false order by id desc limit 1")
        .bind(repo.id)
        .fetch_optional(&mut *tx)
        .await?;

    let libgit2_repo = repo.libgit2(&mut tx).await?;

    let (commit, commit_message) = tag_commit_info(&libgit2_repo, &release.tag);

    let assets: Vec<ReleaseAssets> = sqlx::query_as("select * from release_assets where release_id = $1 and available = true")
        .bind(release.id)
        .fetch_all(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok(HttpResponse::Ok().json(ReleaseResponse {
        latest: latest_id == Some(release.id),
        release,
        commit,
        commit_message,
        assets,
    }))
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpdateReleaseRequest {
    title: Option<String>,
    description: Option<String>,
    pre_release: Option<bool>,
}

#[utoipa::path(
    delete,
    path = "/api/repos/{namespace}/{repository}/releases/{release_id}",
    params(
        ("namespace" = String, Path, description = "Repository namespace (user or organization)"),
        ("repository" = String, Path, description = "Repository name"),
        ("release_id" = Uuid, Path, description = "Release ID"),
    ),
    responses(
        (status = 204, description = "Release deleted"),
        (status = 401, description = "Authentication required"),
        (status = 403, description = "Insufficient permissions"),
        (status = 404, description = "Release not found"),
    ),
    security(("cookieAuth" = [])),
    tag = "repository"
)]
#[route("/api/repos/{namespace}/{repository}/releases/{release_id}", method = "DELETE", err = "json")]
pub(crate) async fn delete_release(
    repo: Repository,
    web_user: WebUser,
    path: web::Path<(String, String, Uuid)>,
    storage: web::Data<Storage>,
    db_pool: web::Data<Pool>,
) -> Result<impl Responder> {
    let (_, _, release_id) = path.into_inner();
    let user = web_user.into_user()?;

    if repo.archived_at.is_some() {
        die!(FORBIDDEN, "Repository is archived and read-only");
    }

    let mut tx = db_pool.begin().await?;

    if !privilege::check_push(&repo, Some(&user), &mut tx).await? {
        die!(FORBIDDEN, "Insufficient permissions");
    }

    let exists: bool = sqlx::query_scalar("select exists(select 1 from releases where id = $1 and repo_id = $2)")
        .bind(release_id)
        .bind(repo.id)
        .fetch_one(&mut *tx)
        .await?;

    if !exists {
        die!(NOT_FOUND, "Release not found");
    }

    if let Some(store) = storage.as_ref().as_ref() {
        let assets: Vec<ReleaseAssets> = sqlx::query_as("select * from release_assets where release_id = $1")
            .bind(release_id)
            .fetch_all(&mut *tx)
            .await?;

        for asset in assets {
            if let Err(err) = store.delete(&asset.s3_key()).await {
                error!(?err, ?asset, "failed to delete asset in s3");
            }
        }
    }

    sqlx::query("delete from releases where id = $1 and repo_id = $2")
        .bind(release_id)
        .bind(repo.id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok(HttpResponse::NoContent().finish())
}

#[utoipa::path(
    post,
    path = "/api/repos/{namespace}/{repository}/releases/{release_id}/assets",
    params(
        ("namespace" = String, Path, description = "Repository namespace (user or organization)"),
        ("repository" = String, Path, description = "Repository name"),
        ("release_id" = Uuid, Path, description = "Release ID"),
    ),
    request_body = CreateAssetRequest,
    responses(
        (status = 200, description = "Asset created with upload URL", body = CreateAssetResponse),
        (status = 401, description = "Authentication required"),
        (status = 403, description = "Insufficient permissions"),
        (status = 404, description = "Release not found"),
        (status = 503, description = "Object storage not available"),
    ),
    security(("cookieAuth" = [])),
    tag = "repository"
)]
#[route("/api/repos/{namespace}/{repository}/releases/{release_id}/assets", method = "POST", err = "json")]
pub(crate) async fn create_asset(
    repo: Repository,
    web_user: WebUser,
    path: web::Path<(String, String, Uuid)>,
    body: web::Json<CreateAssetRequest>,
    storage: web::Data<Storage>,
    db_pool: web::Data<Pool>,
) -> Result<impl Responder> {
    let (_, _, release_id) = path.into_inner();
    let user = web_user.into_user()?;

    if repo.archived_at.is_some() {
        die!(FORBIDDEN, "Repository is archived and read-only");
    }

    let Some(store) = storage.as_ref().as_ref() else {
        die!(SERVICE_UNAVAILABLE, "Object storage is not available");
    };

    let mut tx = db_pool.begin().await?;

    if !privilege::check_push(&repo, Some(&user), &mut tx).await? {
        die!(FORBIDDEN, "Insufficient permissions");
    }

    let exists: bool = sqlx::query_scalar("select exists(select 1 from releases where id = $1 and repo_id = $2)")
        .bind(release_id)
        .bind(repo.id)
        .fetch_one(&mut *tx)
        .await?;

    if !exists {
        die!(NOT_FOUND, "Release not found");
    }

    let asset: ReleaseAssets = sqlx::query_as(
        "insert into release_assets (id, release_id, name, size, hash, os, arch, libc, kind) values ($1, $2, $3, $4, $5, $6, $7, $8, $9) returning *",
    )
    .bind(Uuid::now_v7())
    .bind(release_id)
    .bind(&body.name)
    .bind(body.size)
    .bind("")
    .bind(&body.os)
    .bind(&body.arch)
    .bind(&body.libc)
    .bind(&body.kind)
    .fetch_one(&mut *tx)
    .await?;

    let upload_url = store
        .signed_url(Method::PUT, &asset.s3_key(), Duration::from_secs(900))
        .await
        .context("failed to generate upload url")?;

    tx.commit().await?;

    Ok(HttpResponse::Ok().json(CreateAssetResponse {
        asset_id: asset.id,
        upload_url: upload_url.to_string(),
    }))
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateAssetRequest {
    name: String,
    size: i64,
    os: Option<Os>,
    arch: Option<Arch>,
    libc: Option<Libc>,
    kind: Option<Kind>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateAssetResponse {
    asset_id: Uuid,
    upload_url: String,
}

#[utoipa::path(
    put,
    path = "/api/repos/{namespace}/{repository}/releases/{release_id}/assets/{asset_id}/confirm",
    params(
        ("namespace" = String, Path, description = "Repository namespace (user or organization)"),
        ("repository" = String, Path, description = "Repository name"),
        ("release_id" = Uuid, Path, description = "Release ID"),
        ("asset_id" = Uuid, Path, description = "Asset ID"),
    ),
    responses(
        (status = 204, description = "Asset confirmed and available"),
        (status = 400, description = "Asset not yet uploaded to object storage"),
        (status = 401, description = "Authentication required"),
        (status = 403, description = "Insufficient permissions"),
        (status = 404, description = "Asset not found"),
        (status = 503, description = "Object storage not available"),
    ),
    security(("cookieAuth" = [])),
    tag = "repository"
)]
#[route(
    "/api/repos/{namespace}/{repository}/releases/{release_id}/assets/{asset_id}/confirm",
    method = "PUT",
    err = "json"
)]
pub(crate) async fn confirm_asset(
    repo: Repository,
    web_user: WebUser,
    path: web::Path<(String, String, Uuid, Uuid)>,
    storage: web::Data<Storage>,
    db_pool: web::Data<Pool>,
) -> Result<impl Responder> {
    let (_, _, release_id, asset_id) = path.into_inner();
    let user = web_user.into_user()?;

    let Some(store) = storage.as_ref().as_ref() else {
        die!(SERVICE_UNAVAILABLE, "Object storage is not available");
    };

    let mut tx = db_pool.begin().await?;

    if !privilege::check_push(&repo, Some(&user), &mut tx).await? {
        die!(FORBIDDEN, "Insufficient permissions");
    }

    let asset: Option<ReleaseAssets> = sqlx::query_as("select * from release_assets where id = $1 and release_id = $2")
        .bind(asset_id)
        .bind(release_id)
        .fetch_optional(&mut *tx)
        .await?;

    let Some(asset) = asset else {
        die!(NOT_FOUND, "Asset not found");
    };

    let object_bytes = store
        .get(&asset.s3_key())
        .await
        .map_err(|_| err!(BAD_REQUEST, "Asset has not been uploaded yet"))?
        .bytes()
        .await?;

    let hash = hex::encode(sha2::Sha256::digest(&object_bytes));

    sqlx::query("update release_assets set available = true, hash = $2 where id = $1")
        .bind(asset_id)
        .bind(&hash)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok(HttpResponse::NoContent().finish())
}

#[utoipa::path(
    get,
    path = "/api/repos/{namespace}/{repository}/releases/{release_id}/assets/{asset_id}/download",
    params(
        ("namespace" = String, Path, description = "Repository namespace (user or organization)"),
        ("repository" = String, Path, description = "Repository name"),
        ("release_id" = Uuid, Path, description = "Release ID"),
        ("asset_id" = Uuid, Path, description = "Asset ID"),
    ),
    responses(
        (status = 302, description = "Redirect to presigned download URL"),
        (status = 404, description = "Asset not found or not yet available"),
        (status = 503, description = "Object storage not available"),
    ),
    security((), ("cookieAuth" = [])),
    tag = "repository"
)]
#[route(
    "/api/repos/{namespace}/{repository}/releases/{release_id}/assets/{asset_id}/download",
    method = "GET",
    err = "json"
)]
pub(crate) async fn download_asset(
    repo: Repository,
    path: web::Path<(String, String, Uuid, Uuid)>,
    storage: web::Data<Storage>,
    db_pool: web::Data<Pool>,
) -> Result<impl Responder> {
    let (_, _, release_id, asset_id) = path.into_inner();

    let Some(store) = storage.as_ref().as_ref() else {
        die!(SERVICE_UNAVAILABLE, "Object storage is not available");
    };

    let mut tx = db_pool.begin().await?;

    let asset: Option<ReleaseAssets> = sqlx::query_as("select * from release_assets where id = $1 and release_id = $2 and available = true")
        .bind(asset_id)
        .bind(release_id)
        .fetch_optional(&mut *tx)
        .await?;

    let Some(asset) = asset else {
        die!(NOT_FOUND, "Asset not found");
    };

    let release_exists: bool = sqlx::query_scalar("select exists(select 1 from releases where id = $1 and repo_id = $2)")
        .bind(release_id)
        .bind(repo.id)
        .fetch_one(&mut *tx)
        .await?;

    if !release_exists {
        die!(NOT_FOUND, "Asset not found");
    }

    sqlx::query("update release_assets set downloads = downloads + 1 where id = $1")
        .bind(asset_id)
        .execute(&mut *tx)
        .await?;

    let download_url = store
        .signed_url(Method::GET, &asset.s3_key(), Duration::from_secs(900))
        .await
        .context("failed to generate download url")?;

    tx.commit().await?;

    Ok(HttpResponse::Found().append_header((LOCATION, download_url.to_string())).finish())
}

#[utoipa::path(
    delete,
    path = "/api/repos/{namespace}/{repository}/releases/{release_id}/assets/{asset_id}",
    params(
        ("namespace" = String, Path, description = "Repository namespace (user or organization)"),
        ("repository" = String, Path, description = "Repository name"),
        ("release_id" = Uuid, Path, description = "Release ID"),
        ("asset_id" = Uuid, Path, description = "Asset ID"),
    ),
    responses(
        (status = 204, description = "Asset deleted"),
        (status = 401, description = "Authentication required"),
        (status = 403, description = "Insufficient permissions"),
        (status = 404, description = "Asset not found"),
        (status = 503, description = "Object storage not available"),
    ),
    security(("cookieAuth" = [])),
    tag = "repository"
)]
#[route("/api/repos/{namespace}/{repository}/releases/{release_id}/assets/{asset_id}", method = "DELETE", err = "json")]
pub(crate) async fn delete_asset(
    repo: Repository,
    web_user: WebUser,
    path: web::Path<(String, String, Uuid, Uuid)>,
    storage: web::Data<Storage>,
    db_pool: web::Data<Pool>,
) -> Result<impl Responder> {
    let (_, _, release_id, asset_id) = path.into_inner();
    let user = web_user.into_user()?;

    if repo.archived_at.is_some() {
        die!(FORBIDDEN, "Repository is archived and read-only");
    }

    let Some(store) = storage.as_ref().as_ref() else {
        die!(SERVICE_UNAVAILABLE, "Object storage is not available");
    };

    let mut tx = db_pool.begin().await?;

    if !privilege::check_push(&repo, Some(&user), &mut tx).await? {
        die!(FORBIDDEN, "Insufficient permissions");
    }

    let exists: bool = sqlx::query_scalar("select exists(select 1 from releases where id = $1 and repo_id = $2)")
        .bind(release_id)
        .bind(repo.id)
        .fetch_one(&mut *tx)
        .await?;

    if !exists {
        die!(NOT_FOUND, "Asset not found");
    }

    let asset: Option<ReleaseAssets> = sqlx::query_as("select * from release_assets where id = $1 and release_id = $2")
        .bind(asset_id)
        .bind(release_id)
        .fetch_optional(&mut *tx)
        .await?;

    let Some(asset) = asset else {
        die!(NOT_FOUND, "Asset not found");
    };

    store.delete(&asset.s3_key()).await.context("failed to delete asset")?;

    sqlx::query("delete from release_assets where id = $1").bind(asset_id).execute(&mut *tx).await?;

    tx.commit().await?;

    Ok(HttpResponse::NoContent().finish())
}

#[instrument(skip(libgit2_repo))]
fn tag_commit_info(libgit2_repo: &git2::Repository, tag: &str) -> (Option<String>, Option<String>) {
    let Ok(reference) = libgit2_repo.find_reference(&format!("refs/tags/{tag}")) else {
        return (None, None);
    };

    let Ok(commit) = reference.peel_to_commit() else {
        return (None, None);
    };

    let id = commit.id().to_string();
    let message = commit.message().and_then(|m| m.lines().next()).unwrap_or_default().to_owned();

    (Some(id), Some(message))
}
