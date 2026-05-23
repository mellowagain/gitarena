# gitarena-issues

Rust library for interacting with `git-bug` issues. Written primarily for usage with the `gitarena` software development platform (forge)
but open-sourced and published for greater usage by the Rust community.

## Limitations

The library can write and read `git-bug` issues and identities, but **cannot yet** do CRDT for allowing offline editing of issues.
If you want this feature, make your voice heard on the issue tracker.

## Example usage

- `src/tests` - Integration tests testing against real `git-bug` CLI
- `../gitarena` - Git forge implementing `git-bug`-backend storage for issues
