use chrono::{DateTime, FixedOffset};
use serde::Serialize;

#[derive(Serialize)]
pub(crate) struct RepoFile<'a> {
    pub(crate) file_type: u16,
    pub(crate) file_name: &'a str,
    pub(crate) commit: GitCommit,
    pub(crate) submodule_target_oid: Option<String>,
}

#[derive(Serialize)]
pub(crate) struct RepoReadme<'a> {
    pub(crate) file_name: &'a str,
    pub(crate) content: &'a str
}

#[derive(Serialize)]
pub(crate) struct GitCommit {
    pub(crate) oid: String,
    pub(crate) message: String,

    pub(crate) time: i64, // Unix timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) date: Option<DateTime<FixedOffset>>,

    pub(crate) author_name: String,
    pub(crate) author_uid: Option<i32>,
    pub(crate) author_email: String
}
