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

Requirements:

* Latest Rust stable toolchain
* `libmagic`
  * Windows: Please install `libmagic` via `vcpkg` (triplet `x64-windows-static-md`) and set the environment variable `VCPKG_ROOT` to your vcpkg directory ([more information](https://github.com/robo9k/rust-magic-sys#building))
  * macOS: Please install `libmagic` using Homebrew
  * Linux: Please install `libmagic` with your system package manager

Compiling:

```
$ cargo build --release
```

Cargo will build all required dependencies as well as GitArena itself.
The resulting binary can be found in `./target/release`.

## Usage

In order to run GitArena, the following environment variable needs to be set:

* `BIND_ADDRESS`: [Socket address](https://doc.rust-lang.org/nightly/std/net/trait.ToSocketAddrs.html) to bind to, for example `localhost:8080` or `127.0.0.1:80` (Port is required)
* Specify either of these two environment variables:
    * `DATABASE_URL_FILE`: Path to a file containing the [Postgres connection string][postgres]
    * `DATABASE_URL`: Raw [Postgres connection string][postgres]

After start GitArena will automatically create the required table as defined
in `schema.sql` and exit. Please edit the `settings` table to configure your
GitArena instance and start GitArena again. In the future this will be do-able in the web ui.

Afterwards your GitArena instance will be fully set up and you can register
your account. In order to access the admin panel (`/admin`), please set
`admin` on your user account in the `users` table to `true`.

### Logs

By default, GitArena will write logs to a file (instead of the console) when built with `--release`. In order
to view the logs, look for a file in the `logs` directory ending with the current date.

### Optional environment variables

* `MAX_POOL_CONNECTIONS`: Max amount of connections the Postgres connection pool should keep open and ready to use.
* `DATABASE_PASSWORD_FILE`: This environment variable may contain a path to a file containing the Postgres database password. In that case, the password does not need to be specified in the [Postgres connection string][postgres]. This is for usage with Docker secrets.
* `SERVE_STATIC_FILES`: If this environment variable is set, GitArena will serve `/static` resources. This is experimental. It is instead recommended configuring your reverse proxy to serve them.
* `MAGIC`: Path to a [libmagic](https://man7.org/linux/man-pages/man3/libmagic.3.html) file database. If not specified, GitArena will fall back to the generic one shipped with this program.

## Screenshots

Repository:

![Repository](https://i.cutegirl.tech/vka53i6m9wnv.png)

Repository commits:

![Commits](https://i.cutegirl.tech/ed3qdisinquh.png)

File view:

![File](https://i.cutegirl.tech/cjqzyh1lre07.png)

Directory view:

![Directory](https://i.cutegirl.tech/kxgv64zjneoz.png)

Create repository:

![Create](https://i.cutegirl.tech/2xfz586doi0q.png)

Import repository:

![Import](https://i.cutegirl.tech/ya0rkuv0py0c.png)

Login:

![Login](https://i.cutegirl.tech/8biqtc0a7fhi.png)

Sign up:

![Sign up](https://i.cutegirl.tech/xiuba03gdmkv.png)

Explore:

![Explore](https://i.cutegirl.tech/c6uba7e0os35.png)

Admin panel:

![Admin panel](https://i.cutegirl.tech/b5g9vx54fnae.png)

## Sponsors

[![Jetbrains](https://resources.jetbrains.com/storage/products/company/brand/logos/jb_beam.svg)][jetbrains]

[postgres]: https://www.postgresql.org/docs/12/libpq-connect.html#id-1.7.3.8.3.6
[jetbrains]: https://jb.gg/OpenSourceSupport
