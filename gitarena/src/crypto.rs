use crate::user::User;

use anyhow::{Context, Result};
use argon2::{Config, ThreadMode, Variant, Version};
use rand::distributions::Distribution;
use rand::distributions::Uniform;

const ARGON_CONFIG: Config = Config {
    ad: &[],
    hash_length: 32,
    lanes: 4,
    mem_cost: 4096,
    secret: &[],
    thread_mode: ThreadMode::Parallel,
    time_cost: 3,
    variant: Variant::Argon2id,
    version: Version::Version13,
};

pub(crate) fn random_string_charset(length: usize, charset: &'static [u8]) -> String {
    let mut rng = rand::thread_rng();
    let uniform = Uniform::new(0, charset.len());

    (0..length)
        .map(|_| {
            let index = uniform.sample(&mut rng);
            charset[index] as char
        })
        .collect()
}

pub(crate) fn random_string(length: usize) -> String {
    const CHARSET: &[u8] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789)(*&^%$#@!~";

    random_string_charset(length, CHARSET)
}

pub(crate) fn random_numeric_ascii_string(length: usize) -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";

    random_string_charset(length, CHARSET)
}

pub(crate) fn random_hex_string(length: usize) -> String {
    const CHARSET: &[u8] = b"abcdef0123456789";

    random_string_charset(length, CHARSET)
}

pub(crate) fn hash_password(password: &str) -> Result<String> {
    let salt = random_string(16);

    argon2::hash_encoded(password.as_bytes(), salt.as_bytes(), &ARGON_CONFIG)
        .context("Failed to hash password")
}

pub(crate) fn check_password(user: &User, password: &str) -> Result<bool> {
    argon2::verify_encoded(user.password.as_str(), password.as_bytes())
        .with_context(|| format!("Failed to check password for user #{}", user.id))
}
