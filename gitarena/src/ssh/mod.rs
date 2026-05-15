use crate::config::Setting;
use crate::database::Pool;
use crate::ssh::server::SshServer;
use anyhow::{Context, Result};
use gitarena_macros::from_config;
use russh::keys::ssh_encoding::LineEnding;
use russh::keys::{Algorithm, PrivateKey};
use russh::server::{Config, RunningServerHandle, Server};
use russh::{MethodKind, MethodSet, SshId};
use std::borrow::Cow;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::{OnceCell, mpsc};
use tokio::task::JoinHandle;
use tracing::{Instrument, info, info_span, instrument, warn};

pub(crate) mod key;
mod server;

pub(crate) static SSH_TASK_HANDLE: OnceCell<JoinHandle<()>> = OnceCell::const_new();

#[instrument(skip_all)]
pub(crate) async fn init(db_pool: Pool, bind_address: &str) -> Result<Option<RunningServerHandle>> {
    let (enabled, port): (bool, i32) = from_config!(
        "ssh.enabled" => bool,
        "ssh.port" => i32
    );

    let key = {
        let new_key = PrivateKey::random(&mut rand::rng(), Algorithm::Ed25519).context("failed to generate ed25519 private key for server")?;

        let encoded = new_key
            .to_openssh(LineEnding::LF)
            .context("failed to serialize ed25519 private key to string")?;

        let mut tx = db_pool.begin().await?;

        let setting = sqlx::query_as::<_, Setting>("update settings set value = coalesce(value, $1) where key = 'ssh.private_key' returning *")
            .bind(encoded.as_str())
            .fetch_one(&mut *tx)
            .await
            .context("unable to read/write setting ssh.private_key from database")?;

        tx.commit().await?;

        let key = setting.value.expect("the value we just set to be non-null");
        PrivateKey::from_openssh(key.as_bytes()).context("failed to parse ssh private key")?
    };

    if !enabled {
        warn!("SSH server is disabled. Only HTTP(s) will allow Git CLI access.");
        return Ok(None);
    }

    let mut methods = MethodSet::empty();
    methods.push(MethodKind::PublicKey);

    let config = Arc::new(Config {
        server_id: SshId::Standard(Cow::Borrowed(concat!("SSH-2.0-gitarena-v", env!("CARGO_PKG_VERSION")))),
        methods,
        auth_rejection_time: Duration::ZERO, // all public keys of users are public anyway so key existence confidentiality is not broken (because it doesn't exist) by setting this to 0
        keys: vec![key],
        window_size: 16 * 1024 * 1024,  // 16mb
        maximum_packet_size: 32 * 1024, // 32kib (russh warns if > 65535)
        event_buffer_size: 256,
        inactivity_timeout: Some(Duration::from_mins(5)),
        keepalive_interval: Some(Duration::from_secs(10)),
        keepalive_max: 6,
        nodelay: true,
        ..Default::default()
    });

    let address = bind_address
        .rsplit_once(':')
        .map_or_else(|| bind_address.to_string(), |(address, _)| address.to_string());

    let port = u16::try_from(port).context("port needs to be within 0-65535")?;

    let mut server = SshServer { db_pool };

    let socket = TcpListener::bind((address.as_str(), port))
        .await
        .with_context(|| format!("failed to bind ssh server to {address}:{port}"))?;

    let (tx, mut rx) = mpsc::channel(1);

    SSH_TASK_HANDLE
        .set(tokio::spawn(
            async move {
                let server = server.run_on_socket(config, &socket);

                tx.send(server.handle()).await.expect("to be able to send the server handle over a channel");

                info!("running ssh server on {address}:{port}");
                server.await.expect("to be able to run the ssh server");
            }
            .instrument(info_span!("ssh")),
        ))
        .context("failed to set ssh task handle")?;

    let handle = rx.recv().await;
    Ok(handle)
}

#[instrument(skip_all)]
pub(crate) fn destroy(handle: Option<RunningServerHandle>) {
    if let Some(handle) = handle {
        handle.shutdown("instance is shutting down".to_string());
        info!("ssh server shutting down gracefully");
    }
}
