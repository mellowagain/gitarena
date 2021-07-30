use actix_web::web::{Bytes, BytesMut};
use anyhow::Result;
use futures::AsyncWriteExt;
use git_packetline::{PacketLine, Writer as PacketlineWriter};

pub(crate) struct GitWriter {
    inner: PacketlineWriter<Vec<u8>>
}

impl GitWriter {
    pub(crate) fn new() -> GitWriter {
        GitWriter {
            inner: PacketlineWriter::new(Vec::<u8>::new()).text_mode()
        }
    }

    pub(crate) async fn write_text<S: AsRef<str>>(&mut self, text: S) -> Result<&mut GitWriter> {
        let str_ref = text.as_ref();

        self.inner.write(str_ref.as_bytes()).await?;
        Ok(self)
    }

    pub(crate) async fn write_text_bytes(&mut self, text: &[u8]) -> Result<&mut GitWriter> {
        self.inner.write(text).await?;
        Ok(self)
    }

    pub(crate) async fn write_binary(&mut self, binary: &[u8]) -> Result<&mut GitWriter> {
        self.inner.enable_binary_mode();
        self.inner.write(binary).await?;

        self.inner.enable_text_mode();
        Ok(self)
    }

    pub(crate) async fn write_raw(&mut self, binary: &[u8]) -> Result<&mut GitWriter> {
        self.inner.inner_mut().write(binary).await?;
        Ok(self)
    }

    pub(crate) async fn flush(&mut self) -> Result<&mut GitWriter> {
        PacketLine::Flush.write_to(self.inner.inner_mut()).await?;
        Ok(self)
    }

    pub(crate) async fn delimiter(&mut self) -> Result<&mut GitWriter> {
        PacketLine::Delimiter.write_to(self.inner.inner_mut()).await?;
        Ok(self)
    }

    pub(crate) async fn response_end(&mut self) -> Result<&mut GitWriter> {
        PacketLine::ResponseEnd.write_to(self.inner.inner_mut()).await?;
        Ok(self)
    }

    pub(crate) async fn append(&mut self, other: GitWriter) -> Result<&mut GitWriter> {
        let serialized = other.serialize().await?;
        self.write_raw(serialized.to_vec().as_slice()).await?;

        Ok(self)
    }

    pub(crate) async fn serialize(self) -> Result<Bytes> {
        let mut bytes = BytesMut::new();
        bytes.extend(self.inner.into_inner().iter());

        Ok(bytes.freeze())
    }
}
