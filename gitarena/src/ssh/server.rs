use crate::git::GitProtocol;
use crate::privileges::privilege;
use crate::repository::{Repository, extract_repo_from_request};
use crate::ssh::key::SshKey;
use crate::user::{User, WebUser};
use anyhow::{Context, Result, anyhow, bail};
use gitarena_common::database::Pool;
use russh::keys::{HashAlg, PublicKey};
use russh::server::{Auth, Handler, Msg, Server, Session};
use russh::{Channel, ChannelId};
use std::net::SocketAddr;
use std::str::FromStr;
use tracing::{debug, error, instrument};

#[derive(Clone, Debug)]
pub(crate) struct SshServer {
    pub(crate) db_pool: Pool,
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

    pub(crate) version: GitProtocol,
}

impl SshHandler {
    fn new(db_pool: Pool) -> Self {
        Self {
            db_pool,
            user: None,
            key: None,
            version: GitProtocol::V1,
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

        debug!(?algorithm, ?fingerprint, "ssh public key login received");

        let mut tx = self.db_pool.begin().await?;

        let Some(key) = SshKey::find(&algorithm, &fingerprint, &mut tx).await? else {
            debug!(?algorithm, ?fingerprint, "received ssh login from unknown public key");
            return Ok(Auth::reject());
        };

        let user = User::find_using_id(key.owner, &mut tx)
            .await
            .expect("existing ssh key to be associated with a existant user");

        tx.commit().await?;

        debug!(?user.id, ?user.username, ?key.id, ?key.title, "ssh public key login succeeded");

        self.user = Some(user);
        self.key = Some(key);

        Ok(Auth::Accept)
    }

    #[instrument(skip(session))]
    async fn channel_eof(&mut self, channel: ChannelId, session: &mut Session) -> Result<(), Self::Error> {
        session.close(channel).context("failed to close ssh channel during eof")?;
        Ok(())
    }

    async fn channel_open_session(&mut self, _channel: Channel<Msg>, _session: &mut Session) -> Result<bool, Self::Error> {
        Ok(true)
    }

    #[instrument(skip(session))]
    async fn data(&mut self, channel: ChannelId, data: &[u8], session: &mut Session) -> Result<(), Self::Error> {
        todo!()
    }

    #[instrument(skip(session))]
    async fn env_request(&mut self, channel: ChannelId, variable_name: &str, variable_value: &str, session: &mut Session) -> Result<(), Self::Error> {
        match variable_name {
            "GIT_PROTOCOL" => {
                let (_, number) = variable_value.split_once('=').ok_or_else(|| anyhow!("received malformed GIT_PROTOCOL"))?;
                self.version = GitProtocol::from_str(number)?;

                session.channel_success(channel)?;
            }
            _ => session.channel_failure(channel)?,
        }

        Ok(())
    }

    #[instrument(skip(session))]
    async fn shell_request(&mut self, channel: ChannelId, session: &mut Session) -> Result<(), Self::Error> {
        let user = self.user.as_ref().expect("shell request to happen after pubkey auth");
        let key = self.key.as_ref().expect("shell request to happen after pubkey auth");

        let message = format!(
            "Hello {}! You've successfully authenticated with your SSH key \"{}\", but GitArena does not provide shell access.\r\n",
            user.username, key.title
        );

        session.channel_success(channel)?;
        session.data(channel, message.into_bytes())?;
        session.close(channel)?;

        debug!(?user.id, ?user.username, ?key.id, ?key.title, "denied ssh shell request");
        Ok(())
    }

    #[instrument(skip(session))]
    async fn exec_request(&mut self, channel: ChannelId, data: &[u8], session: &mut Session) -> Result<(), Self::Error> {
        let Ok(command) = str::from_utf8(data) else {
            session.channel_failure(channel)?;
            session.data(channel, "error: exec request data should be UTF-8")?;
            session.close(channel)?;

            debug!("received non-utf8 exec request: {data:?}");
            return Ok(());
        };

        // git-upload-pack '/user/repo.git'
        // git-receive-pack '/user/repo.git'

        let Some((command, path)) = command.split_once(' ') else {
            session.channel_failure(channel)?;
            session.data(channel, "error: malformed exec request")?;
            session.close(channel)?;

            debug!("received malformatted git exec request: {command}");
            return Ok(());
        };

        let Some((username, repo)) = path
            .strip_prefix('\'')
            .and_then(|p| p.strip_suffix('\''))
            .and_then(|p| p.strip_prefix('/'))
            .and_then(|p| p.strip_suffix(".git"))
            .and_then(|p| p.split_once('/'))
        else {
            session.channel_failure(channel)?;
            session.data(channel, "error: malformed repository name (expected `'/user/repo.git'`)")?;
            session.close(channel)?;

            debug!("received malformatted repository name: {path}");
            return Ok(());
        };

        let Ok(repo) = extract_repo_from_request(&self.db_pool, self.user.as_ref(), username, repo).await else {
            session.channel_failure(channel)?;
            session.data(channel, "error: repository not found")?;
            session.close(channel)?;

            return Ok(());
        };

        match command {
            "git-upload-pack" => {
                todo!()
            }
            "git-receive-pack" => {
                let mut tx = self.db_pool.begin().await?;

                if !privilege::check_push(&repo, self.user.as_ref(), &mut tx).await? {
                    session.channel_failure(channel)?;
                    session.data(channel, "error: repository not found")?;
                    session.close(channel)?;

                    return Ok(());
                }

                tx.commit().await?;

                todo!()
            }
            _ => {
                session.channel_failure(channel)?;
                session.data(channel, format!("error: unknown git command {command}"))?;
                session.close(channel)?;

                debug!("client sent unknown git command: {command}");
                return Ok(());
            }
        }

        Ok(())
    }
}
