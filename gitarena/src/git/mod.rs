use git_repository::hash::Kind;

pub(crate) mod basic_auth;
pub(crate) mod capabilities;
pub(crate) mod fetch;
pub(crate) mod history;
pub(crate) mod hooks;
pub(crate) mod io;
pub(crate) mod ls_refs;
pub(crate) mod pack;
pub(crate) mod receive_pack;
pub(crate) mod ref_update;
pub(crate) mod utils;
pub(crate) mod write;

pub(crate) const GIT_HASH_KIND: Kind = Kind::Sha1;
