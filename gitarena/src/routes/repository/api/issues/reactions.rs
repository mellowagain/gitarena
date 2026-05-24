use crate::database::Pool;
use crate::die;
use crate::repository::Repository;
use crate::routes::repository::api::issues::{ReactionGroup, ReactionTarget, get_issue_by_index, load_reactions};
use crate::user::WebUser;
use actix_web::{HttpResponse, Responder, web};
use anyhow::Result;
use gitarena_macros::route;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[utoipa::path(
    post,
    path = "/api/repos/{namespace}/{repository}/issues/{index}/reactions",
    params(
        ("namespace" = String, Path, description = "Repository namespace"),
        ("repository" = String, Path, description = "Repository name"),
        ("index" = i32, Path, description = "Issue number"),
    ),
    request_body = ToggleReactionRequest,
    responses(
        (status = 200, description = "Updated reactions", body = ReactionsResponse),
        (status = 401, description = "Authentication required"),
        (status = 404, description = "Issue not found"),
    ),
    security(("cookieAuth" = [])),
    tag = "issues"
)]
#[route("/api/repos/{namespace}/{repository}/issues/{index}/reactions", method = "POST", err = "json")]
pub(crate) async fn toggle_issue_reaction(
    repo: Repository,
    web_user: WebUser,
    path: web::Path<(String, String, i32)>,
    body: web::Json<ToggleReactionRequest>,
    db_pool: web::Data<Pool>,
) -> Result<impl Responder> {
    let user = web_user.into_user()?;

    let (_, _, index) = path.into_inner();
    let emoji = body.into_inner().emoji;

    if !ALLOWED_EMOJIS.contains(&emoji.as_str()) {
        die!(BAD_REQUEST, "Emoji not allowed");
    }

    let mut tx = db_pool.begin().await?;

    let issue = get_issue_by_index(repo.id, index, &mut tx).await?;

    let existing: Option<Uuid> = sqlx::query_scalar("select id from reactions where issue_id = $1 and user_id = $2 and emoji = $3 limit 1")
        .bind(issue.id)
        .bind(user.id)
        .bind(&emoji)
        .fetch_optional(&mut *tx)
        .await?;

    if let Some(reaction_id) = existing {
        sqlx::query("delete from reactions where id = $1").bind(reaction_id).execute(&mut *tx).await?;
    } else {
        sqlx::query("insert into reactions (id, user_id, emoji, issue_id) values ($1, $2, $3, $4)")
            .bind(Uuid::now_v7())
            .bind(user.id)
            .bind(&emoji)
            .bind(issue.id)
            .execute(&mut *tx)
            .await?;
    }

    let reactions = load_reactions(ReactionTarget::Issue(issue.id), Some(user.id), &mut tx).await?;

    tx.commit().await?;

    Ok(HttpResponse::Ok().json(ReactionsResponse { reactions }))
}

#[utoipa::path(
    post,
    path = "/api/repos/{namespace}/{repository}/issues/{index}/comments/{comment_id}/reactions",
    params(
        ("namespace" = String, Path, description = "Repository namespace"),
        ("repository" = String, Path, description = "Repository name"),
        ("index" = i32, Path, description = "Issue number"),
        ("comment_id" = Uuid, Path, description = "Comment ID"),
    ),
    request_body = ToggleReactionRequest,
    responses(
        (status = 200, description = "Updated reactions", body = ReactionsResponse),
        (status = 401, description = "Authentication required"),
        (status = 404, description = "Comment not found"),
    ),
    security(("cookieAuth" = [])),
    tag = "issues"
)]
#[route(
    "/api/repos/{namespace}/{repository}/issues/{index}/comments/{comment_id}/reactions",
    method = "POST",
    err = "json"
)]
pub(crate) async fn toggle_comment_reaction(
    _repo: Repository,
    web_user: WebUser,
    path: web::Path<(String, String, i32, Uuid)>,
    body: web::Json<ToggleReactionRequest>,
    db_pool: web::Data<Pool>,
) -> Result<impl Responder> {
    let user = web_user.into_user()?;

    let (_, _, _, comment_id) = path.into_inner();
    let emoji = body.into_inner().emoji;

    if !ALLOWED_EMOJIS.contains(&emoji.as_str()) {
        die!(BAD_REQUEST, "Emoji not allowed");
    }

    let mut tx = db_pool.begin().await?;

    let exists: bool = sqlx::query_scalar("select exists(select 1 from issue_comment_cache where id = $1)")
        .bind(comment_id)
        .fetch_one(&mut *tx)
        .await?;

    if !exists {
        die!(NOT_FOUND, "Comment not found");
    }

    let existing: Option<Uuid> = sqlx::query_scalar("select id from reactions where comment_id = $1 and user_id = $2 and emoji = $3 limit 1")
        .bind(comment_id)
        .bind(user.id)
        .bind(&emoji)
        .fetch_optional(&mut *tx)
        .await?;

    if let Some(reaction_id) = existing {
        sqlx::query("delete from reactions where id = $1").bind(reaction_id).execute(&mut *tx).await?;
    } else {
        sqlx::query("insert into reactions (id, user_id, emoji, comment_id) values ($1, $2, $3, $4)")
            .bind(Uuid::now_v7())
            .bind(user.id)
            .bind(&emoji)
            .bind(comment_id)
            .execute(&mut *tx)
            .await?;
    }

    let reactions = load_reactions(ReactionTarget::Comment(comment_id), Some(user.id), &mut tx).await?;

    tx.commit().await?;

    Ok(HttpResponse::Ok().json(ReactionsResponse { reactions }))
}

// todo: make configurable
const ALLOWED_EMOJIS: &[&str] = &[
    "👍",
    "👎",
    "😄",
    "🎉",
    "😕",
    "❤️",
    "🚀",
    "👀",
    "😹😭✌️",
    "☝️🤓",
    "🕳️👨‍🦯",
    "❓",
    "⁉️",
    "‼️",
    "😭",
    "😂",
    "🥀",
    "🤬",
    "🥹",
    "🍔",
    "💯",
    "🤣",
    "🫶",
    "💀",
    "🤡",
    "🧢",
    "🐐",
    "🙏",
    "🔥",
    "😈",
    "😔",
    "✅",
    "🤦",
    "❌",
];

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ToggleReactionRequest {
    emoji: String,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all(serialize = "camelCase"))]
pub(crate) struct ReactionsResponse {
    reactions: Vec<ReactionGroup>,
}
