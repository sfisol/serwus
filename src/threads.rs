use ::num_cpus;
use ::std::env;

pub fn num_threads() -> usize {
    let num_cpus = if env::var("TEST").is_ok() {
        2
    } else {
        num_cpus::get()
    };

    if num_cpus < 2 {
        2
    } else {
        num_cpus
    }
}
