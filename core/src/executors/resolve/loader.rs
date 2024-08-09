use std::{
    panic::{RefUnwindSafe, UnwindSafe},
    path::{Path, PathBuf},
};

use blaze_common::{
    error::Result,
    executor::ExecutorKind,
    value::{to_value, Value},
    workspace::Workspace,
};
use serde::Serialize;

use crate::executors::{
    node::prelude::NodeExecutorLoader, rust::RustExecutorLoader, DynExecutor, Executor,
};

pub type DynCustomExecutor = Box<dyn CustomExecutor>;

pub trait CustomExecutor: Executor + Send + Sync {
    fn metadata(&self) -> Result<Value>;

    fn to_dyn(self: Box<Self>) -> DynExecutor;
}

impl<T> CustomExecutor for T
where
    T: Executor + Send + Sync + UnwindSafe + RefUnwindSafe + Serialize + 'static,
{
    fn metadata(&self) -> Result<Value> {
        Ok(to_value(self)?)
    }

    fn to_dyn(self: Box<Self>) -> DynExecutor {
        self
    }
}

pub trait CustomExecutorLoader {
    fn load_from_metadata(&self, metadata: &Value) -> Result<DynCustomExecutor>;

    fn load_from_src(&self, root: &Path, context: LoadContext<'_>) -> Result<DynCustomExecutor>;
}

#[derive(Clone, Copy)]
pub struct LoadContext<'a> {
    pub workspace: &'a Workspace,
}

pub struct LoadMetadata {
    pub src: PathBuf,
    pub kind: ExecutorKind,
}

pub fn loader_for_executor_kind(kind: ExecutorKind) -> Box<dyn CustomExecutorLoader> {
    match kind {
        ExecutorKind::Node => Box::new(NodeExecutorLoader),
        ExecutorKind::Rust => Box::new(RustExecutorLoader),
    }
}
