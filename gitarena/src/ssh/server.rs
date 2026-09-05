use crate::database::Pool;
use crate::git::GitProtocol;
use crate::git::capabilities::capabilities;
use crate::git::ls_refs::{ls_refs_all, ls_refs_all_upload_pack};
use crate::git::receive_pack::execute_receive_pack;
use crate::git::upload_pack::{execute_upload_pack_v1, execute_upload_pack_v2};
use crate::meili::MeiliClient;
use crate::metrics::git::{OPERATION_COUNT, OPERATION_DURATION};
use crate::privileges::privilege;
use crate::repository::{Repository, extract_repo_from_request};
use crate::ssh::key::SshKey;
use crate::user::User;
use anyhow::{Context, Result, anyhow};
use gix::protocol::transport::packetline::PacketLineRef;
use gix::protocol::transport::packetline::decode::{Stream, streaming};
use opentelemetry::KeyValue;
use russh::keys::{HashAlg, PublicKey};
use russh::server::{Auth, Handler, Msg, Server, Session};
use russh::{Channel, ChannelId};
use std::io;
use std::net::SocketAddr;
use std::str::FromStr;
use std::time::Instant;
use tokio::runtime::Handle;
use tokio::task::spawn_blocking;
use tracing::{debug, error, instrument, warn};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub(crate) struct SshServer {
    pub(crate) db_pool: Pool,
    pub(crate) meili_client: MeiliClient,
}

impl Server for SshServer {
    type Handler = SshHandler;

    fn new_client(&mut self, _peer_addr: Option<SocketAddr>) -> Self::Handler {
        SshHandler::new(self.db_pool.clone(), self.meili_client.clone())
    }

    fn handle_session_error(&mut self, err: <Self::Handler as Handler>::Error) {
        match err.downcast_ref::<russh::Error>() {
            Some(russh::Error::Disconnect) => debug!(?err, "SSH client disconnected from user side"),
            Some(russh::Error::ConnectionTimeout) => debug!(?err, "SSH client connection timed out"),
            Some(russh::Error::KeepaliveTimeout) => debug!(?err, "SSH client connection keep alive timed out"),
            Some(russh::Error::InactivityTimeout) => debug!(?err, "SSH client connection inactivity timed out"),
            Some(russh::Error::IO(io))
                if matches!(
                    io.kind(),
                    io::ErrorKind::UnexpectedEof | io::ErrorKind::ConnectionReset | io::ErrorKind::BrokenPipe
                ) =>
            {
                debug!(?err, "SSH client disconnected early");
            }
            _ => error!(?err, "SSH session error"),
        }
    }
}

#[derive(Debug)]
pub(crate) struct SshHandler {
    db_pool: Pool,
    meili_client: MeiliClient,

    user: Option<User>,
    key: Option<SshKey>,

    version: GitProtocol,

    operation: Option<SshOperation>,
    buffer: Vec<u8>,

    cancelled: bool,
}

impl SshHandler {
    fn new(db_pool: Pool, meili_client: MeiliClient) -> Self {
        Self {
            db_pool,
            meili_client,
            user: None,
            key: None,
            version: GitProtocol::V1,
            operation: None,
            buffer: Vec::new(),
            cancelled: false,
        }
    }
}

impl Handler for SshHandler {
    type Error = anyhow::Error;

    #[instrument]
    async fn auth_publickey_offered(&mut self, user: &str, public_key: &PublicKey) -> Result<Auth, Self::Error> {
        if user != "git" {
            debug!(?user, "received ssh login from unknown user, should be `git`");
            return Ok(Auth::reject());
        }

        let algorithm = public_key.algorithm();
        let fingerprint = public_key.fingerprint(HashAlg::Sha256);

        debug!(?algorithm, ?fingerprint, "ssh public key offered");

        let mut tx = self.db_pool.begin().await?;

        let Some(key) = SshKey::find(&algorithm, &fingerprint, &mut tx).await? else {
            debug!(?algorithm, ?fingerprint, "offered ssh key not found");
            return Ok(Auth::reject());
        };

        let user = User::find_using_id(key.owner, &mut tx)
            .await
            .expect("existing ssh key to be associated with a existant user");

        tx.commit().await?;

        debug!(?user.id, ?user.username, "ssh key accepted, awaiting signature");

        self.user = Some(user);
        self.key = Some(key);
        Ok(Auth::Accept)
    }

    async fn auth_publickey(&mut self, _: &str, _: &PublicKey) -> Result<Auth, Self::Error> {
        // key was already validated in auth_publickey_offered
        if self.user.is_some() && self.key.is_some() {
            Ok(Auth::Accept)
        } else {
            Ok(Auth::reject())
        }
    }

