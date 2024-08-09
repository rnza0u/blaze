use blaze_common::{error::Result, executor::Location, value::Value};
use url::Url;

use super::{
    file_system::FileSystemResolver, git::GitResolver, git_common::GitResolverContext,
    http_git::GitOverHttpResolver, loader::LoadMetadata, ssh_git::GitOverSshResolver,
    CustomResolutionContext,
};

pub struct ExecutorSource {
    pub load_metadata: LoadMetadata,
    pub state: Value,
}

pub trait ExecutorResolver {
    fn resolve(&self, url: &Url) -> Result<ExecutorSource>;

    fn update(&self, url: &Url, state: &Value) -> Result<Option<ExecutorSource>>;
}

pub fn resolver_for_location<'a>(
    location: Location,
    context: CustomResolutionContext<'a>,
) -> Box<dyn ExecutorResolver + 'a> {
    let git_context = || GitResolverContext {
        logger: context.logger,
        workspace: context.workspace,
    };

    match location {
        Location::LocalFileSystem { options } => {
            Box::new(FileSystemResolver::new(context.workspace.root(), options))
        }
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
