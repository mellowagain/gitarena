use std::fs::File;

use askalono::Store;
use log::info;
use once_cell::sync::OnceCell;
use tracing_unwrap::{OptionExt, ResultExt};

static LICENSE_STORE: OnceCell<Store> = OnceCell::new();

pub(crate) async fn init() {
    // Normally we'd use .expect_or_log() here but askalono::Store does not implement Debug, so just ignore the error
    // This is safe because OnceCell only returns an Error on set() when it already was once initialized
    let _ = LICENSE_STORE.set(init_askalono());

    info!("Successfully loaded {} licenses from cache", store().len());
}

fn init_askalono() -> Store {
    let file =
        File::open("askalono-cache.bin.zstd").expect_or_log("Failed to open askalono cache file");

    Store::from_cache(file).expect_or_log("Failed to parse askalono cache file")
}

pub(crate) fn store() -> &'static Store {
    LICENSE_STORE.get().unwrap_or_log()
}

pub(crate) const fn license_file_names() -> [&'static [u8]; 18] {
    [
        b"copying",
        b"copyright",
        b"eula",
        b"license",
        b"notice",
        b"patents",
        b"unlicense",
        b"agpl",
        b"gpl",
        b"lgpl",
        b"apache-",
        b"bsd-",
        b"cc-by-",
        b"gfdl-",
        b"gnu-",
        b"mit-",
        b"mpl-",
        b"ofl-",
    ]
}
