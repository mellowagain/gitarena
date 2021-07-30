use crate::error::GAErrors::ParseError;
use crate::extensions::parse_key_value;

use anyhow::Result;
use log::warn;

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

    Err(ParseError("Git request body", "(null)".to_owned()).into())
}
