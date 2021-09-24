use crate::error::GAErrors::ParseError;
use crate::extensions::{flatten_io_result, parse_key_value};

use anyhow::Result;
use git_packetline::{StreamingPeekableIter, PacketLine};
use log::warn;
use tracing::instrument;
use tracing_unwrap::OptionExt;

#[instrument(err)]
pub(crate) async fn read_until_command(mut body: Vec<Vec<u8>>) -> Result<(String, Vec<Vec<u8>>)> {
    for (index, raw_line) in body.iter().enumerate() {
        match String::from_utf8(raw_line.to_vec()) {
            Ok(line) => {
                match parse_key_value(&line) {
                    Ok((key, value)) => {
                        if key != "command" {
                            continue;
                        }

                        for i in 0..index {
                            body.remove(i);
                        }

                        return Ok((value.to_owned(), body));
                    }
                    Err(e) => {
                        warn!("Failed to parse key value: {}", e);
                        continue;
                    }
                }
            }
            Err(e) => {
                warn!("Failed to read line into UTF-8 vec: {}", e);
                continue;
            }
        }
    }

    Err(ParseError("Git request body", String::new()).into())
}

#[instrument(err, skip(iter))]
pub(crate) async fn read_data_lines(iter: &mut StreamingPeekableIter<&[u8]>) -> Result<Vec<Vec<u8>>> {
    let mut body = Vec::<Vec<u8>>::new();

    while let Some(line_result) = iter.read_line().await {
        match flatten_io_result(line_result) {
            Ok(line) => match line {
                PacketLine::Data(data) => {
                    if data.is_empty() {
                        continue;
                    }

                    // We can safely unwrap() as we checked above that the slice is not empty
                    let length = data.len() - (data.last().unwrap_or_log() == &10_u8) as usize;

                    body.push(data[..length].to_vec());
                }
                _ => { /* ignored */ }
            }
            Err(e) => warn!("Failed to read Git data line: {}", e)
        }
    }

    Ok(body)
}
