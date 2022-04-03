use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use futures_locks::RwLock;
use gitarena_common::ipc::{ipc_path, IpcPacket, PacketId};
use log::{debug, error, info, warn};
use parity_tokio_ipc::{Connection, Endpoint};
use serde::Serialize;
use tokio::io::AsyncWriteExt;
use tracing_unwrap::ResultExt;

pub(crate) struct Ipc {
    connection: Option<Connection>
}

impl Ipc {
    pub(crate) async fn new() -> Result<Self> {
        let ipc_path = ipc_path()?;

        let connection = match Ipc::connect().await {
            Ok(connection) => {
                info!("Successfully connected to workhorse at {}", ipc_path);
                Some(connection)
            }
            Err(err) => {
                error!("Failed to connect to workhorse: {}", err);
                warn!("Workhorse features such as repo importing will be unavailable until IPC connection is established");

                None
            }
        };

        Ok(Self {
            connection
        })
    }

    pub(crate) async fn connect() -> Result<Connection> {
        let ipc_path = ipc_path()?;

        Ok(Endpoint::connect(ipc_path).await?)
    }

    pub(crate) async fn send<T: Serialize + Sized + PacketId>(&mut self, packet: T) -> Result<()> {
        let packet = IpcPacket::new(packet);
        let bytes = packet.serialize().context("Failed to serialize packet")?;

        self.connection.as_mut()
            .ok_or_else(|| anyhow!("Not connected to workhorse"))?
            .write_all(bytes.as_slice())
            .await
            .context("Failed to send packet to workhorse")
    }

    pub(crate) fn is_connected(&self) -> bool {
        self.connection.is_some()
    }
}

pub(crate) fn spawn_connection_task(data: RwLock<Ipc>) {
    let mut interval = tokio::time::interval(Duration::new(60, 0));

    tokio::spawn(async move {
        loop {
            interval.tick().await;

            debug!("Trying to re-establish connection to workhorse");

            match Ipc::connect().await {
                Ok(connection) => {
                    // unwrap_or_log() is safe because Ipc::connect (above) calls it as well, thus we never would be Ok if it wasn't also Ok here
                    let ipc_path = ipc_path().unwrap_or_log();

                    data.write().await.connection = Some(connection);

                    info!("Successfully connected to workhorse at {}", ipc_path);
                    break;
                }
                Err(err) => debug!("Failed to re-establish connection to workhorse, retrying in 60 seconds: {}", err)
            }
        }
    });
}
