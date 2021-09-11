use serde::Serialize;

#[derive(Serialize)]
pub(crate) struct RepoFile<'a> {
    pub(crate) file_type: u16,
    pub(crate) file_name: &'a str,
    pub(crate) commit: GitCommit<'a>,
    pub(crate) submodule_target_oid: Option<String>,
}

#[derive(Serialize)]
pub(crate) struct RepoReadme<'a> {
    pub(crate) file_name: &'a str,
    pub(crate) content: &'a str
}

#[derive(Serialize)]
pub(crate) struct GitCommit<'a> {
    pub(crate) oid: String,
    pub(crate) message: String,
    pub(crate) time: i64,

    pub(crate) author_name: &'a str,
    pub(crate) author_uid: Option<i32>
}
