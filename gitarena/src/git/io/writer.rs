use crate::git::io::band::Band;

use actix_web::web::{Bytes, BytesMut};
use anyhow::{Context, Result};
use futures::AsyncWriteExt;
use gix::protocol::transport::packetline::PacketLineRef;
use gix::protocol::transport::packetline::async_io::{Writer as PacketlineWriter, encode};
use tracing::instrument;
use tracing_unwrap::ResultExt;

pub(crate) struct GitWriter {
    inner: PacketlineWriter<Vec<u8>>,
}

impl GitWriter {
    pub(crate) fn new() -> GitWriter {
        let mut writer = PacketlineWriter::new(Vec::<u8>::new());
        writer.enable_text_mode();

        GitWriter { inner: writer }
    }

    // Example [hexl]text
    #[instrument(err, skip_all)]
    pub(crate) async fn write_text<S: AsRef<str>>(&mut self, text: S) -> Result<&mut GitWriter> {
        let str_ref = text.as_ref();

        self.inner
            .write(str_ref.as_bytes())
            .await
            .with_context(|| format!("Unable to write text to Git writer: `{str_ref}`"))?;
        Ok(self)
    }

    // Example: [hexl]\x01text
    #[instrument(err, skip(self, text))]
    pub(crate) async fn write_text_sideband<S: AsRef<str>>(&mut self, band: Band, text: S) -> Result<&mut GitWriter> {
        let str_ref = text.as_ref();
        let with_band = [band.serialize(), str_ref.as_bytes()].concat();

        self.inner
            .write(with_band.as_slice())
            .await
            .with_context(|| format!("Unable to write text to sideband {band} in Git writer: `{str_ref}`"))?;
        Ok(self)
    }

    // Example: "[hexl]\x01[hexl]text"
    #[instrument(err, skip(self, text))]
    pub(crate) async fn write_text_sideband_pktline<S: AsRef<str>>(&mut self, band: Band, text: S) -> Result<&mut GitWriter> {
        let str_ref = text.as_ref();
        let hex_prefix = &u16_to_hex(u16::try_from(str_ref.len() + 4 + 1).context("hex prefix larger than u16")?); // 4 for length, 1 for newline
        let with_band = [band.serialize(), hex_prefix, str_ref.as_bytes()].concat();

        self.inner
            .write(with_band.as_slice())
            .await
            .with_context(|| format!("Unable to write text to sideband {band} in Git writer: `{str_ref}`"))?;
        Ok(self)
    }

    #[instrument(err, skip_all)]
    pub(crate) async fn write_text_bytes(&mut self, text: &[u8]) -> Result<&mut GitWriter> {
        self.inner
            .write(text)
            .await
            .with_context(|| format!("Unable to write text bytes to Git writer: {text:?}"))?;
        Ok(self)
    }

    #[instrument(err, skip_all)]
    pub(crate) async fn write_binary(&mut self, binary: &[u8]) -> Result<&mut GitWriter> {
        self.inner.enable_binary_mode();
        self.inner
            .write(binary)
            .await
            .with_context(|| format!("Unable to write binary to Git writer: {binary:?}"))?;

        self.inner.enable_text_mode();
        Ok(self)
    }

    #[instrument(err, skip(self, binary))]
    pub(crate) async fn write_binary_sideband(&mut self, band: Band, binary: &[u8]) -> Result<&mut GitWriter> {
        let with_band = [band.serialize(), binary].concat();

        self.inner.enable_binary_mode();
        self.inner
            .write(with_band.as_slice())
            .await
            .with_context(|| format!("Unable to write binary to sideband {band} in Git writer: {binary:?}"))?;

        self.inner.enable_text_mode();
        Ok(self)
    }

    #[instrument(err, skip(self, binary))]
    pub(crate) async fn write_binary_sideband_chunked(&mut self, band: Band, binary: &[u8]) -> Result<&mut GitWriter> {
        const MAX_CHUNK: usize = 65515;

        self.inner.enable_binary_mode();

        for chunk in binary.chunks(MAX_CHUNK) {
            let with_band = [band.serialize(), chunk].concat();

            self.inner
                .write(with_band.as_slice())
                .await
                .with_context(|| format!("Unable to write binary chunk to sideband {band} in Git writer: {} bytes", chunk.len()))?;
        }

        self.inner.enable_text_mode();
        Ok(self)
    }

    #[instrument(err, skip_all)]
    pub(crate) async fn write_raw(&mut self, binary: &[u8]) -> Result<&mut GitWriter> {
        self.inner
            .inner_mut()
            .write(binary)
            .await
            .with_context(|| format!("Unable to write raw data to Git writer: {binary:?}"))?;

        Ok(self)
    }

    #[instrument(err, skip_all)]
    pub(crate) async fn flush(&mut self) -> Result<&mut GitWriter> {
        encode::write_packet_line(&PacketLineRef::Flush, self.inner.inner_mut())
            .await
            .context("Unable to write flush to Git writer")?;

        Ok(self)
    }

    #[instrument(err, skip(self))]
    pub(crate) async fn flush_sideband(&mut self, band: Band) -> Result<&mut GitWriter> {
        let with_band = [band.serialize(), b"0000"].concat();

        self.inner.enable_binary_mode();
        self.inner
            .write(with_band.as_slice())
            .await
            .with_context(|| format!("Unable to write flush to sideband {band} in Git writer"))?;

        self.inner.enable_text_mode();
        Ok(self)
    }

    #[instrument(err, skip_all)]
    pub(crate) async fn delimiter(&mut self) -> Result<&mut GitWriter> {
        encode::write_packet_line(&PacketLineRef::Delimiter, self.inner.inner_mut())
            .await
            .context("Unable to write delimiter to Git writer")?;

        Ok(self)
    }

    #[instrument(err, skip_all)]
    pub(crate) async fn response_end(&mut self) -> Result<&mut GitWriter> {
        encode::write_packet_line(&PacketLineRef::ResponseEnd, self.inner.inner_mut())
            .await
            .context("Unable to write response end to Git writer")?;

        Ok(self)
    }

    #[instrument(err, skip_all)]
    pub(crate) async fn append(&mut self, other: GitWriter) -> Result<&mut GitWriter> {
        let serialized = other.serialize().await.context("Unable to write deserialize Git writer")?;
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
    hex::encode_to_slice(value.to_be_bytes(), &mut buffer).unwrap_or_log();
    buffer
}
