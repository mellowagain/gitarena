use crate::database::Pool;
use opentelemetry::metrics::ObservableGauge;
use opentelemetry::{KeyValue, global};
use opentelemetry_semantic_conventions::attribute::DB_CLIENT_CONNECTION_POOL_NAME;
use std::future;
use tokio::task::JoinHandle;
use tracing::{Instrument, info_span};

pub fn spawn_db_pool_metrics_task(pool: Pool) -> JoinHandle<()> {
    tokio::spawn(
        async move {
            let meter = global::meter("gitarena");

            let options = pool.connect_options();

            let db_name = options.get_database().map_or_else(String::new, |db| format!("/{db}"));
            let pool_name = format!("{}:{}/{db_name}", options.get_host(), options.get_port());

            let pool_name_attr = KeyValue::new(DB_CLIENT_CONNECTION_POOL_NAME, pool_name);
            let db_system_attr = KeyValue::new("db.system.name", "postgresql");

            let pool_for_count = pool.clone();
            let pool_name_for_count = pool_name_attr.clone();
            let db_sys_for_count = db_system_attr.clone();

            let _conn_count: ObservableGauge<i64> = meter
                .i64_observable_gauge("db.client.connection.count")
                .with_description("The number of connections that are currently in state")
                .with_unit("{connection}")
                .with_callback(move |observer| {
                    let total = i64::from(pool_for_count.size());
                    let idle = i64::try_from(pool_for_count.num_idle()).unwrap_or(0);
                    let used = total - idle;
                    observer.observe(used, &[pool_name_for_count.clone(), db_sys_for_count.clone(), KeyValue::new("state", "used")]);
                    observer.observe(idle, &[pool_name_for_count.clone(), db_sys_for_count.clone(), KeyValue::new("state", "idle")]);
                })
                .build();

            let pool_for_max = pool.clone();
            let pool_name_for_max = pool_name_attr.clone();
            let db_sys_for_max = db_system_attr.clone();

            let _conn_max: ObservableGauge<i64> = meter
                .i64_observable_gauge("db.client.connection.max")
                .with_description("The maximum number of open connections allowed")
                .with_unit("{connection}")
                .with_callback(move |observer| {
                    let max = i64::from(pool_for_max.options().get_max_connections());
                    observer.observe(max, &[pool_name_for_max.clone(), db_sys_for_max.clone()]);
                })
                .build();

            // Keep this task (and the gauge handles) alive until the process exits.
            future::pending::<()>().await;
        }
        .instrument(info_span!("db_metrics_observer_task")),
    )
}
