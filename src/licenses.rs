use std::fs::File;

use anyhow::Result;
use askalono::Store;
use lazy_static::lazy_static;
use log::info;
use tracing_unwrap::ResultExt;

lazy_static! {
    pub(crate) static ref LICENSE_STORE: Store = init_askalono();
}

pub(crate) async fn init() -> Result<()> {
    // Calling .len() here initializes the lazy static variable
    info!("Successfully loaded {} licenses from cache", LICENSE_STORE.len());

    Ok(())
}

fn init_askalono() -> Store {
    let file = File::open("askalono-cache.bin.zstd").expect_or_log("Failed to open askalono cache file");

    Store::from_cache(file).expect_or_log("Failed to parse askalono cache file")
}

pub(crate) const fn license_file_names() -> [&'static [u8]; 18] {
    [
        b"copying", b"copyright", b"eula", b"license", b"notice", b"patents", b"unlicense", b"agpl", b"gpl",
        b"lgpl", b"apache-", b"bsd-", b"cc-by-", b"gfdl-", b"gnu-", b"mit-", b"mpl-", b"ofl-"
    ]
}
