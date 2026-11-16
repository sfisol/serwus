//! The environment of the service.

use std::sync::Once;

/// Reads the variables of `.env`, if there is one, into the environment, without overriding the
/// ones already set - the first time it's called; later calls do nothing.
///
/// [`Serwus::start`](crate::server::Serwus::start) and the database pools call it before they
/// read the environment. A service which reads its own settings from it earlier, f. ex. into
/// a `static` read before the pool is made, calls it first, at the top of `main`:
///
/// ```no_run
/// let port = {
///     serwus::load_env();
///     std::env::var("PORT").unwrap_or_else(|_| "8000".to_string())
/// };
/// ```
pub fn load_env() {
    static LOADED: Once = Once::new();
    LOADED.call_once(|| {
        dotenvy::dotenv().ok();
    });
}
