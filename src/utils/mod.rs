use std::future::Future;
use std::time::Instant;

pub(crate) mod cookie_file;
pub(crate) mod identifiers;
pub(crate) mod oid;

/// Counts the amount of seconds the provided [Future][future] took to execute.
/// The [Future][future] _should_ not return a output, as it will be discarded and not returned.
///
/// # Panics
///
/// This function panics if the provided [Future][future] panics.
///
/// # Example
///
/// ```
/// use crate::extensions::time_function;
/// use std::time::Duration;
///
/// let seconds = time_function(|| async {
///     std::thread::sleep(Duration::from_secs(5));
/// });
///
/// assert_eq!(5, seconds);
/// ```
///
/// [future]: core::future::Future
pub(crate) async fn time_function<T: Future, F: FnOnce() -> T>(func: F) -> u64 {
    let start = Instant::now();

    func().await;

    start.elapsed().as_secs()
}
