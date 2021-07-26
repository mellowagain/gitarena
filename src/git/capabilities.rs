use crate::git::writer::GitWriter;

use actix_web::web::Bytes;
use anyhow::Result;

// https://git-scm.com/docs/protocol-v2#_capabilities
pub(crate) async fn capabilities(service: &str) -> Result<Bytes> {
    GitWriter::new()
        .write_text(format!("# service={}", service))?

        .flush()?
        .write_text("version 2")?

        .write_text(concat!("agent=git/gitarena-", env!("CARGO_PKG_VERSION")))?
        .write_text("ls-refs")?
        .write_text("unborn")?
        .write_text("fetch=shallow")?
        .write_text("server-option")?
        .write_text("object-format=sha1")?

        .flush()?

        .to_actix()
}
