# GitArena

GitArena is a software development platform with built-in vcs, issue tracking and code review.
It is meant as a lightweight and performant alternative to the likes of 
GitLab and Gitea, built with self-hosting and cross-platform/cross-architecture
support in mind.

## Progress

Currently, GitArena is work in progress and is not yet fully featured.
The basics such as repositories and pushing/pulling as well as accounts
work. Please see the issues tab for features that are still work in progress.

## Building

Latest Rust stable toolchain and compiler is required to be installed.

```
$ cargo build --release
```

Cargo will build all required dependencies as well as GitArena itself.
The resulting binary can be found in `./target/release`.

## Usage

In order to run GitArena, the following environment variable needs to be set:

* `DATABASE_URL`: [Postgres connection string](https://www.postgresql.org/docs/12/libpq-connect.html#id-1.7.3.8.3.6)
* `BIND_ADDRESS`: [Socket address](https://doc.rust-lang.org/nightly/std/net/trait.ToSocketAddrs.html) to bind to, for example `localhost:8080` or `127.0.0.1:80` (Port is required)

After start GitArena will automatically create the required table as defined
in `schema.sql`. Please edit the `settings` table to configure your
GitArena instance. In the future this will be do-able in the web ui.

Afterwards your GitArena instance will be fully set up and you can register
your account. In order to access the admin panel (`/admin`), please set
`admin` on your user account in the `users` table to `true`.

## Screenshots

Repository:

![Repository](https://i.cutegirl.tech/vka53i6m9wnv.png)
