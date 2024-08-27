use std::path::Path;

use blaze_common::{error::Result, value::Value, workspace::Workspace};

use crate::executors::{
    node::loaders::LocalNodeExecutorLoader, rust::loaders::LocalRustExecutorLoader, DynExecutor,
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

#[allow(unused)]
pub enum ExecutorLoadStrategy {
    RustLocal,
    RustCrate,
    NodeLocal,
    NodePackage,
}

impl ExecutorLoadStrategy {
    pub fn get_loader(&self, context: LoaderContext<'_>) -> Box<dyn ExecutorLoader> {
        match self {
            Self::NodeLocal => Box::new(LocalNodeExecutorLoader),
            Self::RustLocal => Box::new(LocalRustExecutorLoader::new(context.workspace.root())),
            _ => todo!(),
        }
    }
}
