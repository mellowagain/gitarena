use crate::database::Pool;
use crate::prelude::LibGit2SignatureExtensions;
use crate::privileges::privilege;
use crate::repository::Repository;
use crate::routes::repository::api::commit_detail::SignatureInfo;
use crate::user::WebUser;
use crate::{die, err};
use actix_web::{HttpResponse, Responder, web};
use anyhow::Result;
use chrono::{DateTime, TimeZone, Utc};
use git2::{ObjectType, Signature};
use gitarena_macros::route;
use serde::Serialize;
use utoipa::ToSchema;

#[utoipa::path(
    get,
    path = "/api/repos/{namespace}/{repository}/tags",
    params(
        ("namespace" = String, Path, description = "Repository namespace (user or organization)"),
        ("repository" = String, Path, description = "Repository name"),
    ),
    responses(
        (status = 200, description = "List of tags", body = TagsResponse),
        (status = 404, description = "Repository not found or access denied"),
    ),
    security((), ("cookieAuth" = [])),
    tag = "repository"
)]
#[route("/api/repos/{namespace}/{repository}/tags", method = "GET", err = "json")]
pub(crate) async fn list_tags(repo: Repository, db_pool: web::Data<Pool>) -> Result<impl Responder> {
    let mut tx = db_pool.begin().await?;

    let libgit2_repo = repo.libgit2(&mut tx).await?;

    let mut raw: Vec<RawTagData> = Vec::new();

    libgit2_repo.tag_foreach(|oid, name_bytes| {
        let cow = String::from_utf8_lossy(name_bytes);

        let Some(name) = cow.strip_prefix("refs/tags/") else {
            return true;
        };

        let (kind, message) = match libgit2_repo.find_tag(oid) {
            Ok(tag) => (TagKind::Annotated, tag.message().map(str::to_owned)),
            Err(_) => (TagKind::Lightweight, None),
        };

        let Ok(commit) = libgit2_repo
            .find_object(oid, None)
            .and_then(|obj| obj.peel(ObjectType::Commit))
            .map(|obj| obj.into_commit().expect("peeling to a commit to actually result in a commit"))
        else {
            return true;
        };

        raw.push(RawTagData {
            name: name.to_owned(),
            commit: commit.id().to_string(),
            commit_message: commit.message().and_then(|m| m.lines().next()).unwrap_or("").to_owned(),
            author: commit.author().to_owned(),
            timestamp: commit.time().seconds(),
            kind,
            message,
        });

        true
    })?;

    let mut tags = Vec::<TagInfo>::with_capacity(raw.len());

    for raw in raw {
        let (name, uid, email) = raw.author.try_disassemble(&mut tx).await;
        let date = Utc.timestamp_opt(raw.timestamp, 0).single().unwrap_or(DateTime::<Utc>::UNIX_EPOCH);

        tags.push(TagInfo {
            name: raw.name,
            commit: raw.commit,
            commit_message: raw.commit_message,
            author: SignatureInfo {
                name,
                email,
                timestamp: raw.timestamp,
                uid,
            },
            date,
            kind: raw.kind,
            message: raw.message,
        });
    }

    tx.commit().await?;

    tags.sort_by(|a, b| b.date.cmp(&a.date));

    Ok(HttpResponse::Ok().json(TagsResponse { tags }))
}

#[utoipa::path(
    delete,
    path = "/api/repos/{namespace}/{repository}/tags/{tag}",
    params(
        ("namespace" = String, Path, description = "Repository namespace (user or organization)"),
        ("repository" = String, Path, description = "Repository name"),
        ("tag" = String, Path, description = "Tag name to delete"),
    ),
    responses(
        (status = 204, description = "Tag deleted"),
        (status = 401, description = "Authentication required"),
        (status = 403, description = "Insufficient permissions or repository is archived"),
        (status = 404, description = "Repository or tag not found"),
    ),
    security(("cookieAuth" = [])),
    tag = "repository"
)]
#[route("/api/repos/{namespace}/{repository}/tags/{tag}", method = "DELETE", err = "json")]
pub(crate) async fn delete_tag(
    repo: Repository,
    web_user: WebUser,
    path: web::Path<(String, String, String)>,
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

    tx.commit().await?;

    let (_, _, tag_name) = path.into_inner();
    let ref_name = format!("refs/tags/{tag_name}");

    let mut reference = libgit2_repo.find_reference(&ref_name).map_err(|_| err!(NOT_FOUND, "Tag not found"))?;

    reference.delete()?;

    Ok(HttpResponse::NoContent().finish())
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub(crate) enum TagKind {
    /// Tag ref points directly to a commit object
    Lightweight,
    /// Tag ref points to a tag object which in turn points to a commit
    Annotated,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TagInfo {
    /// Name
    name: String,
    /// Commit SHA
    commit: String,
    /// First line of the commit message
    commit_message: String,
    /// Author
    author: SignatureInfo,
    /// Timestamp
    date: DateTime<Utc>,
    /// Kind
    kind: TagKind,
    /// Annotated tag message, if present
    message: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub(crate) struct TagsResponse {
    /// List of tags
    tags: Vec<TagInfo>,
}

struct RawTagData {
    name: String,
    commit: String,
    commit_message: String,
    author: Signature<'static>,
    timestamp: i64,
    kind: TagKind,
    message: Option<String>,
}
