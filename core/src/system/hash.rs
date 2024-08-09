use blaze_common::error::Result;
use std::{hash::Hasher, io::Read, path::Path};
use xxhash_rust::xxh3::Xxh3;

/// Get a hasher instance. Not for crypto !
pub fn hasher() -> impl Hasher + Clone {
    Xxh3::new()
}

/// Get a checksum for a file.
/// Not for crypto !
pub fn hash_file(path: &Path) -> Result<u64> {
    let mut buffer = [0_u8; 8192];
    let mut file = std::fs::File::open(path)?;
    let mut hasher = hasher();

    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            return Ok(hasher.finish());
        }
        hasher.write(&buffer[..read])
    }
}
