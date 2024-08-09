use std::{
    fs::{self, OpenOptions},
    io,
    path::{Path, PathBuf},
};

use anyhow::Context;
use blaze_common::error::Result;

use fs4::FileExt;
use serde::{de::DeserializeOwned, Serialize};
use xxhash_rust::xxh3;

pub struct CacheStore {
    root: PathBuf,
}

const CACHE_FOLDER_NAME: &str = ".blaze/cache";

impl CacheStore {
    /// Cache an object.
    pub fn cache<T>(&self, key: &str, value: &T) -> Result<()>
    where
        T: Serialize,
    {
        let file_path = self.get_entry_filename(key);

        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .open(&file_path)
            .with_context(|| format!("could not open cache entry at {}", file_path.display()))?;

        file.lock_exclusive()?;
        file.set_len(0)?;
        serde_cbor::to_writer(&file, value)?;
        file.unlock()?;

        Ok(())
    }

    /// Invalidate a cache key and remove it.
    pub fn invalidate(&self, key: &str) -> Result<()> {
        let path = self.get_entry_filename(key);
        match std::fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(err) => Err(err)
                .with_context(|| format!("could not remove cache entry at {}", path.display())),
        }
    }

    /// Tries to restore an object from cache based on its type and key.
    /// If the key does not exist, it will return [`Ok(None)`].
    /// Otherwise, it will return [`Ok(Some(T))`].
    pub fn restore<T>(&self, key: &str) -> Result<Option<T>>
    where
        T: DeserializeOwned,
    {
        let path = self.get_entry_filename(key);

        let file = match OpenOptions::new().read(true).open(&path) {
            Ok(file) => file,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(err) => {
                return Err(err)
                    .with_context(|| format!("could not read cache entry at {}", path.display()))
            }
        };

        file.lock_shared()?;
        let object = serde_cbor::from_reader(&file);
        file.unlock()?;

        Ok(Some(object.with_context(|| {
            format!("could not deserialize cache entry at {}", path.display())
        })?))
    }

    pub fn load(root: &Path) -> Result<Self> {
        let root = root.join(CACHE_FOLDER_NAME);

        let is_dir = match fs::metadata(&root) {
            Ok(metadata) => metadata.is_dir(),
            Err(err) if err.kind() == io::ErrorKind::NotFound => false,
            Err(err) => return Err(err.into()),
        };

        if !is_dir {
            std::fs::create_dir_all(&root).with_context(|| {
                format!("failed to created cache directory at {}", root.display())
            })?;
        }

        Ok(Self { root })
    }

    fn get_entry_filename(&self, key: &str) -> PathBuf {
        self.root
            .join(format!("{:0>16x}", xxh3::xxh3_64(key.as_bytes())))
    }
}
