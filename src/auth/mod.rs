//! Helpers for user authentication (JWT, 3rd-party)

#[cfg(feature = "auth")]
pub mod jwt;

#[cfg(feature = "session")]
pub mod session;

#[cfg(feature = "auth")]
pub use crate::containers::role::*;

#[cfg(any(feature = "rs256_jwks", feature = "rs256_jwks_native_roots"))]
pub mod rs256_jwks;
