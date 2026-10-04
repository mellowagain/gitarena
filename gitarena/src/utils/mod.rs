use once_cell::sync::Lazy;
use std::future::Future;
use std::time::Instant;
use trust_dns_resolver::TokioAsyncResolver;
use uuid::Uuid;

pub(crate) mod admin_panel_layer;
pub(crate) mod filesystem;
pub(crate) mod identifiers;
pub(crate) mod oid;
pub(crate) mod system;

pub(crate) static DNS_RESOLVER: Lazy<TokioAsyncResolver> =
    Lazy::new(|| TokioAsyncResolver::tokio_from_system_conf().expect("to be able to initialize dns resolver"));

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
/// [future]: Future
pub(crate) async fn time_function<T: Future, F: FnOnce() -> T>(func: F) -> (u64, T::Output) {
    let start = Instant::now();

    let result = func().await;

    #[allow(clippy::cast_possible_truncation)]
    (start.elapsed().as_millis() as u64, result)
}

/// Checks if the email is a forge private email and if it is, decodes the base 58 to return the UUID.
/// The function does not check if the UUID is actually a valid user
pub(crate) fn decode_forge_mail(email: &str, forge_domain: &str) -> Option<Uuid> {
    let (local_part, domain) = email.rsplit_once('@')?;

    if !domain.eq_ignore_ascii_case(forge_domain) {
        return None;
    }

    let bytes = bs58::decode(local_part).into_vec().ok()?;
    Uuid::from_slice(&bytes).ok()
}
