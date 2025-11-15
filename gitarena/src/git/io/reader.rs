use anyhow::{bail, Result};
use git_repository::protocol::transport::packetline::{PacketLineRef, StreamingPeekableIter};
use log::warn;
use tracing::instrument;
use tracing_unwrap::OptionExt;

#[instrument(err)]
pub(crate) async fn read_until_command(mut body: Vec<Vec<u8>>) -> Result<(String, Vec<Vec<u8>>)> {
    for (index, raw_line) in body.iter().enumerate() {
        match String::from_utf8(raw_line.to_vec()) {
            Ok(line) => match line.split_once('=') {
                Some((key, value)) => {
                    if key != "command" {
                        continue;
                    }

                    for i in 0..index {
                        body.remove(i);
                    }

                    return Ok((value.to_owned(), body));
                }
                None => continue,
            },
            Err(err) => {
                warn!("Failed to read line into UTF-8 vec: {}", err);
                continue;
            }
        }
    }

    bail!("Unable to parse Git request body")
}

#[instrument(err, skip(iter))]
pub(crate) async fn read_data_lines(
    iter: &mut StreamingPeekableIter<&[u8]>,
) -> Result<Vec<Vec<u8>>> {
    let mut body = Vec::<Vec<u8>>::new();

    while let Some(line_result) = iter.read_line().await {
        match line_result {
            Ok(Ok(line)) => {
                if let PacketLineRef::Data(data) = line {
                    if data.is_empty() {
                        continue;
                    }

                    // We can safely unwrap() as we checked above that the slice is not empty
                    let length = data.len() - (data.last().unwrap_or_log() == &10_u8) as usize;

                    body.push(data[..length].to_vec());
                }
            }
            Ok(Err(err)) => warn!("Failed to read Git data line: {}", err),
            Err(err) => warn!("Failed to read Git data line: {}", err),
        }
    }

    Ok(body)
}
