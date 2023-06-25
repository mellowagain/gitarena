use std::time::Duration;

use futures_locks::RwLock;
use once_cell::sync::Lazy;
use sysinfo::{RefreshKind, System, SystemExt};

pub(crate) static SYSTEM_INFO: Lazy<RwLock<System>> = Lazy::new(init);

fn init() -> RwLock<System> {
    let mut interval = tokio::time::interval(Duration::from_secs(5 * 60));

    let system = System::new_with_specifics(RefreshKind::new().with_memory());
    let lock = RwLock::new(system);

    tokio::spawn(async move {
        loop {
            interval.tick().await;
            SYSTEM_INFO.write().await.refresh_memory()
        }
    });

    lock
}
