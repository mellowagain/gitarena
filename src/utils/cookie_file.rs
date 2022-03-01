use anyhow::{Context, Result};
use derive_more::Display;
use log::warn;
use magic::Cookie;
use serde::Serialize;

#[derive(Debug, Display, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum FileType {
    Text,
    Binary,
    Unknown(String)
}

pub(crate) trait CookieExtensions {
    fn probe(&self, buffer: &[u8]) -> Result<FileType>;
}

impl CookieExtensions for Cookie {
    fn probe(&self, buffer: &[u8]) -> Result<FileType> {
        let output = self.buffer(buffer).context("Failed to find type of buffer")?;

        Ok(if output.contains("ASCII text") {
            FileType::Text
        } else if output == "data" {
            FileType::Binary
        } else {
            let header = buffer.iter().take(5).copied().collect::<Vec<u8>>();
            let hex_str = hex::encode(header);

            warn!("Received unknown output from libmagic (0x{}): {}", hex_str, output);

            FileType::Unknown(hex_str)
        })
    }
}
