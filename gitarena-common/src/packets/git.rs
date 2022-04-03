use serde::{Deserialize, Serialize};
use gitarena_macros::IpcPacket;

#[derive(Deserialize, Serialize, Debug, IpcPacket)]
#[ipc(packet = "Git", id = 1)]
pub struct GitImport {
    pub url: String,
    pub username: Option<String>,
    pub password: Option<String>
}
