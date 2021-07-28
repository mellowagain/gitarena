use std::io::Write;

use actix_web::web::{Bytes, BytesMut};
use anyhow::Result;
use git_packetline::Writer as PacketlineWriter;

pub(crate) struct GitWriter {
    inner: PacketlineWriter<Vec<u8>>
}

impl GitWriter {
    pub(crate) fn new() -> GitWriter {
        GitWriter {
            inner: PacketlineWriter::new(Vec::<u8>::new()).text_mode()
        }
    }

    pub(crate) fn write_text<S: AsRef<str>>(&mut self, text: S) -> Result<&mut GitWriter> {
        let str_ref = text.as_ref();

        self.inner.write(str_ref.as_bytes())?;
        Ok(self)
    }

    pub(crate) fn write_text_raw(&mut self, text: &[u8]) -> Result<&mut GitWriter> {
        self.inner.write(text)?;
        Ok(self)
    }

    pub(crate) fn write_binary(&mut self, binary: &[u8]) -> Result<&mut GitWriter> {
        self.inner.enable_binary_mode();
        self.inner.write(binary)?;

        self.inner.enable_text_mode();
        Ok(self)
    }

    pub(crate) fn flush(&mut self) -> Result<&mut GitWriter> {
        self.inner.inner.write(b"0000")?;
        Ok(self)
    }

    pub(crate) fn delimiter(&mut self) -> Result<&mut GitWriter> {
        self.inner.inner.write(b"0001")?;
        Ok(self)
    }

    pub(crate) fn response_end(&mut self) -> Result<&mut GitWriter> {
        self.inner.inner.write(b"0002")?;
        Ok(self)
    }

    pub(crate) fn append(&mut self, other: &mut GitWriter) -> &mut GitWriter {
        self.inner.inner.append(&mut other.inner.inner);
        self
    }

    pub(crate) fn to_actix(&self) -> Result<Bytes> {
        let mut bytes = BytesMut::new();
        bytes.extend(self.inner.inner.iter());

        Ok(bytes.freeze())
    }

}
