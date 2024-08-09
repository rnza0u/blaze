use std::path::PathBuf;

use blaze_common::{error::Result, executor::NpmOptions, logger::Logger, value::Value, workspace::Workspace};
use url::Url;

use super::resolver::{ExecutorResolver, ExecutorSource};

const PACKAGE_LOCATION: &str = ".blaze/npm";

struct NpmResolver<'a> {
    options: NpmOptions,
    logger: &'a Logger,
    packages_root: PathBuf
}

#[derive(Clone, Copy)]
pub struct NpmResolverContext<'a> {
    workspace: &'a Workspace,
    logger: &'a Logger
}

impl <'a> NpmResolver<'a> {
    pub fn new(options: NpmOptions, context: NpmResolverContext<'a>) -> Self {
        Self {
            options,
            logger: context.logger,
            packages_root: context.workspace.root().join(PACKAGE_LOCATION)
        }
    }
}

impl ExecutorResolver for NpmResolver<'_> {
    fn resolve(&self, _url: &Url) -> Result<ExecutorSource> {
        todo!()
    }

    fn update(&self, _url: &Url, _state: &Value) -> Result<Option<ExecutorSource>> {
        todo!()
    }
}
