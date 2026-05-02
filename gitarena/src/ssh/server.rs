use crate::ssh::key::SshKey;
use crate::user::User;
use anyhow::{Result, anyhow};
use gitarena_common::database::Pool;
use russh::keys::{HashAlg, PublicKey};
use russh::server::{Auth, Handler, Msg, Server, Session};
use russh::{Channel, ChannelId};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, instrument};

#[derive(Clone, Debug)]
pub(crate) struct SshServer {
    pub(crate) db_pool: Pool
}

impl Server for SshServer {
    type Handler = SshHandler;

    fn new_client(&mut self, _peer_addr: Option<SocketAddr>) -> Self::Handler {
        SshHandler::new(self.db_pool.clone())
    }

    fn handle_session_error(&mut self, err: <Self::Handler as Handler>::Error) {
        error!(?err, "SSH session error");
    }
}

#[derive(Clone, Debug)]
pub(crate) struct SshHandler {
    pub(crate) db_pool: Pool,

    pub(super) user: Option<User>,
    pub(crate) key: Option<SshKey>,
}

impl SshHandler {
    fn new(db_pool: Pool) -> Self {
        Self {
            db_pool,
            user: None,
            key: None,
        }
    }
}

impl Handler for SshHandler {
    type Error = anyhow::Error;

    #[instrument]
    async fn auth_publickey(&mut self, user: &str, public_key: &PublicKey) -> Result<Auth, Self::Error> {
        if user != "git" {
            debug!(?user, "received ssh login from unknown user, should be `git`");
            return Ok(Auth::reject());
        }

        let algorithm = public_key.algorithm();
        let fingerprint = public_key.fingerprint(HashAlg::Sha256);

        debug!(?algorithm, ?fingerprint, "ssh public key login");

        let mut tx = self.db_pool.begin().await?;

        let Some(key) = SshKey::find(&algorithm, &fingerprint, &mut tx).await? else {
            debug!(?algorithm, ?fingerprint, "received ssh login from unknown public key");
            return Ok(Auth::reject());
        };

        let user = User::find_using_id(key.owner, &mut tx)
            .await
            .expect("existing ssh key to be associated with a existant user");

        tx.commit().await?;

        debug!(?user.id, ?user.username, "received ssh login");

        self.user = Some(user);
        self.key = Some(key);

        Ok(Auth::Accept)
    }

    async fn channel_eof(&mut self, channel: ChannelId, session: &mut Session) -> Result<(), Self::Error> {
        session.close(channel)?;
        Ok(())
    }

    async fn channel_open_session(&mut self, channel: Channel<Msg>, session: &mut Session) -> Result<bool, Self::Error> {
        Ok(true)
    }

    #[instrument]
    async fn shell_request(&mut self, channel: ChannelId, session: &mut Session) -> Result<(), Self::Error> {
        let user = self.user.as_ref().ok_or_else(|| anyhow!("no user associated with ssh connection"))?;
        let key = self.key.as_ref().ok_or_else(|| anyhow!("no ssh key associated with ssh connection"))?;

        let message = format!(
            "Hello {}! You've successfully authenticated with your SSH key \"{}\", but GitArena does not provide shell access.\r\n",
            user.username, key.title
        );

        session.channel_success(channel)?;
        session.data(channel, message.into_bytes())?;
        session.close(channel)?;

        debug!(?user.id, ?user.username, ?key.id, "denied ssh shell request");
        Ok(())
    }
}
