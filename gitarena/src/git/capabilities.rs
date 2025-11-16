use crate::git::io::writer::GitWriter;

use actix_web::web::Bytes;
use anyhow::Result;
use tracing::instrument;

// https://git-scm.com/docs/protocol-v2#_capabilities
#[instrument(err)]
pub(crate) async fn capabilities(service: &str) -> Result<Bytes> {
    let mut writer = GitWriter::new();

    writer.write_text(format!("# service={}", service)).await?;

    writer.flush().await?;
    writer.write_text("version 2").await?;

    writer.write_text(concat!("agent=git/gitarena-", env!("CARGO_PKG_VERSION"))).await?;
    writer.write_text("ls-refs").await?;
    writer.write_text("unborn").await?;
    writer.write_text("fetch").await?;
    writer.write_text("server-option").await?;
    writer.write_text("object-format=sha1").await?;

    writer.flush().await?;

    writer.serialize().await
}
