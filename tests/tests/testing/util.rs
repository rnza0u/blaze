use std::path::{Path, PathBuf};

#[cfg(windows)]
pub fn get_fixtures_root() -> PathBuf {
    Path::new(env!("PROJECT_ROOT")).join("tests\\fixtures")
}

#[cfg(not(windows))]
pub fn get_fixtures_root() -> PathBuf {
    Path::new(env!("PROJECT_ROOT")).join("tests/fixtures")
}
