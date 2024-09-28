use std::{borrow::Cow, path::PathBuf};

use anyhow::bail;
use blaze_common::{
    error::Result, executor::NpmOptions, logger::Logger, util::path_to_string, value::{to_value, Value}, workspace::Workspace
};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{
    executors::{node::executor::NodeExecutor, npm},
    system::{npm::npm, process::ProcessOptions, random::random_string},
};

use super::{loader::{ExecutorLoadStrategy, LoaderContext}, resolver::{ExecutorResolution, ExecutorResolver, ExecutorUpdate}};

const PACKAGE_LOCATION: &str = ".blaze/npm";

#[derive(Serialize, Deserialize)]
struct State {
    package_root: PathBuf,
    metadata: Value
}

struct NpmResolver<'a> {
    options: NpmOptions,
    logger: &'a Logger,
    packages_root: PathBuf,
    workspace: &'a Workspace
}

#[derive(Clone, Copy)]
pub struct NpmResolverContext<'a> {
    pub workspace: &'a Workspace,
    pub logger: &'a Logger,
    pub save_in_workspace: bool
}

impl<'a> NpmResolver<'a> {
    pub fn new(options: NpmOptions, context: NpmResolverContext<'a>) -> Self {
        Self {
            options,
            logger: context.logger,
            workspace: context.workspace,
            packages_root: if context.save_in_workspace {
                context.workspace.root().join(PACKAGE_LOCATION)
            } else {
                std::env::temp_dir()
            }
        }
    }
}

impl ExecutorResolver for NpmResolver<'_> {
    fn resolve(&self, url: &Url) -> Result<ExecutorResolution> {
        let package = url.path();
        let registry = url.host_str();

        let package_root =  self.packages_root.join(random_string(12));
        let package_root_str = path_to_string(&package_root)?;

        std::fs::create_dir_all(&package_root_str)?;

        let mut npm_install_options = vec![
            "install", 
            "--global", 
            "--prefix",
            package_root_str.as_str(),
        ];

        let package_with_version = if let Some(version) = self.options.version(){
            Cow::Owned(format!("{package}@{version}"))
        } else {
            Cow::Borrowed(package)
        };

        if let Some(registry) = registry {
            npm_install_options.extend([
                "--registry",
                registry
            ]);
        }

        npm_install_options.push(&package_with_version);

        let install = npm(
            npm_install_options,
            ProcessOptions {
                display_output: true,
                ..Default::default()
            },
        )?.wait()?;

        if !install.success {
            bail!("installation of {url} failed (code: {:?})", install.code);
        }

        let loader = ExecutorLoadStrategy::NodePackage
            .get_loader(LoaderContext {
                workspace: &self.workspace
            });

        let executor_with_metadata =  loader.load_from_src(&package_root)?;

        Ok(ExecutorResolution { 
            executor: executor_with_metadata.executor, 
            state: to_value(State {
                package_root,
                metadata: executor_with_metadata.metadata
            })?
        })
    }

    fn update(&self, _url: &Url, _state: &Value) -> Result<ExecutorUpdate> {
        todo!()
    }
}
