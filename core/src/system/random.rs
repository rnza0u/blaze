use rand::{distributions::Alphanumeric, thread_rng, Rng};

pub fn random_string(size: usize) -> String {
    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(size)
        .map(char::from)
        .collect::<String>()
}
