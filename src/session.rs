use crate::error::GAErrors::HttpError;
use crate::prelude::*;
use crate::user::User;

use std::error::Error;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::net::Ipv6Addr;
use std::str::FromStr;

use actix_web::HttpRequest;
use anyhow::Result;
use chrono::{DateTime, Local};
use ipnetwork::{IpNetwork, Ipv6Network};
use log::warn;
use serde::Serialize;
use sqlx::{Executor, FromRow, Postgres};
use tracing_unwrap::ResultExt;

#[derive(FromRow, Debug, Serialize)]
pub(crate) struct Session {
    pub(crate) user_id: i32,
    #[serde(skip_serializing)]
    pub(crate) hash: String,
    pub(crate) ip_address: IpNetwork,
    pub(crate) user_agent: String, // TODO: Move this to a dedicated table to prevent duplicates
    created_at: DateTime<Local>,
    pub(crate) updated_at: DateTime<Local>
}

impl Display for Session {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}${}", self.user_id, self.hash)
    }
}

impl Session {
    pub(crate) async fn new<'e, E: Executor<'e, Database = Postgres>>(request: &HttpRequest, user: &User, executor: E) -> Result<Session> {
        let (ip_address, user_agent) = extract_ip_and_ua(request)?;

        // Limit user agent to 256 characters: https://stackoverflow.com/questions/654921/how-big-can-a-user-agent-string-get/654992#comment106798172_654992
        let user_agent = user_agent.chars().take(256).collect::<String>();

        let repo: Session = sqlx::query_as::<_, Session>("insert into sessions (user_id, ip_address, user_agent) values ($1, $2, $3) returning *")
            .bind(&user.id)
            .bind(&ip_address)
            .bind(&user_agent)
            .fetch_one(executor)
            .await?;

        Ok(repo)
    }

    /// Finds existing session from Identity (Display of Session)
    pub(crate) async fn from_identity<'e, E: Executor<'e, Database = Postgres>>(identity: Option<String>, executor: E) -> Result<Option<Session>> {
        match identity {
            Some(identity) => {
                let (user_id_str, hash) = identity.split_once('$').ok_or_else(|| HttpError(500, "Unable to parse identity".to_owned()))?;
                let user_id = user_id_str.parse::<i32>()?;

                let option: Option<Session> = sqlx::query_as::<_, Session>("select * from sessions where user_id = $1 and hash = $2 limit 1")
                    .bind(user_id)
                    .bind(hash)
                    .fetch_optional(executor)
                    .await?;

                Ok(option)
            }
            None => Ok(None)
        }
    }

    pub(crate) async fn update_explicit<'e, E: Executor<'e, Database = Postgres>>(&self, ip_address: &IpNetwork, user_agent: &str, executor: E) -> Result<()> {
        let now = Local::now();

        // Limit user agent to 256 characters: https://stackoverflow.com/questions/654921/how-big-can-a-user-agent-string-get/654992#comment106798172_654992
        let user_agent = user_agent.chars().take(256).collect::<String>();

        sqlx::query("update sessions set ip_address = $1, user_agent = $2, updated_at = $3 where user_id = $4 and hash = $5")
            .bind(&ip_address)
            .bind(&user_agent)
            .bind(&now)
            .bind(&self.user_id)
            .bind(self.hash.as_str())
            .execute(executor)
            .await?;

        Ok(())
    }

    #[allow(dead_code)]
    pub(crate) async fn update_from_request<'e, E: Executor<'e, Database = Postgres>>(&self, request: &HttpRequest, executor: E) -> Result<()> {
        let (ip_address, user_agent) = extract_ip_and_ua(request)?;

        self.update_explicit(&ip_address, user_agent, executor).await
    }

    /// Consumes the current session and destroys it
    pub(crate) async fn destroy<'e, E: Executor<'e, Database = Postgres>>(self, executor: E) -> Result<()> {
        sqlx::query("delete from sessions where user_id = $1 and hash = $2")
            .bind(&self.user_id)
            .bind(self.hash.as_str())
            .execute(executor)
            .await?;

        Ok(())
    }
}

pub(crate) fn extract_ip_and_ua(request: &HttpRequest) -> Result<(IpNetwork, &str)> {
    let ip_address = extract_ip(request)?;
    let user_agent = request.get_header("user-agent").ok_or_else(|| HttpError(500, "No user-agent header in request".to_owned()))?;

    Ok((ip_address, user_agent))
}

pub(crate) fn extract_ip_and_ua_owned(request: HttpRequest) -> Result<(IpNetwork, String)> {
    let ip_address = extract_ip(&request)?;
    let user_agent = request.get_header("user-agent").unwrap_or("No user agent sent");

    Ok((ip_address, user_agent.to_owned()))
}

fn extract_ip(request: &HttpRequest) -> Result<IpNetwork> {
    let connection_info = request.connection_info();
    let ip_str = connection_info.realip_remote_addr().unwrap_or("No user agent sent");

    match IpNetwork::from_str(ip_str) {
        Ok(ip_network) => Ok(ip_network),
        Err(err) => {
            // If we got the local address, it includes the port so try again but with port stripped
            Ok(if let Some((ip, _)) = ip_str.split_once(':') {
                IpNetwork::from_str(ip).unwrap_or_else(|err| default_ip_address(Some(err)))
            } else {
                default_ip_address(Some(err))
            })
        }
    }
}

fn default_ip_address<E: Error>(err: Option<E>) -> IpNetwork {
    if let Some(error) = err {
        warn!("Unable to parse ip address: {}", error);
    }

    // 100::/64 is a valid, reserved black hole IPv6 address block: https://en.wikipedia.org/wiki/Reserved_IP_addresses#IPv6
    const RESERVED_IP: Ipv6Addr = Ipv6Addr::new(0x100, 0, 0, 0, 0, 0, 0, 0);

    Ipv6Network::new(RESERVED_IP, 64).unwrap_or_log().into()
}
