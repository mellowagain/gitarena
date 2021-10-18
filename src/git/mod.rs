use git_odb::pack::cache::lru::StaticLinkedList;

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

pub(crate) const ODB_CACHE_SIZE: usize = 64;
pub(crate) type GitoxideCacheList = StaticLinkedList<ODB_CACHE_SIZE>;
