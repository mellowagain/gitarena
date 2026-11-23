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

    #[instrument(err, skip(self))]
    pub(crate) async fn write_no_good(&mut self, side_band: bool, target_ref: &str, reason: &str) -> Result<&mut GitWriter> {
        let line = format!("ng {target_ref} {reason}");

        if side_band {
            self.write_text_sideband_pktline(Band::Data, line).await
        } else {
            self.write_text(line).await
        }
    }

    #[instrument(err, skip_all)]
    pub(crate) async fn write_text_bytes(&mut self, text: &[u8]) -> Result<&mut GitWriter> {
        self.inner
            .write(text)
            .await
            .with_context(|| format!("Unable to write text bytes to Git writer: {text:?}"))?;

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
    pub(crate) async fn write_binary_sideband_chunked(&mut self, band: Band, binary: &[u8], max_chunk: usize) -> Result<&mut GitWriter> {
        self.inner.enable_binary_mode();

        for chunk in binary.chunks(max_chunk) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    async fn serialized(writer: GitWriter) -> Result<Vec<u8>> {
        Ok(writer.serialize().await?.to_vec())
    }

    #[tokio::test]
    async fn text_line_gets_length_prefix_and_newline() -> Result<()> {
        let mut writer = GitWriter::new();
        writer.write_text("hello").await?;

        assert_eq!(serialized(writer).await?, b"000ahello\n");
        Ok(())
    }

    #[tokio::test]
    async fn empty_text_line_is_rejected() {
        let mut writer = GitWriter::new();

        assert!(writer.write_text("").await.is_err());
    }

    #[tokio::test]
    async fn text_sideband_prefixes_band_byte() -> Result<()> {
        let mut writer = GitWriter::new();
        writer.write_text_sideband(Band::Progress, "hi").await?;
        writer.write_text_sideband(Band::Error, "no").await?;

        assert_eq!(serialized(writer).await?, b"0008\x02hi\n0008\x03no\n");
        Ok(())
    }

    #[tokio::test]
    async fn text_sideband_pktline_nests_a_pktline_in_band_one() -> Result<()> {
        let mut writer = GitWriter::new();
        writer.write_text_sideband_pktline(Band::Data, "unpack ok").await?;

        assert_eq!(serialized(writer).await?, b"0013\x01000eunpack ok\n");
        Ok(())
    }

    #[tokio::test]
    async fn no_good_without_sideband_is_plain_text() -> Result<()> {
        let mut writer = GitWriter::new();
        writer.write_no_good(false, "refs/heads/main", "push contains secrets").await?;

        assert_eq!(serialized(writer).await?, b"002dng refs/heads/main push contains secrets\n");
        Ok(())
    }

    #[tokio::test]
    async fn no_good_with_sideband_is_nested() -> Result<()> {
        let mut writer = GitWriter::new();
        writer.write_no_good(true, "refs/heads/main", "nope").await?;

        assert_eq!(serialized(writer).await?, b"0021\x01001cng refs/heads/main nope\n");
        Ok(())
    }

    #[tokio::test]
    async fn binary_sideband_has_no_newline_and_restores_text_mode() -> Result<()> {
        let mut writer = GitWriter::new();
        writer.write_binary_sideband(Band::Data, b"PACK").await?;
        writer.write_text("x").await?;

        assert_eq!(serialized(writer).await?, b"0009\x01PACK0006x\n");
        Ok(())
    }

    #[tokio::test]
    async fn binary_sideband_chunked_splits_at_max_packet_size() -> Result<()> {
        let payload = vec![b'a'; 65516];

        let mut writer = GitWriter::new();
        writer.write_binary_sideband_chunked(Band::Data, &payload, 65515).await?;
        writer.write_text("x").await?;

        let bytes = serialized(writer).await?;

        assert_eq!(&bytes[..5], b"fff0\x01");
        assert_eq!(&bytes[65520..65526], b"0006\x01a");
        assert_eq!(&bytes[65526..], b"0006x\n");
        Ok(())
    }

    #[tokio::test]
    async fn binary_sideband_chunked_exact_chunk_is_one_packet() -> Result<()> {
        let payload = vec![b'a'; 65515];

        let mut writer = GitWriter::new();
        writer.write_binary_sideband_chunked(Band::Data, &payload, 65515).await?;

        let bytes = serialized(writer).await?;

        assert_eq!(bytes.len(), 65520);
        assert_eq!(&bytes[..5], b"fff0\x01");
        Ok(())
    }

    #[tokio::test]
    async fn special_packets() -> Result<()> {
        let mut writer = GitWriter::new();
        writer.flush().await?;
        writer.delimiter().await?;
        writer.response_end().await?;

        assert_eq!(serialized(writer).await?, b"000000010002");
        Ok(())
    }

    #[tokio::test]
    async fn flush_sideband_wraps_flush_in_band() -> Result<()> {
        let mut writer = GitWriter::new();
        writer.flush_sideband(Band::Data).await?;

        assert_eq!(serialized(writer).await?, b"0009\x010000");
        Ok(())
    }

    #[tokio::test]
    async fn raw_and_append_are_written_verbatim() -> Result<()> {
        let mut other = GitWriter::new();
        other.write_text("b").await?;

        let mut writer = GitWriter::new();
        writer.write_raw(b"raw").await?;
        writer.append(other).await?;

        assert_eq!(serialized(writer).await?, b"raw0006b\n");
        Ok(())
    }

    #[test]
    fn u16_to_hex_is_lowercase_and_zero_padded() {
        assert_eq!(&u16_to_hex(0), b"0000");
        assert_eq!(&u16_to_hex(0x0a), b"000a");
        assert_eq!(&u16_to_hex(0xfff0), b"fff0");
    }
}
