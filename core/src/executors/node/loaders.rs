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

/// This loader will manually install dependencies and build the executor before loading.
pub struct LocalNodeExecutorLoader;

impl ExecutorLoader for LocalNodeExecutorLoader {
    fn load_from_src(&self, root: &Path) -> Result<ExecutorWithMetadata> {
        let package = read_package(root)?;

        package
            .build()
            .with_context(|| format!("failed to build node executor at {}", root.display()))?;

        get_executor_with_metadata(package)
    }

    fn load_from_metadata(&self, metadata: &Value) -> Result<DynExecutor> {
        load_from_metadata(metadata)
    }
}

// This loader will not call `npm install` or any build script before loading.
pub struct NpmPackageNodeExecutorLoader;

impl ExecutorLoader for NpmPackageNodeExecutorLoader {
    fn load_from_src(&self, root: &Path) -> Result<ExecutorWithMetadata> {
        get_executor_with_metadata(read_package(root)?)
    }

    fn load_from_metadata(&self, metadata: &Value) -> Result<DynExecutor> {
        load_from_metadata(metadata)
    }
}

fn get_executor_with_metadata(package: NodeExecutorPackage) -> Result<ExecutorWithMetadata> {
    let executor = Box::new(NodeExecutor::new(package));

    Ok(ExecutorWithMetadata {
        metadata: to_value(&executor)?,
        executor,
    })
}

fn read_package(root: &Path) -> Result<NodeExecutorPackage> {
    NodeExecutorPackage::from_root(root).with_context(|| {
        format!(
            "error while reading node executor metadata at {}",
            root.display()
        )
    })
}

fn load_from_metadata(metadata: &Value) -> Result<DynExecutor> {
    Ok(Box::new(NodeExecutor::deserialize(metadata)?))
}
