use std::path::Path;

use blaze_common::{error::Result, executor::ExecutorKind};

pub trait ExecutorBuilder {
    fn build(&self, root: &Path) -> Result<()>;
}

pub fn builder_for_executor_kind(kind: ExecutorKind) -> Box<dyn ExecutorBuilder> {
    match kind {
        ExecutorKind::Node => todo!(),
        ExecutorKind::Rust => todo!()
    }
}