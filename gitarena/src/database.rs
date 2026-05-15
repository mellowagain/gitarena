use std::env;
use std::env::VarError;
use std::str::FromStr;
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use once_cell::sync::OnceCell;
use sqlx::{Executor, Postgres};
use tokio::fs;
use tracing::{info, instrument};

use sqlx::postgres::PgConnectOptions;

pub type ConnectOptions = PgConnectOptions;
pub type Database = Postgres;

pub type Pool = sqlx::Pool<Database>;
pub type PoolOptions = sqlx::pool::PoolOptions<Database>;

#[instrument(err)]
pub async fn create_postgres_pool(max_conns: Option<u32>) -> Result<Pool> {
    static ONCE: OnceCell<String> = OnceCell::new();

    let pool = PoolOptions::new()
        .max_connections(max_conns.ok_or(()).or_else(|()| get_max_connections())?)
        .acquire_timeout(Duration::from_secs(10))
        .after_connect(move |connection, _meta| {
            Box::pin(async move {
                // If setting the app name fails it's not a big deal if the connection is still fine so let's ignore the error
                let _ = connection
                    .execute(ONCE.get_or_init(|| "set application_name = 'gitarena';".to_string()).as_str())
                    .await;

                info!("Successfully connected to database");
                Ok(())
            })
        })
        .connect_with(read_database_config().await?)
        .await?;

    sqlx::migrate!("../migrations").run(&pool).await?;
    Ok(pool)
}

#[instrument(err)]
async fn read_database_config() -> Result<ConnectOptions> {
    let mut options = match (env::var_os("DATABASE_URL"), env::var_os("DATABASE_URL_FILE")) {
        (Some(url), None) => {
            let str = url
                .into_string()
                .map_err(|_| anyhow!("`DATABASE_URL` environment variable is not valid unicode"))?;
            ConnectOptions::from_str(str.as_str())?
        }
        (None, Some(file)) => {
            let url = fs::read_to_string(file).await?;
            ConnectOptions::from_str(url.as_str())?
        }
        _ => bail!("Either environment variable `DATABASE_URL` or `DATABASE_URL_FILE` needs to be specified to before starting GitArena"),
    };

    // Docker secrets compatibility
    match env::var("DATABASE_PASSWORD_FILE") {
        Ok(file) => {
            let password = fs::read_to_string(file).await?;
            options = options.password(password.as_str());
        }
        Err(VarError::NotUnicode(_)) => {
            bail!("`DATABASE_PASSWORD_FILE` environment variable is not valid unicode")
        }
        Err(VarError::NotPresent) => { /* No password auth required, or it was already set in the connection string; safe to ignore */ }
    }

    Ok(options)
}

fn get_max_connections() -> Result<u32> {
    Ok(match env::var("MAX_POOL_CONNECTIONS") {
        Ok(env_str) => env_str
            .parse::<u32>()
            .context("Unable to parse MAX_POOL_CONNECTIONS environment variable into a u32")?,
        Err(VarError::NotPresent) => 5.max(u32::try_from(num_cpus::get()).context("too many cpu threads to fit into u32")?),
        Err(VarError::NotUnicode(_)) => {
            bail!("MAX_POOL_CONNECTIONS environment variable is not a valid unicode string")
        }
    })
}
