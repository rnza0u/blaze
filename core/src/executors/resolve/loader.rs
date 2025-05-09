use std::path::Path;

use blaze_common::{error::Result, executor::ExecutorKind, value::Value};

use crate::executors::{
    node::loader::NodeExecutorLoader, rust::loaders::RustExecutorLoader, DynExecutor,
};

pub struct ExecutorWithMetadata {
    pub executor: DynExecutor,
    pub metadata: Value,
}

pub trait ExecutorLoader {
    fn load_from_src(&self, root: &Path) -> Result<ExecutorWithMetadata>;

    fn load_from_metadata(&self, metadata: &Value) -> Result<DynExecutor>;
}

pub fn get_loader_for_executor_kind(kind: ExecutorKind) -> Box<dyn ExecutorLoader> {
    match kind {
        ExecutorKind::Node => Box::new(NodeExecutorLoader),
        ExecutorKind::Rust => Box::new(RustExecutorLoader),
    }
}
