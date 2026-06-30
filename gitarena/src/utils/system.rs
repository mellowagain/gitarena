use std::time::Duration;

use futures_locks::RwLock;
use once_cell::sync::Lazy;
use sysinfo::{RefreshKind, System};
use tracing::{Instrument, info_span};

pub(crate) static SYSTEM_INFO: Lazy<RwLock<System>> = Lazy::new(init);

fn init() -> RwLock<System> {
    let mut interval = tokio::time::interval(Duration::from_mins(5));

    let system = System::new_with_specifics(RefreshKind::nothing().with_memory(sysinfo::MemoryRefreshKind::everything()));
    let lock = RwLock::new(system);

    tokio::spawn(
        async move {
            loop {
                interval.tick().await;
                SYSTEM_INFO.write().await.refresh_memory();
            }
        }
        .instrument(info_span!("system_memory_refresh_task")),
    ); // idk if it even makes sense to instrument this task

    lock
}
