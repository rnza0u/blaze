use std::path::Path;

use blaze_common::{error::Result, value::Value, workspace::Workspace};
use serde::{Deserialize, Serialize};

use crate::executors::{
    node::loaders::{LocalNodeExecutorLoader, NpmPackageNodeExecutorLoader}, rust::loaders::LocalRustExecutorLoader, DynExecutor,
};

pub struct ExecutorWithMetadata {
    pub executor: DynExecutor,
    pub metadata: Value,
}

pub trait ExecutorLoader {
    fn load_from_src(&self, root: &Path) -> Result<ExecutorWithMetadata>;

    fn load_from_metadata(&self, metadata: &Value) -> Result<DynExecutor>;
}

pub struct LoaderContext<'a> {
    pub workspace: &'a Workspace,
}

#[derive(Serialize, Deserialize)]
pub enum ExecutorLoadStrategy {
    RustLocal,
    NodeLocal,
    NodePackage,
}

impl ExecutorLoadStrategy {
    pub fn get_loader(&self, context: LoaderContext<'_>) -> Box<dyn ExecutorLoader> {
        match self {
            Self::NodeLocal => Box::new(LocalNodeExecutorLoader),
            Self::RustLocal => Box::new(LocalRustExecutorLoader::new(context.workspace.root())),
            Self::NodePackage => Box::new(NpmPackageNodeExecutorLoader),
            _ => todo!(),
        }
    }
}
