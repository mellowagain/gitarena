# gitarena-issues

Rust library for interacting with `git-bug` issues. Written primarily for usage with the `gitarena` software development platform (forge)
but open-sourced and published for greater usage by the Rust community.

## Publishing requirements

This crate is planned to be published on [crates.io](https://crates.io). Before that can be done, we need:

- Add typed errors with `thiserror`
- Replace `anyhow::Result` with `std::result::Result`
- Setup CI publishing pipeline

## Limitations

- The library can write and read `git-bug` issues and identities, but **cannot yet** do CRDT for allowing offline editing of issues.
- Git writes are not in a transaction, meaning caller errors (e.g Postgres failures) from for example GitArena will still create the git-bug issue

## Example usage

- `src/tests` - Integration tests testing against real `git-bug` CLI
- `../gitarena` - Git forge implementing `git-bug`-backend storage for issues