    #[instrument(skip(session))]
    async fn channel_close(&mut self, channel: ChannelId, session: &mut Session) -> Result<(), Self::Error> {
        if self.operation.is_some() {
            debug!("SSH channel closed mid-operation, cancelling");

            self.cancelled = true;
            self.operation = None;
            self.buffer.clear();
        }

        session.close(channel).context("failed to close ssh channel after client close")?;
        Ok(())
    }

    #[instrument(skip(session))]
    async fn channel_eof(&mut self, channel: ChannelId, session: &mut Session) -> Result<(), Self::Error> {
        let Some(operation) = self.operation.take() else {
            session.close(channel).context("failed to close ssh channel during eof")?;
            return Ok(());
        };

        let SshOperation::ReceivePack(repo) = operation else {
            self.buffer.clear();

            session.exit_status_request(channel, 0)?;
            session.close(channel)?;

            return Ok(());
        };

        let vec = self.buffer.clone();

        let db_pool = self.db_pool.clone();
        let meili_client = self.meili_client.clone();

        let actor_id = self.user.as_ref().map(|u| u.id).unwrap_or(Uuid::nil());

        // git objects are not `Send` so they need to be run in a separate task where they don't cross thread boundaries
        let result = spawn_blocking(move || Handle::current().block_on(run_receive_pack(db_pool, &meili_client, repo, vec, actor_id))).await?;

        match result {
            Ok(output) if output.is_empty() => {
                session.exit_status_request(channel, 0)?;
                session.close(channel)?;
            }
            Ok(output) => {
                session.data(channel, output)?;
                session.exit_status_request(channel, 0)?;
                session.close(channel)?;
            }
            Err(err) => {
                warn!(?err, "SSH git operation failed");
                session.channel_failure(channel)?;
                session.exit_status_request(channel, 1)?;
                session.close(channel)?;
            }
        }

        Ok(())
    }

    async fn channel_open_session(&mut self, _channel: Channel<Msg>, _session: &mut Session) -> Result<bool, Self::Error> {
        Ok(true)
    }

    #[instrument(skip(session))]
    async fn data(&mut self, channel: ChannelId, data: &[u8], session: &mut Session) -> Result<(), Self::Error> {
        if self.cancelled {
            return Ok(());
        }

        self.buffer.extend_from_slice(data);

        let Some(SshOperation::UploadPack(repo)) = self.operation.as_ref() else {
            return Ok(());
        };

        let repo = repo.clone();

        while let Some(length) = next_request_length(&self.buffer, self.version)? {
            let request = self.buffer.drain(..length).collect::<Vec<u8>>();

            let db_pool = self.db_pool.clone();
            let repo = repo.clone();
            let version = self.version;

            let result = spawn_blocking(move || Handle::current().block_on(run_upload_pack(db_pool, repo, request, version))).await?;

            match result {
                Ok(output) => session.data(channel, output)?,
                Err(err) => {
                    warn!(?err, "SSH git upload-pack round failed");

                    self.operation = None;
                    self.buffer.clear();

                    session.exit_status_request(channel, 1)?;
                    session.close(channel)?;

                    return Ok(());
                }
            }
        }

        Ok(())
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
            session.exit_status_request(channel, 1)?;
            session.data(channel, "error: exec request data should be UTF-8")?;
            session.close(channel)?;

            debug!("received non-utf8 exec request: {data:?}");
            return Ok(());
        };

        // git-upload-pack '/user/repo.git'
        // git-receive-pack '/user/repo.git'

        let Some((command, path)) = command.split_once(' ') else {
            session.channel_failure(channel)?;
            session.exit_status_request(channel, 1)?;
            session.data(channel, "error: malformed exec request")?;
            session.close(channel)?;

            debug!("received malformatted git exec request: {command}");
            return Ok(());
        };

        let Some((username, repo_name)) = path
            .strip_prefix('\'')
            .and_then(|p| p.strip_suffix('\''))
            .map(|p| p.trim_matches('/'))
            .and_then(|p| p.strip_suffix(".git"))
            .and_then(|p| p.split_once('/'))
        else {
            session.channel_failure(channel)?;
            session.exit_status_request(channel, 1)?;
            session.data(channel, "error: malformed repository name (expected `'/user/repo.git'`)")?;
            session.close(channel)?;

            debug!("received malformatted repository name: {path}");
            return Ok(());
        };

        let Ok(repo) = extract_repo_from_request(&self.db_pool, self.user.as_ref(), username, repo_name).await else {
            session.channel_failure(channel)?;
            session.exit_status_request(channel, 1)?;
            session.data(channel, "error: repository not found")?;
            session.close(channel)?;

            return Ok(());
        };

