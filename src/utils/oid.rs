use anyhow::{anyhow, Result};
use git_hash::ObjectId;

/// Normalizes an Git object id string.
///
/// This function checks if the input is the null oid (`Some("0000000000000000000000000000000000000000")`),
/// in which case it will return `None`. If that is not the case, the passed input will be returned as-is.
/// Null oid is used by Git to represent an invalid or unknown object.
///
/// # Example
///
/// ```
/// use crate::utils::oid::normalize_str;
///
/// let a = Some("b52f683ce73e8be06428b8c6cf0eb421eae21772");
/// assert_eq!(normalize_str(a), a);
///
/// let b = Some("0000000000000000000000000000000000000000");
/// assert_eq!(normalize_str(b), None);
///
/// let c: Option<&str> = None;
/// assert_eq!(normalize_str(c), None);
/// ```
pub(crate) fn normalize_str(option: Option<&str>) -> Option<&str> {
    match option {
        Some("0000000000000000000000000000000000000000") => None,
        _ => option
    }
}

/// Converts a Git object id string into a [ObjectId][oid].
///
/// If the provided input is `None`, a null [ObjectId][oid] (`0000000000000000000000000000000000000000`)
/// will be returned instead of an error.
///
/// The function will return an error if the input string is not 40 characters long or not a valid
/// hexadecimal string.
///
/// # Example
///
/// ```
/// use crate::utils::oid::from_hex_str;
///
/// assert!(from_hex_str(Some("b52f683ce73e8be06428b8c6cf0eb421eae21772")).is_ok());
/// assert!(from_hex_str(None).is_ok());
/// assert!(from_hex_str(Some("invalid length string")).is_err()); // Invalid length
/// assert!(from_hex_str(Some("yZ0r3ny0K55qqxoz0HZhCWzqAdyFdZ3L9GmXG7EU")).is_err()); // Not hexadecimal
/// ```
///
/// [oid]: git_hash::ObjectId
pub(crate) fn from_hex_str(option: Option<&str>) -> Result<ObjectId> {
    match option {
        Some(oid) => if oid.chars().all(|c| c.is_ascii_hexdigit()) {
            Ok(ObjectId::from_hex(oid.as_bytes())?)
        } else {
            Err(anyhow!("Input string is not hexadecimal: {}", oid))
        }
        None => Ok(ObjectId::null_sha1())
    }
}
