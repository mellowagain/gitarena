# gitarena-issues

Rust library for interacting with `git-bug` issues. Written primarily for usage with the `gitarena` software development platform (forge)
but open-sourced and published for greater usage by the Rust community.

## Publishing requirements

This crate is planned to be published on [crates.io](https://crates.io). Before that can be done, we need:

- Add typed errors with `thiserror`
- Replace `anyhow::Result` with `std::result::Result`
- Setup CI publishing pipeline

## Limitations

- Merging (`merge_bug`, `merge_identity`) expects the remote commits to already be fetched into the repository, fetching and pushing is left to the caller.
- Bug ref updates are only serialized between users of this crate. A `git` or `git-bug` process updating the same ref at the same time can still have its update dropped, as gix checks the old ref value before taking the ref lock.
- Git writes are not in a transaction, meaning caller errors (e.g Postgres failures) from for example GitArena will still create the git-bug issue

## Example usage

- `tests` - Integration tests testing against real `git-bug` CLI
- `../gitarena` - Git forge implementing `git-bug`-backend storage for issues
