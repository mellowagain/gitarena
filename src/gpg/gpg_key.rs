use pgp::{Deserializable, SignedPublicKey};
use crate::user::User;
use anyhow::Result;
use pgp::types::KeyTrait;

pub(crate) struct GpgKey {
    pub(crate) id: i32,
    pub(crate) user_id: i32,
    pub(crate) email: String,
    pub(crate) key_id: String,
    pub(crate) raw_key: Vec<u8>
}

impl GpgKey {
    #[deprecated(note = "This function generates invalid values for field `id` and `raw_key`, let sqlx create this object")]
    pub(crate) fn new(user: &User, key: &str) -> Result<GpgKey> {
        let signed_public_key = parse_public_key(key)?;

        Ok(GpgKey {
            id: -1,
            user_id: user.id,
            email: user.email.clone(), // Should probably get the email from the GPG key, not from the user
            key_id: hex::encode(signed_public_key.primary_key.key_id().as_ref()),
            raw_key: Vec::new()
        })
    }
}

fn parse_public_key(key: &str) -> Result<SignedPublicKey> {
    Ok(SignedPublicKey::from_string(key)?.0)
}
