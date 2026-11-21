use anyhow::{Context, Result, bail};
use gix::protocol::transport::packetline::PacketLineRef;
use gix::protocol::transport::packetline::async_io::StreamingPeekableIter;
use std::io::ErrorKind;
use tracing::instrument;
use tracing::warn;
use tracing_unwrap::OptionExt;

#[instrument(err)]
pub(crate) async fn read_until_command(mut body: Vec<Vec<u8>>) -> Result<(String, Vec<Vec<u8>>)> {
    for (index, raw_line) in body.iter().enumerate() {
        match String::from_utf8(raw_line.clone()) {
            Ok(line) => {
                if let Some((key, value)) = line.split_once('=') {
                    if key != "command" {
                        continue;
                    }

                    for i in 0..index {
                        body.remove(i);
                    }

                    return Ok((value.to_owned(), body));
                }
            }
            Err(err) => {
                warn!(?err, "Failed to read line into UTF-8 vec");
            }
        }
    }

    bail!("Unable to parse Git request body")
}

#[instrument(err, skip(iter))]
pub(crate) async fn read_data_lines(iter: &mut StreamingPeekableIter<&[u8]>) -> Result<Vec<Vec<u8>>> {
    let mut body = Vec::<Vec<u8>>::new();

    while let Some(line_result) = iter.read_line().await {
        match line_result {
            Ok(Ok(line)) => {
                if let PacketLineRef::Data(data) = line {
                    if data.is_empty() {
                        continue;
                    }

                    // We can safely unwrap() as we checked above that the slice is not empty
                    let length = data.len() - usize::from(data.last().unwrap_or_log() == &10_u8);

                    body.push(data[..length].to_vec());
                }
            }
            Ok(Err(err)) => return Err(err).context("failed to read git data line"),
            Err(err) if err.kind() == ErrorKind::UnexpectedEof => break,
            Err(err) => return Err(err.into()),
        }
    }

    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use gix::protocol::transport::packetline::PacketLineRef;
    use pretty_assertions::assert_eq;

    fn lines(input: &[&str]) -> Vec<Vec<u8>> {
        input.iter().map(|line| line.as_bytes().to_vec()).collect()
    }

    #[tokio::test]
    async fn read_until_command_with_command_first_keeps_every_line() -> Result<()> {
        let (command, body) = read_until_command(lines(&["command=ls-refs", "peel", "symrefs"])).await?;

        assert_eq!(command, "ls-refs");
        assert_eq!(body, lines(&["command=ls-refs", "peel", "symrefs"]));
        Ok(())
    }

    #[tokio::test]
    async fn read_until_command_strips_only_lines_before_command() -> Result<()> {
        let body = lines(&["agent=git/2.43", "object-format=sha1", "command=fetch", "thin-pack", "ofs-delta"]);
        let (command, body) = read_until_command(body).await?;

        assert_eq!(command, "fetch");
        assert_eq!(body, lines(&["command=fetch", "thin-pack", "ofs-delta"]));
        Ok(())
    }

    #[tokio::test]
    async fn read_until_command_without_command_fails() {
        assert!(read_until_command(lines(&["agent=git/2.43", "peel"])).await.is_err());
    }

    #[tokio::test]
    async fn read_data_lines_strips_one_trailing_lf() -> Result<()> {
        let input = b"000ahello\n0009world000cdouble\n\n0000";
        let mut iter = StreamingPeekableIter::new(&input[..], &[PacketLineRef::Flush], false);

        let body = read_data_lines(&mut iter).await?;

        assert_eq!(body, lines(&["hello", "world", "double\n"]));
        Ok(())
    }

    #[tokio::test]
    async fn read_data_lines_skips_delimiters() -> Result<()> {
        let input = b"0014command=ls-refs\n00010009peel\n0000";
        let mut iter = StreamingPeekableIter::new(&input[..], &[PacketLineRef::Flush], false);

        let body = read_data_lines(&mut iter).await?;

        assert_eq!(body, lines(&["command=ls-refs", "peel"]));
        Ok(())
    }
}
