use blaze_common::{error::Result, executor::Location, value::Value};
use url::Url;

use crate::executors::DynExecutor;

use super::{
    file_system::{FileSystemResolver, FileSystemResolverContext},
    git::GitResolver,
    git_common::GitResolverContext,
    http_git::GitOverHttpResolver,
    ssh_git::GitOverSshResolver,
    CustomResolutionContext,
};

pub struct ExecutorResolution {
    pub executor: DynExecutor,
    pub state: Value,
}

pub struct ExecutorUpdate {
    pub executor: DynExecutor,
    pub new_state: Option<Value>,
    pub updated: bool,
}

pub trait ExecutorResolver {
    fn resolve(&self, url: &Url) -> Result<ExecutorResolution>;

    fn update(&self, url: &Url, state: &Value) -> Result<ExecutorUpdate>;
}

pub fn resolver_for_location<'a>(
    location: Location,
    context: CustomResolutionContext<'a>,
) -> Box<dyn ExecutorResolver + 'a> {
    let git_context = || GitResolverContext {
        logger: context.logger,
        workspace: context.workspace,
        save_in_workspace: context.cache.is_some()
    };

    match location {
        Location::LocalFileSystem { options } => Box::new(FileSystemResolver::new(
            options,
            FileSystemResolverContext {
                workspace: context.workspace,
            },
        )),
        Location::Git { options } => Box::new(GitResolver::new(options, git_context())),
        Location::GitOverHttp {
            transport,
            git_options,
            authentication,
        } => Box::new(GitOverHttpResolver::new(
            git_options,
            transport,  
            authentication,
            git_context(),
        )),
        Location::GitOverSsh {
            transport,
            git_options,
            authentication,
        } => Box::new(GitOverSshResolver::new(
            git_options,
            transport,
            authentication,
            git_context(),
        )),
        _ => todo!(),
    }
}
