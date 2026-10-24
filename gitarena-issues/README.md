# gitarena-issues

Rust library for interacting with `git-bug` issues. Written primarily for usage with the `gitarena` software development platform (forge)
but open-sourced and published for greater usage by the Rust community.

## Target

The library is currently targeting git-bug v0.11.0 and the test suite is testing against it.

## Features

- `tracing`: Instrument the bug and operation functions with [`tracing`](https://crates.io/crates/tracing) spans

## Limitations

- Merging (`merge_bug`, `merge_identity`) expects the remote commits to already be fetched into the repository, fetching and pushing is left to the caller.
- Bug ref updates are only serialized between users of this crate. A `git` or `git-bug` process updating the same ref at the same time can still have its update dropped, as gix checks the old ref value before taking the ref lock.
- Git writes are not in a transaction, meaning caller errors (e.g Postgres failures) from for example GitArena will still create the git-bug issue

## Example usage

```rust
use anyhow::Result;
use gitarena_issues::author::Author;
use gitarena_issues::bug::{create_bug, load_bug};
use gitarena_issues::operation::BugStatus;
use gitarena_issues::ops::{add_comment, set_status};
use uuid::Uuid;

# fn main() -> Result<()> {
# let dir = tempfile::tempdir()?;
let repo = gix::init(dir.path())?;
let author = Author::from_user(Uuid::new_v4(), "mari", "git_bug@git.mari.zip", 1_700_000_000);

let bug = create_bug(&repo, author.clone(), "Crash on startup".into(), "Steps to reproduce: ...".into())?;
let bug = add_comment(&repo, bug, author.clone(), "Fixed in abc123".into())?.bug;
set_status(&repo, bug.clone(), author, BugStatus::CLOSED)?;

let snapshot = load_bug(&repo, &bug.id)?.snapshot();
assert_eq!(snapshot.status, BugStatus::CLOSED);
assert_eq!(snapshot.comments.len(), 1);
# Ok(())
# }
```

For advanced usage, consider checking out the [GitArena](https://github.com/mellowagain/gitarena/tree/main/gitarena) source code.

## Why not `git-bug` crate on crates.io

<https://crates.io/crates/git-bug> currently lacks write support. I wanted to contribute to it but the original source
code repo linked from the crates.io page is down. The last update has also been as of writing over a year ago.

## Structure

- `tests` - Integration tests testing against real `git-bug` CLI
- `../gitarena` - Git forge implementing `git-bug`-backend storage for issues
