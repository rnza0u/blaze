use std::panic::{RefUnwindSafe, UnwindSafe};

use blaze_common::{
    error::Result, logger::Logger, project::Project, value::Value, workspace::Workspace,
};
use serde::Serialize;

#[derive(Serialize)]
pub struct ExecutorContext<'a> {
    pub workspace: &'a Workspace,
    pub project: &'a Project,
    pub target: &'a str,
    #[serde(skip)]
    pub logger: &'a Logger,
}

pub type DynExecutor = Box<dyn Executor + Send + Sync + UnwindSafe + RefUnwindSafe>;

pub trait Executor {
    fn execute(&self, context: ExecutorContext, options: Value) -> Result<()>;
}
