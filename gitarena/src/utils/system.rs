use anyhow::{Context, Result};
use futures_locks::RwLock;
use std::sync::OnceLock;
use std::time::Duration;
use sysinfo::{MemoryRefreshKind, RefreshKind, System};

pub(crate) static SYSTEM_INFO: OnceLock<RwLock<System>> = OnceLock::new();

pub(crate) fn init() -> Result<()> {
    let mut interval = tokio::time::interval(Duration::from_mins(5));

    let system = System::new_with_specifics(RefreshKind::nothing().with_memory(MemoryRefreshKind::everything()))
        .context("unable to initialize memory system info refresher")?;

    let _ = SYSTEM_INFO.set(RwLock::new(system));

    tokio::spawn(async move {
        loop {
            interval.tick().await;

            if let Some(system) = SYSTEM_INFO.get() {
                system.write().await.refresh_memory();
            }
        }
    });

    Ok(())
}
