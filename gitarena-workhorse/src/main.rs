use std::io;

use anyhow::{Context, Result};
use futures::stream::StreamExt;
use gitarena_common::ipc::{ipc_path, IpcPacket};
use gitarena_common::log::init_logger;
use gitarena_common::num_traits::cast::FromPrimitive;
use gitarena_common::packets::git::GitImport;
use gitarena_common::packets::PacketId;
use log::{debug, error, info, warn};
use parity_tokio_ipc::Endpoint;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite};
use tracing_unwrap::ResultExt;

#[tokio::main]
async fn main() -> Result<()> {
    let _log_guards = init_logger("gitarena-workhorse", &[])?;

    Endpoint::new(ipc_path()?.to_owned())
        .incoming()
        .with_context(|| format!("Failed to create endpoint at {}", ipc_path().unwrap_or_log()))? // .unwrap_or_log() is safe as it would've excited early two lines above if this errors
        .for_each(|connection| async {
            if let Err(err) = handle(connection).await {
                error!("Error occurred while reading stream: {}", err);
            }
        })
        .await;

    info!("Thank you and goodbye.");

    Ok(())
}

async fn handle<T: AsyncRead + AsyncWrite + Unpin + 'static>(connection: Result<T, io::Error>) -> Result<()> {
    let mut connection = connection?;

    let type_ = connection.read_u64().await.context("Failed to read type")?;
    let length = connection.read_u64().await.context("Failed to read length")?;

    let id: PacketId = PacketId::from_u64(type_).with_context(|| format!("Received unknown packet id: {}", type_))?;

    let mut buffer: Vec<u8> = Vec::with_capacity(length as usize);
    let read_length = connection.read(buffer.as_mut_slice()).await.context("Failed to read payload")? as u64;

    if read_length != length {
        warn!("Failed to read correct payload size, expected: {} read: {}", length, read_length);
    }

    // Bincode is configured in gitarena-common/src/ipc.rs to use little endianness
    let ser_type = &type_.to_le_bytes();
    let ser_length = &length.to_le_bytes();
    let payload = &[ser_type, ser_length, buffer.as_slice()].concat();

    match id {
        PacketId::GitImport => {
            let packet = IpcPacket::<GitImport>::deserialize(payload)?;

            debug!("got {:?}", packet);
        }
    }

    Ok(())
}
