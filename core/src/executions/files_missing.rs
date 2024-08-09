use std::{borrow::Cow, collections::BTreeSet, path::PathBuf};

use anyhow::bail;
use blaze_common::{error::Result, logger::Logger};

use super::{
    check::{CacheInvalidationCheck, ExecutionCacheState},
    execution::TargetExecution,
};

pub struct FilesMissingCheck<'a> {
    logger: &'a Logger,
    files: &'a BTreeSet<PathBuf>,
}

impl<'a> FilesMissingCheck<'a> {
    pub fn new(files: &'a BTreeSet<PathBuf>, logger: &'a Logger) -> Self {
        Self { files, logger }
    }
}

impl CacheInvalidationCheck for FilesMissingCheck<'_> {
    fn validate(&mut self, execution: &TargetExecution, _: &ExecutionCacheState) -> Result<bool> {
        for path in self.files {
            let normalized_path = if path.is_relative() {
                Cow::Owned(execution.get_project().root().join(path))
            } else {
                Cow::Borrowed(path)
            };

            let exists = match std::fs::metadata(normalized_path.as_path()) {
                Ok(metadata) => metadata.is_file() || metadata.is_dir(),
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => false,
                Err(err) => bail!(err),
            };

            if !exists {
                self.logger
                    .debug(format!("{} is missing", normalized_path.display()));
                return Ok(false);
            }
        }
        Ok(true)
    }
}
