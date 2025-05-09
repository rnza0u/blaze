use rand::{distr::Alphanumeric, rng, Rng};

pub fn random_string(size: usize) -> String {
    rng()
        .sample_iter(&Alphanumeric)
        .take(size)
        .map(char::from)
        .collect::<String>()
}