        match command {
            "git-upload-pack" => {
                let db_pool = self.db_pool.clone();
                let version = self.version;
                let repo_clone = repo.clone();

                let advert = spawn_blocking(move || {
                    Handle::current().block_on(async move {
                        let mut tx = db_pool.begin().await?;

                        let git2repo = repo_clone.libgit2(&mut tx).await?;

                        tx.commit().await?;

                        match version {
                            GitProtocol::V2 => capabilities(None).await,
                            GitProtocol::V1 => ls_refs_all_upload_pack(&git2repo, None).await,
                        }
                    })
                })
                .await??;

                self.operation = Some(SshOperation::UploadPack(repo));
                self.buffer = Vec::new();

                session.data(channel, advert.to_vec())?;
                session.channel_success(channel)?;
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

                let db_pool = self.db_pool.clone();
                let repo_clone = repo.clone();

                let advert = spawn_blocking(move || {
                    Handle::current().block_on(async move {
                        let mut tx = db_pool.begin().await?;

                        let git2repo = repo_clone.libgit2(&mut tx).await?;

                        tx.commit().await?;

                        ls_refs_all(&git2repo, None).await
                    })
                })
                .await??;

                self.operation = Some(SshOperation::ReceivePack(repo));
                self.buffer = Vec::new();

                session.data(channel, advert.to_vec())?;
                session.channel_success(channel)?;
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

    #[instrument(skip(session))]
    async fn signal(&mut self, channel: ChannelId, signal: russh::Sig, session: &mut Session) -> Result<(), Self::Error> {
        debug!(?signal, "SSH client sent signal, cancelling operation");

        self.cancelled = true;
        self.operation = None;
        self.buffer.clear();

        session.close(channel).context("failed to close ssh channel after signal")?;
        Ok(())
    }
}

#[derive(Debug)]
enum SshOperation {
    UploadPack(Repository),
    ReceivePack(Repository),
}

fn next_request_length(buffer: &[u8], version: GitProtocol) -> Result<Option<usize>> {
    let mut offset = 0;

    loop {
        let (line, consumed) = match streaming(&buffer[offset..])? {
            Stream::Complete { line, bytes_consumed } => (line, bytes_consumed),
            Stream::Incomplete { .. } => return Ok(None),
        };

        offset += consumed;

        match (line, version) {
            (PacketLineRef::Flush, GitProtocol::V2) => return Ok(Some(offset)),
            (PacketLineRef::Data(data), GitProtocol::V1) if data.trim_ascii_end() == b"done" => return Ok(Some(offset)),
            _ => {}
        }
    }
}

#[instrument(err, skip(db_pool, vec))]
async fn run_upload_pack(db_pool: Pool, repo: Repository, vec: Vec<u8>, version: GitProtocol) -> Result<Vec<u8>> {
    let mut tx = db_pool.begin().await?;
    let git2repo = repo.libgit2(&mut tx).await?;

    let start = Instant::now();

    let output = match version {
        GitProtocol::V2 => execute_upload_pack_v2(vec.as_slice(), &git2repo).await?,
        GitProtocol::V1 => execute_upload_pack_v1(vec.as_slice(), &git2repo).await?,
    };

    let elapsed = start.elapsed().as_secs_f64();

    OPERATION_COUNT.add(
        1,
        &[
            KeyValue::new("operation", "upload-pack"),
            KeyValue::new("transport", "ssh"),
            KeyValue::new("status", "ok"),
        ],
    );

    OPERATION_DURATION.record(elapsed, &[KeyValue::new("operation", "upload-pack"), KeyValue::new("transport", "ssh")]);

    Ok(output)
}

#[instrument(err, skip(db_pool, vec))]
async fn run_receive_pack(db_pool: Pool, meili_client: &MeiliClient, mut repo: Repository, vec: Vec<u8>, actor_id: Uuid) -> Result<Vec<u8>> {
    let start = Instant::now();

    let output_writer = execute_receive_pack(&db_pool, meili_client, &mut repo, vec.as_slice(), actor_id, None).await?;
    let output = output_writer.serialize().await.map(|b| b.to_vec())?;

    let elapsed = start.elapsed().as_secs_f64();

    OPERATION_COUNT.add(
        1,
        &[
            KeyValue::new("operation", "push"),
            KeyValue::new("transport", "ssh"),
            KeyValue::new("status", "ok"),
        ],
    );

    OPERATION_DURATION.record(elapsed, &[KeyValue::new("operation", "push"), KeyValue::new("transport", "ssh")]);

    Ok(output)
}
