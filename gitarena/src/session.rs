use crate::prelude::HttpRequestExtensions;
use crate::user::User;

use std::error::Error;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::net::Ipv6Addr;
use std::str::FromStr;

use actix_web::HttpRequest;
use anyhow::{Result, anyhow};
use chrono::{DateTime, Local};
use gitarena_common::database::Database;
use ipnetwork::{IpNetwork, Ipv6Network};
use serde::Serialize;
use sqlx::{FromRow, Transaction};
use tracing::{instrument, warn};
use tracing_unwrap::ResultExt;
use uuid::Uuid;

#[derive(FromRow, Debug, Serialize)]
pub(crate) struct Session {
    pub(crate) user_id: Uuid,
    #[serde(skip_serializing)]
    pub(crate) hash: String,
    pub(crate) ip_address: IpNetwork,
    pub(crate) user_agent: String, // TODO: Move this to a dedicated table to prevent duplicates
    pub(crate) updated_at: DateTime<Local>,
}

impl Display for Session {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}${}", self.user_id, self.hash)
    }
}

impl Session {
    #[instrument(err, skip(request, tx))]
    pub(crate) async fn new(request: &HttpRequest, user: &User, tx: &mut Transaction<'_, Database>) -> Result<Session> {
        let (ip_address, user_agent) = extract_ip_and_ua(request);

        // Limit user agent to 256 characters: https://stackoverflow.com/questions/654921/how-big-can-a-user-agent-string-get/654992#comment106798172_654992
        let user_agent = user_agent.chars().take(256).collect::<String>();

        let repo: Session = sqlx::query_as::<_, Session>("insert into sessions (user_id, ip_address, user_agent) values ($1, $2, $3) returning *")
            .bind(user.id)
            .bind(ip_address)
            .bind(&user_agent)
            .fetch_one(&mut **tx)
            .await?;

        Ok(repo)
    }

    /// Finds existing session from Identity (Display of Session)
    #[instrument(err, skip(tx))]
    pub(crate) async fn from_identity(identity: Option<String>, tx: &mut Transaction<'_, Database>) -> Result<Option<Session>> {
        match identity {
            Some(identity) => {
                let (user_id_str, hash) = identity.split_once('$').ok_or_else(|| anyhow!("Unable to parse identity"))?;
                let user_id = user_id_str.parse::<Uuid>()?;

                let option: Option<Session> = sqlx::query_as::<_, Session>("select * from sessions where user_id = $1 and hash = $2 limit 1")
                    .bind(user_id)
                    .bind(hash)
                    .fetch_optional(&mut **tx)
                    .await?;

                Ok(option)
            }
            None => Ok(None),
        }
    }

    #[instrument(err, skip(tx))]
    pub(crate) async fn update_explicit(&self, ip_address: &IpNetwork, user_agent: &str, tx: &mut Transaction<'_, Database>) -> Result<()> {
        let now = Local::now();

        // Limit user agent to 256 characters: https://stackoverflow.com/questions/654921/how-big-can-a-user-agent-string-get/654992#comment106798172_654992
        let user_agent = user_agent.chars().take(256).collect::<String>();

        sqlx::query("update sessions set ip_address = $1, user_agent = $2, updated_at = $3 where user_id = $4 and hash = $5")
            .bind(ip_address)
            .bind(&user_agent)
            .bind(now)
            .bind(self.user_id)
            .bind(self.hash.as_str())
            .execute(&mut **tx)
            .await?;

        Ok(())
    }

    #[instrument(err, skip(tx))]
    pub(crate) async fn update_from_request(&self, request: &HttpRequest, tx: &mut Transaction<'_, Database>) -> Result<()> {
        let (ip_address, user_agent) = extract_ip_and_ua(request);

        self.update_explicit(&ip_address, user_agent, tx).await
    }

    /// Consumes the current session and destroys it
    #[instrument(err, skip(tx))]
    pub(crate) async fn destroy(self, tx: &mut Transaction<'_, Database>) -> Result<()> {
        sqlx::query("delete from sessions where user_id = $1 and hash = $2")
            .bind(self.user_id)
            .bind(self.hash.as_str())
            .execute(&mut **tx)
            .await?;

        Ok(())
    }
}

pub(crate) fn extract_ip_and_ua(request: &HttpRequest) -> (IpNetwork, &str) {
    let ip_address = extract_ip(request);
    let user_agent = request.get_header("user-agent").unwrap_or_default();

    (ip_address, user_agent)
}

pub(crate) fn extract_ip_and_ua_owned(request: &HttpRequest) -> (IpNetwork, String) {
    let ip_address = extract_ip(request);
    let user_agent = request.get_header("user-agent").unwrap_or_default();

    (ip_address, user_agent.to_owned())
}

fn extract_ip(request: &HttpRequest) -> IpNetwork {
    let connection_info = request.connection_info();
    let ip_str = connection_info.realip_remote_addr().unwrap_or("No user agent sent");

    match IpNetwork::from_str(ip_str) {
        Ok(ip_network) => ip_network,
        Err(err) => {
            // If we got the local address, it includes the port so try again but with port stripped
            if let Some((ip, _)) = ip_str.split_once(':') {
                IpNetwork::from_str(ip).unwrap_or_else(|err| default_ip_address(Some(err)))
            } else {
                default_ip_address(Some(err))
            }
        }
    }
}

fn default_ip_address<E: Error>(err: Option<E>) -> IpNetwork {
    if let Some(error) = err {
        warn!(err = ?error, "Unable to parse ip address");
    }

    // 100::/64 is a valid, reserved black hole IPv6 address block: https://en.wikipedia.org/wiki/Reserved_IP_addresses#IPv6
    const RESERVED_IP: Ipv6Addr = Ipv6Addr::new(0x100, 0, 0, 0, 0, 0, 0, 0);

    Ipv6Network::new(RESERVED_IP, 64).unwrap_or_log().into()
}
