use std::path::Path;

use anyhow::Context;
use blaze_common::{
    error::Result,
    value::{to_value, Value},
};
use serde::Deserialize;

use crate::executors::{
    loader::{ExecutorLoader, ExecutorWithMetadata},
    DynExecutor,
};

use super::{executor::NodeExecutor, package::NodeExecutorPackage};

pub struct NodeExecutorLoader;

impl ExecutorLoader for NodeExecutorLoader {
    fn load_from_src(&self, root: &Path) -> Result<ExecutorWithMetadata> {
        let package = NodeExecutorPackage::from_root(root).with_context(|| {
            format!(
                "error while reading node executor metadata at {}",
                root.display()
            )
        })?;

        let executor = Box::new(NodeExecutor::new(package));

        Ok(ExecutorWithMetadata {
            metadata: to_value(&executor)?,
            executor,
        })
    }

    fn load_from_metadata(&self, metadata: &Value) -> Result<DynExecutor> {
        Ok(Box::new(NodeExecutor::deserialize(metadata)?))
    }
}
