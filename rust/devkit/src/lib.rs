use blaze_common::logger::Logger;
use blaze_common::project::Project;
use blaze_common::workspace::Workspace;
use std::error::Error;
use value::Value;

pub struct ExecutorContext<'a> {
    pub workspace: &'a Workspace,
    pub project: &'a Project,
    pub target: &'a str,
    pub logger: &'a Logger,
}

pub type ExecutorResult = Result<(), Box<dyn Error + Send + Sync>>;

pub type ExecutorFn = fn(ctx: ExecutorContext, options: Value) -> ExecutorResult;

pub use blaze_common::*;

pub use value;
