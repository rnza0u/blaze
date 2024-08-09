use std::path::{Path, PathBuf};

use crate::error::Result;
use anyhow::anyhow;

pub fn normalize_path<P: AsRef<Path>>(path: P) -> Result<PathBuf> {
    Ok(PathBuf::from(normalize_path_string(&path_to_string(
        path.as_ref(),
    )?)))
}

pub fn normalize_path_string(path_str: &str) -> String {
    normalize_slashes(path_str)
}

fn normalize_slashes(path_string: &str) -> String {
    #[cfg(unix)]
    return path_string.replace('\\', "/");
    #[cfg(windows)]
    path_string.replace('/', "\\")
}

pub fn path_to_string<P: AsRef<Path>>(path: P) -> Result<String> {
    let path_ref = path.as_ref();
    path_ref
        .to_str()
        .map(str::to_owned)
        .ok_or_else(|| anyhow!("error while converting path {}", path_ref.display()))
}
