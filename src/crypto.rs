use anyhow::{Context, Result};
use argon2::{Config, ThreadMode, Variant, Version};
use rand::Rng;

const CHARSET : &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789)(*&^%$#@!~";
const ARGON_CONFIG : Config = Config {
    ad: &[],
    hash_length: 32,
    lanes: 4,
    mem_cost: 4096,
    secret: &[],
    thread_mode: ThreadMode::Parallel,
    time_cost: 3,
    variant: Variant::Argon2id,
    version: Version::Version13
};

pub(crate) fn random_string(length: usize) -> String {
    let mut rng = rand::thread_rng();

    (0..length).map(|_| {
        let index = rng.gen_range(0, CHARSET.len());
        CHARSET[index] as char
    }).collect()
}

pub(crate) fn hash_password(password: String) -> Result<(String, String)> {
    let salt = random_string(16);

    Ok((argon2::hash_encoded(
        password.as_bytes(), salt.as_bytes(), &ARGON_CONFIG
    ).context("Failed to hash password.")?, salt))
}

pub(crate) fn check_password(user: &crate::user::User, password: String) -> Result<bool> {
    Ok(argon2::verify_encoded(
        user.password.as_str(), password.as_bytes()
    ).with_context(|| format!("Failed to check password for user #{}.", user.id))?)
}
