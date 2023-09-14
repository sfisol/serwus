use rand::{distributions::Alphanumeric, thread_rng, Rng};
use std::iter;

pub fn generate_code() -> String {
    let mut rng = thread_rng();
    iter::repeat(())
        .map(|()| rng.sample(Alphanumeric))
        .map(char::from)
        .take(64)
        .collect()
}
