use std::path::Path;

use blaze_common::{
    error::Result,
    value::{to_value, Value},
};
use serde::Deserialize;

use crate::executors::{
    loader::{ExecutorLoader, ExecutorWithMetadata},
    rust::executor::RustExecutor,
    DynExecutor,
};

use super::package::RustExecutorPackage;

pub struct RustExecutorLoader;

impl ExecutorLoader for RustExecutorLoader {
    fn load_from_metadata(&self, metadata: &Value) -> Result<DynExecutor> {
        Ok(Box::new(RustExecutor::deserialize(metadata)?))
    }

    fn load_from_src(&self, root: &Path) -> Result<ExecutorWithMetadata> {
        let package = RustExecutorPackage::from_root(root)?;

        let executor = RustExecutor::new(&package.library_path(), package.exported_fn());
        let metadata = to_value(&executor)?;

        Ok(ExecutorWithMetadata {
            executor: Box::new(executor),
            metadata,
        })
    }
}
