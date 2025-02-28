use std::path::Path;

use blaze_common::{error::Result, executor::ExecutorKind};

use crate::executors::{node::builder::NodeExecutorBuilder, rust::builder::RustExecutorBuilder};

pub trait ExecutorBuilder {
    fn build(&self, root: &Path) -> Result<()>;
}

pub fn get_builder_for_executor_kind(kind: ExecutorKind) -> Box<dyn ExecutorBuilder> {
    match kind {
        ExecutorKind::Node => Box::new(NodeExecutorBuilder),
        ExecutorKind::Rust => Box::new(RustExecutorBuilder),
    }
}
