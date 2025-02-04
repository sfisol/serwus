use rand::{distr::Alphanumeric, rng, Rng};
use std::iter;

/// Generate random string of 64 chars for one-time token purposes.
pub fn generate_code() -> String {
    let mut rng = rng();
    iter::repeat(())
        .map(|()| rng.sample(Alphanumeric))
        .map(char::from)
        .take(64)
        .collect()
}
