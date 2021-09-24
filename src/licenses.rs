use std::env;
use std::path::Path;

use anyhow::Result;
use askalono::Store;
use futures_locks::Mutex;
use lazy_static::lazy_static;
use log::info;
use tracing_unwrap::ResultExt;

lazy_static! {
    pub(crate) static ref LICENSE_STORE: Mutex<Store> = Mutex::new(Store::new());
}

pub(crate) async fn init() -> Result<()> {
    info!("Loading SPDX license data. This may take a while.");

    let mut path = env::current_dir()?;
    path.push(Path::new("license-list-data/json/details"));

    LICENSE_STORE.lock().await.load_spdx(path.as_path(), true).unwrap_or_log();

    Ok(())
}

pub(crate) const fn license_file_names() -> [&'static [u8]; 18] {
    [
        b"copying", b"copyright", b"eula", b"license", b"notice", b"patents", b"unlicense", b"agpl", b"gpl",
        b"lgpl", b"apache-", b"bsd-", b"cc-by-", b"gfdl-", b"gnu-", b"mit-", b"mpl-", b"ofl-"
    ]
}
