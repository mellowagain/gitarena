use crate::database::Pool;
use crate::die;
use crate::repository::Repository;
use crate::routes::repository::api::issues::get_issue_by_index;
use crate::user::WebUser;
use actix_web::{HttpResponse, Responder, web};
use anyhow::Result;
use gitarena_issues::author::load_author_name;
use gitarena_issues::bug::load_bug;
use gitarena_issues::operation::{BugStatus, OperationType};
use gitarena_macros::route;
use serde::Serialize;
use tracing::instrument;
use utoipa::ToSchema;

#[utoipa::path(
    get,
    path = "/api/repos/{namespace}/{repository}/issues/{index}/timeline",
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

    let bug = load_bug(&gitoxide_repo, &issue.git_bug_id)?;
    let events = build_timeline(&gitoxide_repo, &bug);

    Ok(HttpResponse::Ok().json(events))
}

fn build_timeline(gitoxide_repo: &gix::Repository, bug: &gitarena_issues::bug::Bug) -> Vec<TimelineEvent> {
    let mut events = Vec::new();
    let mut current_title = String::new();

    for pack in &bug.packs {
        let author_username = load_author_name(gitoxide_repo, &pack.author.id).unwrap_or_else(|_| pack.author.id.clone());

        for op in &pack.ops {
            match op.op_type {
                OperationType::CREATE => {
                    current_title = op.title.clone().unwrap_or_default();
                    events.push(TimelineEvent {
                        kind: "created".to_owned(),
                        timestamp: op.timestamp,
                        author_username: author_username.clone(),
                        old_title: None,
                        new_title: None,
                        status: None,
                        label: None,
                    });
                }
                OperationType::SET_TITLE => {
                    let new_title = op.title.clone().unwrap_or_default();
                    events.push(TimelineEvent {
                        kind: "title_changed".to_owned(),
                        timestamp: op.timestamp,
                        author_username: author_username.clone(),
                        old_title: Some(current_title.clone()),
                        new_title: Some(new_title.clone()),
                        status: None,
                        label: None,
                    });
                    current_title = new_title;
                }
                OperationType::SET_STATUS => {
                    let status_str = match op.status {
                        Some(s) if s == BugStatus::OPEN => "open",
                        _ => "closed",
                    };
                    events.push(TimelineEvent {
                        kind: "status_changed".to_owned(),
                        timestamp: op.timestamp,
                        author_username: author_username.clone(),
                        old_title: None,
                        new_title: None,
                        status: Some(status_str.to_owned()),
                        label: None,
                    });
                }
                OperationType::LABEL_CHANGE => {
                    if let Some(added) = &op.added {
                        for label in added {
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
                    }
                    if let Some(removed) = &op.removed {
                        for label in removed {
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
                    }
                }
                // ADD_COMMENT and EDIT_COMMENT are handled separately via comments API
                _ => {}
            }
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
