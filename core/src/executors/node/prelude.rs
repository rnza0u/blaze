use std::path::Path;

use anyhow::Context;
use blaze_common::{error::Result, value::Value};
use serde::{Deserialize, Serialize};

use crate::executors::{
    loader::{CustomExecutorLoader, DynCustomExecutor, LoadContext},
    Executor, ExecutorContext,
};

use super::{
    bridge::{execute_node_bridge, NodeBridgeParameters},
    package::NodeExecutorPackage,
};

#[derive(Clone, Serialize, Deserialize)]
pub struct NodeExecutor {
    #[serde(flatten)]
    package: NodeExecutorPackage,
}

impl Executor for NodeExecutor {
    fn execute(&self, context: ExecutorContext, options: Value) -> Result<()> {
        execute_node_bridge(NodeBridgeParameters {
            module: &self.package.root.join(self.package.path.as_path()),
            context,
            options: &options,
        })
    }
}

pub struct NodeExecutorLoader;

impl CustomExecutorLoader for NodeExecutorLoader {
    fn load_from_src(&self, root: &Path, _context: LoadContext<'_>) -> Result<DynCustomExecutor> {
        let package = NodeExecutorPackage::from_root(root).with_context(|| {
            format!(
                "error while reading node executor metadata at {}",
                root.display()
            )
        })?;

        package
            .build()
            .with_context(|| format!("error while building node executor at {}", root.display()))?;

        Ok(Box::new(NodeExecutor { package }))
    }

    fn load_from_metadata(&self, metadata: &Value) -> Result<DynCustomExecutor> {
        Ok(Box::new(NodeExecutor::deserialize(metadata)?))
    }
}
