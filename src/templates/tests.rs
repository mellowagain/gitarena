use tera::{Result, Value, Error};

pub(crate) fn empty(value: Option<&Value>, _: &[Value]) -> Result<bool> {
    match value {
        Some(value) => Ok(value.as_str().ok_or_else(|| Error::msg("Can only check if String is empty"))?.is_empty()),
        None => Ok(false)
    }
}

pub(crate) fn none(value: Option<&Value>, _: &[Value]) -> Result<bool> {
    match value {
        Some(value) => Ok(value.is_null()),
        None => Ok(false)
    }
}

pub(crate) fn some(value: Option<&Value>, _: &[Value]) -> Result<bool> {
    match value {
        Some(value) => Ok(!value.is_null()),
        None => Ok(false)
    }
}
