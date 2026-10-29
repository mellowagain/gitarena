use crate::database::Pool;
use crate::die;
use crate::repository::Repository;
use crate::routes::repository::api::issues::get_issue_by_index;
use crate::user::WebUser;
use actix_web::{HttpResponse, Responder, web};
use anyhow::Result;
use gitarena_issues::author::load_author_name;
use gitarena_issues::bug::{Bug, load_bug};
use gitarena_issues::operation::BugStatus;
use gitarena_macros::route;
use serde::Serialize;
use tracing::instrument;
use utoipa::ToSchema;

#[utoipa::path(
    get,
    path = "/api/repos/{namespace}/{repository}/issues/{index}/timeline",
    summary = "Get Issue Timeline",
    params(
        ("namespace" = String, Path, description = "Repository namespace"),
        ("repository" = String, Path, description = "Repository name"),
        ("index" = i32, Path, description = "Issue number"),
    ),
    responses(
        (status = 200, description = "Issue timeline events", body = Vec<TimelineEvent>),
        (status = 404, description = "Issue not found"),
    ),
    security((), ("cookieAuth" = [])),
    tag = "issues"
)]
#[route("/api/repos/{namespace}/{repository}/issues/{index}/timeline", method = "GET", err = "json")]
#[instrument(err, skip(db_pool))]
pub(crate) async fn get_issue_timeline(
    repo: Repository,
    web_user: WebUser,
    path: web::Path<(String, String, i32)>,
    db_pool: web::Data<Pool>,
) -> Result<impl Responder> {
    let (_, _, index) = path.into_inner();
    let mut tx = db_pool.begin().await?;

    let issue = get_issue_by_index(repo.id, index, &mut tx).await?;

    let show_confidential = web_user.as_ref().is_some_and(|user| repo.owner_user == Some(user.id));

    if issue.confidential && !show_confidential {
        die!(NOT_FOUND, "Issue not found");
    }

    let gitoxide_repo = repo.gitoxide(&mut tx).await?;

    tx.commit().await?;

    let bug = load_bug(&gitoxide_repo, &issue.git_bug_id.parse()?)?;
    let events = build_timeline(&gitoxide_repo, &bug);

    Ok(HttpResponse::Ok().json(events))
}

fn build_timeline(gitoxide_repo: &gix::Repository, bug: &Bug) -> Vec<TimelineEvent> {
    let mut events = Vec::new();

    let mut replayed = bug.clone();
    replayed.packs.clear();

    let mut before = replayed.snapshot();

    for (pack_index, pack) in bug.packs.iter().enumerate() {
        let author_username = load_author_name(gitoxide_repo, &pack.author.id).unwrap_or_else(|_| pack.author.id.to_string());

        for (op_index, op) in pack.ops.iter().enumerate() {
            let mut partial = pack.clone();
            partial.ops.truncate(op_index + 1);
            replayed.packs.truncate(pack_index);
            replayed.packs.push(partial);

            let after = replayed.snapshot();

            if pack_index == 0 && op_index == 0 {
                events.push(TimelineEvent {
                    kind: "created".to_owned(),
                    timestamp: op.timestamp,
                    author_username: author_username.clone(),
                    old_title: None,
                    new_title: None,
                    status: None,
                    label: None,
                });

                before = after;
                continue;
            }

            if before.title != after.title {
                events.push(TimelineEvent {
                    kind: "title_changed".to_owned(),
                    timestamp: op.timestamp,
                    author_username: author_username.clone(),
                    old_title: Some(op.was.clone().unwrap_or_else(|| before.title.clone())),
                    new_title: Some(after.title.clone()),
                    status: None,
                    label: None,
                });
            }

            if before.status != after.status {
                events.push(TimelineEvent {
                    kind: "status_changed".to_owned(),
                    timestamp: op.timestamp,
                    author_username: author_username.clone(),
                    old_title: None,
                    new_title: None,
                    status: Some(if after.status == BugStatus::OPEN {
                        "open".to_owned()
                    } else {
                        "closed".to_owned()
                    }),
                    label: None,
                });
            }

            for label in after.labels.iter().filter(|label| !before.labels.contains(label)) {
                events.push(TimelineEvent {
                    kind: "label_added".to_owned(),
                    timestamp: op.timestamp,
                    author_username: author_username.clone(),
                    old_title: None,
                    new_title: None,
                    status: None,
                    label: Some(label.clone()),
                });
            }

            for label in before.labels.iter().filter(|label| !after.labels.contains(label)) {
                events.push(TimelineEvent {
                    kind: "label_removed".to_owned(),
                    timestamp: op.timestamp,
                    author_username: author_username.clone(),
                    old_title: None,
                    new_title: None,
                    status: None,
                    label: Some(label.clone()),
                });
            }

            before = after;
        }
    }

    events
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TimelineEvent {
    /// Type
    #[serde(rename = "type")]
    pub(crate) kind: String,
    /// Unix timestamp
    pub(crate) timestamp: i64,
    /// Actor username
    pub(crate) author_username: String,
    /// Previous title, set for `title_changed` events
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) old_title: Option<String>,
    /// New title, set for `title_changed` events
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) new_title: Option<String>,
    /// New status (`"open"` or `"closed"`), set for `status_changed` events
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) status: Option<String>,
    /// Label name, set for `label_added` and `label_removed` events
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) label: Option<String>,
}
