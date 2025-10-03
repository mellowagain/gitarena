use gitarena_macros::IpcPacket;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Default, IpcPacket)]
#[ipc(packet = "Git", id = 1)] // = 1001
pub struct GitImport {
    pub url: String,
    pub username: Option<String>,
    pub password: Option<String>,
}
