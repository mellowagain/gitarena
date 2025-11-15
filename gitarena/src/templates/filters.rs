use std::collections::HashMap;

use chrono::{DateTime, NaiveDateTime, Utc};
use chrono_humanize::HumanTime;
use tera::{Result, Value};

pub(crate) fn human_prefix(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    let input = value.as_u64().ok_or("Value needs to be u64")?;

    Ok(Value::String(match input {
        i @ 0..=999 => format!("{}", i),
        i @ 1_000..=999_999 => {
            let str = i.to_string();
            format!("{}K", &str[..str.len() - 3])
        }
        _ => "1M+".to_owned(),
    }))
}

pub(crate) fn human_time(value: &Value, _: &HashMap<String, Value>) -> Result<Value> {
    let input = value.as_i64().ok_or("Value needs to be i64")?;

    let naive = NaiveDateTime::from_timestamp(input, 0);
    let date_time: DateTime<Utc> = DateTime::from_naive_utc_and_offset(naive, Utc);

    let human_time = HumanTime::from(date_time);

    Ok(Value::String(format!("{}", human_time)))
}
