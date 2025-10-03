use crate::git::io::band::Band;

use actix_web::web::{Bytes, BytesMut};
use anyhow::{Context, Result};
use futures::AsyncWriteExt;
use git_repository::protocol::transport::packetline::{PacketLineRef, Writer as PacketlineWriter};
use tracing::instrument;
use tracing_unwrap::ResultExt;

pub(crate) struct GitWriter {
    inner: PacketlineWriter<Vec<u8>>,
}

impl GitWriter {
    pub(crate) fn new() -> GitWriter {
        GitWriter {
            inner: PacketlineWriter::new(Vec::<u8>::new()).text_mode(),
        }
    }

    // Example [hexl]text
    pub(crate) async fn write_text<S: AsRef<str>>(&mut self, text: S) -> Result<&mut GitWriter> {
        let str_ref = text.as_ref();

        self.inner
            .write(str_ref.as_bytes())
            .await
            .with_context(|| format!("Unable to write text to Git writer: `{}`", str_ref))?;
        Ok(self)
    }

    // Example: [hexl]\x01text
    pub(crate) async fn write_text_sideband<S: AsRef<str>>(
        &mut self,
        band: Band,
        text: S,
    ) -> Result<&mut GitWriter> {
        let str_ref = text.as_ref();
        let with_band = [band.serialize(), str_ref.as_bytes()].concat();

        self.inner
            .write(with_band.as_slice())
            .await
            .with_context(|| {
                format!(
                    "Unable to write text to sideband {} in Git writer: `{}`",
                    band, str_ref
                )
            })?;
        Ok(self)
    }

    // Example: "[hexl]\x01[hexl]text"
    pub(crate) async fn write_text_sideband_pktline<S: AsRef<str>>(
        &mut self,
        band: Band,
        text: S,
    ) -> Result<&mut GitWriter> {
        let str_ref = text.as_ref();
        let hex_prefix = &u16_to_hex((str_ref.len() + 4 + 1) as u16); // 4 for length, 1 for newline
        let with_band = [band.serialize(), hex_prefix, str_ref.as_bytes()].concat();

        self.inner
            .write(with_band.as_slice())
            .await
            .with_context(|| {
                format!(
                    "Unable to write text to sideband {} in Git writer: `{}`",
                    band, str_ref
                )
            })?;
        Ok(self)
    }

    pub(crate) async fn write_text_bytes(&mut self, text: &[u8]) -> Result<&mut GitWriter> {
        self.inner
            .write(text)
            .await
            .with_context(|| format!("Unable to write text bytes to Git writer: {:?}", text))?;
        Ok(self)
    }

    pub(crate) async fn write_binary(&mut self, binary: &[u8]) -> Result<&mut GitWriter> {
        self.inner.enable_binary_mode();
        self.inner
            .write(binary)
            .await
            .with_context(|| format!("Unable to write binary to Git writer: {:?}", binary))?;

        self.inner.enable_text_mode();
        Ok(self)
    }

    pub(crate) async fn write_binary_sideband(
        &mut self,
        band: Band,
        binary: &[u8],
    ) -> Result<&mut GitWriter> {
        let with_band = [band.serialize(), binary].concat();

        self.inner.enable_binary_mode();
        self.inner
            .write(with_band.as_slice())
            .await
            .with_context(|| {
                format!(
                    "Unable to write binary to sideband {} in Git writer: {:?}",
                    band, binary
                )
            })?;

        self.inner.enable_text_mode();
        Ok(self)
    }

    pub(crate) async fn write_raw(&mut self, binary: &[u8]) -> Result<&mut GitWriter> {
        self.inner
            .inner_mut()
            .write(binary)
            .await
            .with_context(|| format!("Unable to write raw data to Git writer: {:?}", binary))?;
        Ok(self)
    }

    pub(crate) async fn flush(&mut self) -> Result<&mut GitWriter> {
        PacketLineRef::Flush
            .write_to(self.inner.inner_mut())
            .await
            .context("Unable to write flush to Git writer")?;
        Ok(self)
    }

    pub(crate) async fn flush_sideband(&mut self, band: Band) -> Result<&mut GitWriter> {
        let with_band = [band.serialize(), b"0000"].concat();

        self.inner.enable_binary_mode();
        self.inner
            .write(with_band.as_slice())
            .await
            .with_context(|| format!("Unable to write flush to sideband {} in Git writer", band))?;

        self.inner.enable_text_mode();
        Ok(self)
    }

    pub(crate) async fn delimiter(&mut self) -> Result<&mut GitWriter> {
        PacketLineRef::Delimiter
            .write_to(self.inner.inner_mut())
            .await
            .context("Unable to write delimiter to Git writer")?;
        Ok(self)
    }

    pub(crate) async fn response_end(&mut self) -> Result<&mut GitWriter> {
        PacketLineRef::ResponseEnd
            .write_to(self.inner.inner_mut())
            .await
            .context("Unable to write response end to Git writer")?;
        Ok(self)
    }

    pub(crate) async fn append(&mut self, other: GitWriter) -> Result<&mut GitWriter> {
        let serialized = other
            .serialize()
            .await
            .context("Unable to write deserialize Git writer")?;
        self.write_raw(serialized.to_vec().as_slice())
            .await
            .context("Unable to write other Git writer to Git writer")?;

        Ok(self)
    }

    #[instrument(err, skip(self))]
    pub(crate) async fn serialize(self) -> Result<Bytes> {
        let mut bytes = BytesMut::new();
        bytes.extend(self.inner.into_inner().iter());

        Ok(bytes.freeze())
    }
}

fn u16_to_hex(value: u16) -> [u8; 4] {
    let mut buffer = [0u8; 4];
    hex::encode_to_slice((value as u16).to_be_bytes(), &mut buffer).unwrap_or_log();
    buffer
}
