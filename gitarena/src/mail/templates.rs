use askama::Template;

#[derive(Template)]
#[template(path = "verify_email.txt", escape = "none")]
pub(crate) struct VerifyEmailTemplate<'a> {
    pub(crate) link: &'a str,

    pub(crate) instance_name: &'a str,
    pub(crate) domain: &'a str,
}

impl VerifyEmailTemplate<'_> {
    pub(crate) fn subject(&self) -> String {
        "Verify your email address".to_string()
    }
}

#[derive(Template)]
#[template(path = "org_invite.txt", escape = "none")]
pub(crate) struct OrgInviteTemplate<'a> {
    pub(crate) inviter: &'a str,
    pub(crate) org: &'a str,
    pub(crate) role: &'a str,
    pub(crate) link: &'a str,

    pub(crate) instance_name: &'a str,
    pub(crate) domain: &'a str,
}

impl OrgInviteTemplate<'_> {
    pub(crate) fn subject(&self) -> String {
        format!("You've been invited to join {}", self.org)
    }
}
