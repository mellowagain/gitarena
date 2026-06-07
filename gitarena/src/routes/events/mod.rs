use crate::events::Event;

use actix_web::web::ServiceConfig;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

pub(crate) mod audit_log;
pub(crate) mod contributions;
pub(crate) mod dashboard;
pub(crate) mod org_audit_log;
pub(crate) mod user_feed;

pub(crate) fn init(config: &mut ServiceConfig) {
    config
        .service(audit_log::get_personal_audit_log)
        .service(contributions::get_contributions)
        .service(org_audit_log::get_org_audit_log)
        .service(dashboard::get_dashboard_feed)
        .service(user_feed::get_user_feed);
}

#[derive(Deserialize)]
pub(crate) struct EventListParams {
    #[serde(default)]
    pub(crate) offset: i64,
    #[serde(default = "default_limit")]
    pub(crate) limit: i64,
    #[serde(default, rename = "type")]
    pub(crate) type_filter: Option<String>,
}

pub(crate) fn default_limit() -> i64 {
    20
}

impl EventListParams {
    pub(crate) fn sanitize(mut self) -> Self {
        self.offset = self.offset.max(0);
        self.limit = self.limit.clamp(1, 100);
        self
    }
}

#[derive(Serialize, ToSchema, FromRow)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EventResponse {
    #[serde(flatten)]
    #[sqlx(flatten)]
    pub(crate) event: Event,
    pub(crate) actor_username: Option<String>,
    pub(crate) subject_name: Option<String>,
    pub(crate) subject_namespace: Option<String>,
}
