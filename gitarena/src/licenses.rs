use askalono::Store;
use once_cell::sync::OnceCell;
use tracing::{info, instrument};
use tracing_unwrap::{OptionExt, ResultExt};

static LICENSE_STORE: OnceCell<Store> = OnceCell::new();

#[instrument]
pub(crate) fn init() {
    // Normally we'd use .expect_or_log() here but askalono::Store does not implement Debug, so just ignore the error
    // This is safe because OnceCell only returns an Error on set() when it already was once initialized
    let _ = LICENSE_STORE.set(init_askalono());

    let amount = store().len();
    info!(amount, "Successfully loaded licenses from cache");
}

fn init_askalono() -> Store {
    let bytes = include_bytes!("../askalono-cache.bin.zstd");
    Store::from_cache(&bytes[..]).expect_or_log("Failed to parse askalono cache file")
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
