use once_cell::sync::Lazy;
use opentelemetry::global;
use opentelemetry::metrics::{Counter, Histogram};

/// Count of git transport operations (fetch, push, ls-refs).
///
/// # Attributes
///
/// - `operation` (upload-pack|push)
/// - `transport` (http|ssh)
/// - `status` (ok|error)
pub static OPERATION_COUNT: Lazy<Counter<u64>> = Lazy::new(|| {
    global::meter("gitarena")
        .u64_counter("git.operation.count")
        .with_description("Count of git transport operations.")
        .with_unit("{operation}")
        .build()
});

/// Duration of git transport operations in seconds.
///
/// # Attributes
///
/// - `operation` (upload-pack|push)
/// - `transport` (http|ssh)
pub static OPERATION_DURATION: Lazy<Histogram<f64>> = Lazy::new(|| {
    global::meter("gitarena")
        .f64_histogram("git.operation.duration")
        .with_description("Duration of git transport operations.")
        .with_unit("s")
        .build()
});
