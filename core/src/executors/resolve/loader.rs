use std::{
    panic::{RefUnwindSafe, UnwindSafe},
    path::{Path, PathBuf},
};

use blaze_common::{
    error::Result, executor::ExecutorKind, value::Value, workspace::Workspace
};

use crate::executors::{
    node::prelude::NodeExecutorLoader, rust::RustExecutorLoader, DynExecutor
};

pub struct ExecutorWithMetadata {
    pub executor: DynExecutor,
    pub metadata: Value
}

pub trait ExecutorLoader {
    fn load_from_src(&self, root: &Path) -> Result<ExecutorWithMetadata>;

    fn load_from_metadata(&self, metadata: &Value) -> Result<DynExecutor>;
}

#[derive(Clone, Copy)]
pub struct LoadContext<'a> {
    pub workspace: &'a Workspace,
}

pub struct LoadMetadata {
    pub src: PathBuf,
    pub kind: ExecutorKind,
}

pub fn loader_for_executor_kind(kind: ExecutorKind) -> Box<dyn ExecutorLoader> {
    match kind {
        ExecutorKind::Node => Box::new(NodeExecutorLoader),
        ExecutorKind::Rust => Box::new(RustExecutorLoader),
    }
}
