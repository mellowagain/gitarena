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
    Unknown
}

pub(crate) trait CookieExtensions {
    fn probe(&self, buffer: &[u8]) -> Result<FileType>;
}

impl CookieExtensions for Cookie {
    fn probe(&self, buffer: &[u8]) -> Result<FileType> {
        let output = self.buffer(buffer).context("Failed to find type of buffer")?;

        Ok(match output.as_str() {
            "ASCII text" => FileType::Text,
            "data" => FileType::Binary,
            _ => {
                warn!("Received unknown output from libmagic: {}", output);

                FileType::Unknown
            }
        })
    }
}
