use crate::git::io::writer::GitWriter;

use actix_web::web::Bytes;
use anyhow::Result;
use tracing::instrument;

// https://git-scm.com/docs/protocol-v2#_capabilities
#[instrument(err)]
pub(crate) async fn capabilities(service_header: Option<&str>) -> Result<Bytes> {
    let mut writer = GitWriter::new();

    if let Some(service) = service_header {
        writer.write_text(format!("# service={service}")).await?;
        writer.flush().await?;
    }

    writer.write_text("version 2").await?;

    writer.write_text(concat!("agent=git/gitarena-", env!("CARGO_PKG_VERSION"))).await?;
    writer.write_text("ls-refs=unborn").await?;
    writer.write_text("fetch").await?;
    writer.write_text("server-option").await?;
    writer.write_text("object-format=sha1").await?;

    writer.flush().await?;

    writer.serialize().await
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn expected_capabilities() -> String {
        let agent = concat!("agent=git/gitarena-", env!("CARGO_PKG_VERSION"));

        format!(
            "000eversion 2\n{:04x}{agent}\n0013ls-refs=unborn\n000afetch\n0012server-option\n0017object-format=sha1\n0000",
            agent.len() + 5
        )
    }

    #[tokio::test]
    async fn without_service_header_only_capabilities_are_advertised() -> Result<()> {
        let bytes = capabilities(None).await?;

        assert_eq!(String::from_utf8(bytes.to_vec())?, expected_capabilities());
        Ok(())
    }

    #[tokio::test]
    async fn service_header_is_prepended_and_flushed() -> Result<()> {
        let bytes = capabilities(Some("git-upload-pack")).await?;

        assert_eq!(
            String::from_utf8(bytes.to_vec())?,
            format!("001e# service=git-upload-pack\n0000{}", expected_capabilities())
        );
        Ok(())
    }
}
