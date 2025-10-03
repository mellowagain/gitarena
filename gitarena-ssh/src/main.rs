use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use gitarena_common::database::create_postgres_pool;
use gitarena_common::prelude::*;

mod keys;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Args = Args::try_parse()?;

    let db_pool = create_postgres_pool("gitarena-ssh", Some(1)).await?;
    let mut transaction = db_pool.begin().await?;

    // Subcommand execution
    use Command::*;

    match &args.command {
        Some(AuthorizedKeys) => keys::print_all(&mut transaction).await?,
        _ => bail!("GitArena does currently not provide SSH access"),
    }

    transaction.commit().await?;
    Ok(())
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Prints out all non-expired SSH keys added by all GitArena users.
    /// This command should be invoked by the OpenSSH server via [`AuthorizedKeysCommand`](https://man.openbsd.org/sshd_config#AuthorizedKeysCommand)
    AuthorizedKeys,
}

#[derive(Parser, Debug)]
#[clap(
    author,
    version,
    about = "SSH component for GitArena",
    long_about = "SSH component for GitArena: a software development platform with built-in vcs, issue tracking and code review"
)]
struct Args {
    user: Option<String>,

    #[clap(subcommand)]
    command: Option<Command>,
}
