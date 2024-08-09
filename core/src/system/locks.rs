use std::{
    fs::{File, OpenOptions},
    path::Path,
};

use anyhow::bail;
use blaze_common::error::Result;
use fs4::{lock_contended_error, FileExt};

const LOCKS_PATH: &str = ".blaze/locks";
const LOCKS_CLEANUP_LOCK_ID: u64 = 0;

pub fn clean_locks(root: &Path) -> Result<()> {
    let lock = ProcessLock::try_new(root, LOCKS_CLEANUP_LOCK_ID)?;

    lock.locked(|| {
        let entries = std::fs::read_dir(root.join(LOCKS_PATH))?;
        for entry in entries {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }
            let path = entry.path();
            let file = OpenOptions::new().write(true).open(&path)?;
            match file.try_lock_exclusive() {
                Ok(()) => {
                    file.unlock()?;
                    std::fs::remove_file(&path)?;
                }
                Err(err) => {
                    if err.kind() == lock_contended_error().kind() {
                        continue;
                    }
                    bail!(err)
                }
            }
        }
        Ok(())
    })?
}

pub struct ProcessLock {
    lockfile: File,
    on_wait: Option<Box<dyn FnOnce()>>,
}

impl ProcessLock {
    pub fn try_new(root: &Path, id: u64) -> Result<Self> {
        let lockfiles_dir = root.join(LOCKS_PATH);

        std::fs::create_dir_all(&lockfiles_dir)?;

        let lockfile = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .read(true)
            .open(lockfiles_dir.join(id.to_string()))?;

        Ok(Self {
            lockfile,
            on_wait: None,
        })
    }

    pub fn on_wait<F>(&mut self, f: F)
    where
        F: FnOnce() + 'static,
    {
        self.on_wait = Some(Box::new(f))
    }

    pub fn locked<T, F>(self, f: F) -> Result<T>
    where
        F: FnOnce() -> T,
    {
        match self.lockfile.try_lock_exclusive() {
            Ok(()) => {}
            Err(err) if err.kind() == lock_contended_error().kind() => {
                if let Some(on_wait) = self.on_wait {
                    on_wait();
                }
                self.lockfile.lock_exclusive()?;
            }
            Err(err) => bail!(err),
        }

        let result = f();

        self.lockfile.unlock()?;

        Ok(result)
    }
}
