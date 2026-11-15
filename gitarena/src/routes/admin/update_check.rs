use crate::prelude::AwcExtensions;
use crate::user::WebUser;
use crate::{die, err};
use actix_web::{HttpResponse, Responder};
use anyhow::{Context, Result, anyhow};
use awc::Client;
use cached::cached;
use gitarena_macros::route;
use opentelemetry_instrumentation_actix_web::ClientExt;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::instrument;
use utoipa::ToSchema;

#[utoipa::path(
    get,
    path = "/api/admin/check-update",
    summary = "Check for GitArena updates",
    responses(
        (status = 200, description = "Update info", body = UpdateInfo),
        (status = 401, description = "Authentication required"),
        (status = 403, description = "Admin required"),
    ),
    security(("cookieAuth" = [])),
    tag = "admin"
)]
#[route("/api/admin/check-update", method = "GET", err = "json")]
pub(crate) async fn get_update_check(web_user: WebUser) -> Result<impl Responder> {
    let user = web_user.into_user()?;

    if !user.admin {
        die!(FORBIDDEN, "Admin endpoints can only be called by admins");
    }

    Ok(HttpResponse::Ok().json(check_github().await?))
}

#[derive(Serialize, ToSchema, Default, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpdateInfo {
    pub(crate) available: bool,
    pub(crate) current_version: String,
    pub(crate) new_version: Option<String>,
    pub(crate) notes: Option<String>,
    pub(crate) security_relevant: Option<bool>,
}

#[instrument(err)]
#[cached(ttl = "Duration::from_hours(1)")]
pub(crate) async fn check_github() -> Result<UpdateInfo> {
    if let Some(current_commit) = option_env!("VERGEN_GIT_SHA")
        && current_commit != "unknown"
    {
        let response = Client::gitarena()
            .get(format!("https://api.github.com/repos/mellowagain/gitarena/compare/{current_commit}...main"))
            .timeout(Duration::from_secs(5))
            .trace_request()
            .send()
            .await
            .map_err(|err| err!(BAD_GATEWAY, "Failed to connect to GitHub api: {}", err))?
            .json::<serde_json::Value>()
            .await
            .map_err(|err| err!(BAD_GATEWAY, "Failed to parse GitHub response as JSON: {}", err))?;

        let status = response
            .get("status")
            .ok_or_else(|| anyhow!("GitHub api json response does not contain `status` field"))?
            .as_str()
            .ok_or_else(|| anyhow!("GitHub api json response `status` field is not a string"))?;

        if status == "ahead" || status == "behind" || status == "diverged" {
            let commits = response
                .get("commits")
                .ok_or_else(|| anyhow!("GitHub api json response does not contain `commits` field"))?
                .as_array()
                .ok_or_else(|| anyhow!("GitHub api json response `commits` field is not a array"))?
                .iter()
                .cloned()
                .map(serde_json::from_value)
                .collect::<Result<Vec<GitHubApiCommit>, _>>()
                .context("failed to parse `commits` field into type")?;

            let last = commits.last().map(|c| c.sha.clone());
            let mut notes = Vec::with_capacity(commits.len());
            let mut security_relevant = false;

            for commit in commits {
                let message = commit.commit.message;

                if let Some(message) = message.split('\n').next() {
                    notes.push(format!("- {message}"));
                }

                if message.contains("GHSA-") || message.contains("CVE-") {
                    security_relevant = true;
                }
            }

            Ok(UpdateInfo {
                available: true,
                current_version: current_commit.to_string(),
                new_version: last,
                notes: if notes.is_empty() { None } else { Some(notes.join("\n")) },
                security_relevant: Some(security_relevant),
            })
        } else {
            Ok(UpdateInfo {
                available: false,
                current_version: current_commit.to_string(),
                ..Default::default()
            })
        }
    } else {
        Ok(UpdateInfo::default())
    }
}

#[derive(Deserialize, Debug)]
struct GitHubApiCommit {
    sha: String,
    commit: GitHubApiCommitMetadata,
}

#[derive(Deserialize, Debug)]
struct GitHubApiCommitMetadata {
    message: String,
}
