// Third-party crates re-exported for usage by other gitarena-* crates
// Allows us to specify the crate only once in the gitarena-common Cargo.toml, making crate updates easier
// Note: Not all crates use these re-exports *yet*, it is planned to convert them at some point (mainly the main `gitarena` crate)

pub use anyhow;
pub use base64;
pub use console_subscriber;
pub use futures;
pub use futures_locks;
pub use num_cpus;
pub use num_traits;
pub use once_cell;
pub use serde;
pub use sqlx;

/// In order for the `tokio::main` proc macro to work correctly, the main file must have `use gitarena_common::tokio;` specified within it
pub use tokio;
