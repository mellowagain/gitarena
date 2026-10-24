//! Error type returned by this crate

use crate::id::{AuthorId, BugId};
use std::error::Error as StdError;
use std::io;
use std::num::ParseIntError;
use thiserror::Error;

/// Error returned by all fallible functions of this crate
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// No ref exists for the bug
    #[error("bug {0} not found")]
    BugNotFound(BugId),

    /// A ref for the bug already exists
    #[error("bug {0} already exists")]
    BugAlreadyExists(BugId),

    /// The bug's history violates the `git-bug` spec
    #[error("invalid bug {id}: {reason}")]
    InvalidBug {
        /// Bug that was being read
        id: BugId,
        /// What part of the spec was violated
        reason: String,
    },

    /// A ref under [`BUGS_REF_PREFIX`](crate::bug::BUGS_REF_PREFIX) does not end in a valid [`BugId`]
    #[error("{0} does not name a valid bug")]
    InvalidBugRef(String),

    /// No ref exists for the identity
    #[error("identity {0} not found")]
    IdentityNotFound(AuthorId),

    /// The identity's history violates the `git-bug` spec
    #[error("invalid identity {id}: {reason}")]
    InvalidIdentity {
        /// Identity that was being read
        id: AuthorId,
        /// What part of the spec was violated
        reason: String,
    },

    /// Local and remote identity histories diverged. Identities can only be fast-forwarded (spec identity.md §6)
    #[error("identity {0} has diverged and cannot be fast-forwarded")]
    IdentityDiverged(AuthorId),

    /// Reading or writing a Lamport clock file failed
    #[error("failed to access clock {name}")]
    Clock {
        /// Clock name, e.g. [`BUGS_EDIT`](crate::clock::BUGS_EDIT)
        name: String,
        /// Underlying I/O error
        #[source]
        source: io::Error,
    },

    /// A Lamport clock file does not contain a number
    #[error("clock {name} is corrupt")]
    CorruptClock {
        /// Clock name, e.g. [`BUGS_EDIT`](crate::clock::BUGS_EDIT)
        name: String,
        /// Underlying parse error
        #[source]
        source: ParseIntError,
    },

    /// Taking the lock that serializes ref updates failed
    #[error("failed to lock refs")]
    RefLock(#[source] io::Error),

    /// A git operation failed
    #[error(transparent)]
    Git(Box<dyn StdError + Send + Sync>),
}

impl Error {
    pub(crate) fn git(err: impl StdError + Send + Sync + 'static) -> Self {
        Self::Git(Box::new(err))
    }
}
