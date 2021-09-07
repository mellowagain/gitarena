use std::collections::HashMap;

use tera::{Result, Value};

pub fn human_prefix(value: &Value, _args: &HashMap<String, Value>) -> Result<Value> {
    let input = value.as_u64().ok_or("Value needs to be u64")?;

    Ok(Value::String(match input {
        i @ 0..=999 => format!("{}", i),
        i @ 1_000..=999_999 => {
            let str = i.to_string();
            format!("{}K", &str[..str.len() - 3])
        },
        _ => "1M+".to_owned(),
    }))
}
