use crate::LICENSE_STORE;

use std::env;
use std::path::Path;

use anyhow::Result;
use log::info;

pub(crate) fn init() -> Result<()> {
    info!("Loading SPDX license data. This may take a while.");

    let mut path = env::current_dir()?;
    path.push(Path::new("license-list-data/json/details"));

    LICENSE_STORE.lock().unwrap().load_spdx(path.as_path(), true).unwrap();

    Ok(())
}

pub(crate) const fn license_file_names() -> [&'static [u8]; 18] {
    [
        b"copying", b"copyright", b"eula", b"license", b"notice", b"patents", b"unlicense", b"agpl", b"gpl",
        b"lgpl", b"apache-", b"bsd-", b"cc-by-", b"gfdl-", b"gnu-", b"mit-", b"mpl-", b"ofl-"
    ]
}
