use std::path::{Path, PathBuf};

use anyhow::anyhow;
use blaze_common::{
    error::Result,
    executor::{ExecutorKind, GitCheckout, GitOptions},
    logger::Logger,
    value::{to_value, Value},
    workspace::Workspace,
};
use git2::{build::CheckoutBuilder, FetchOptions, RemoteCallbacks};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::system::random::random_string;

use super::{
    kinds::infer_local_executor_type,
    loader::ExecutorLoadStrategy,
    resolver::{ExecutorResolution, ExecutorUpdate},
    ExecutorResolver,
};

const REPOSITORIES_PATH: &str = ".blaze/repositories";

#[derive(Serialize, Deserialize)]
struct State {
    repository_path: PathBuf,
    src_path: PathBuf,
    kind: ExecutorKind,
}

pub struct GitHeadlessResolver<'a> {
    git_options: GitOptions,
    context: GitResolverContext<'a>,
    repositories_root: PathBuf,
    remote_callbacks_customizer: Box<dyn Fn(&mut RemoteCallbacks<'_>)>,
    fetch_options_customizer: Box<dyn Fn(&mut FetchOptions<'_>)>,
}

#[derive(Clone, Copy)]
pub struct GitResolverContext<'a> {
    pub workspace: &'a Workspace,
    pub logger: &'a Logger,
    pub save_in_workspace: bool,
}

impl<'a> GitHeadlessResolver<'a> {
    pub fn new(
        git_options: GitOptions,
        context: GitResolverContext<'a>,
        remote_callbacks_customizer: impl Fn(&mut RemoteCallbacks<'_>) + 'static,
        fetch_options_customizer: impl Fn(&mut FetchOptions<'_>) + 'static,
    ) -> Self {
        Self {
            context,
            repositories_root: if context.save_in_workspace {
                context.workspace.root().join(REPOSITORIES_PATH)
            } else {
                std::env::temp_dir()
            },
            remote_callbacks_customizer: Box::new(remote_callbacks_customizer),
            fetch_options_customizer: Box::new(fetch_options_customizer),
            git_options,
        }
    }

    fn default_remote_callbacks(&self) -> RemoteCallbacks {
        let remote_callbacks = RemoteCallbacks::new();
        remote_callbacks
    }

    fn default_fetch_options<'r>(&self, remote_callbacks: RemoteCallbacks<'r>) -> FetchOptions<'r> {
        let mut fetch_options = git2::FetchOptions::new();
        fetch_options.remote_callbacks(remote_callbacks);
        fetch_options.download_tags(git2::AutotagOption::All);
        fetch_options
    }

    fn get_src_path(&self, repository_path: &Path) -> PathBuf {
        if let Some(path) = &self.git_options.path() {
            repository_path.join(path)
        } else {
            repository_path.to_owned()
        }
    }

    fn get_kind(&self, src_path: &Path) -> Result<ExecutorKind> {
        let kind = if let Some(kind) = self.git_options.kind() {
            kind
        } else {
            infer_local_executor_type(src_path)?
        };
        Ok(kind)
    }

    fn get_load_strategy(&self, kind: ExecutorKind) -> ExecutorLoadStrategy {
        match kind {
            ExecutorKind::Rust => ExecutorLoadStrategy::RustLocal,
            ExecutorKind::Node => ExecutorLoadStrategy::NodeLocal,
        }
    }
}

impl ExecutorResolver for GitHeadlessResolver<'_> {
    fn resolve(&self, url: &Url) -> Result<ExecutorResolution> {
        let repository_path = self.repositories_root.join(random_string(12));

        if repository_path.try_exists()? {
            std::fs::remove_dir_all(&repository_path)?;
        }

        std::fs::create_dir_all(&repository_path)?;

        let mut repo_builder = git2::build::RepoBuilder::new();
        let mut remote_callbacks = self.default_remote_callbacks();
        (self.remote_callbacks_customizer)(&mut remote_callbacks);

        let mut fetch_options = self.default_fetch_options(remote_callbacks);
        (self.fetch_options_customizer)(&mut fetch_options);

        repo_builder.fetch_options(fetch_options);

        let repository = repo_builder.clone(url.as_ref(), &repository_path)?;

        self.context
            .logger
            .debug(format!("cloned {} to {}", url, repository_path.display()));

        if let Some(checkout) = &self.git_options.checkout() {
            match checkout {
                GitCheckout::Branch {
                    branch: branch_name,
                } => {
                    let branch = repository
                        .find_branch(&format!("origin/{branch_name}"), git2::BranchType::Remote)?;
                    repository.set_head(
                        branch
                            .into_reference()
                            .name()
                            .ok_or_else(|| anyhow!("could not get refname for {branch_name}"))?,
                    )?;
                }
                GitCheckout::Tag { tag } => {
                    let tag = repository.find_reference(&format!("refs/tags/{tag}"))?;
                    repository.set_head_detached(tag.peel_to_commit()?.id())?;
                }
                GitCheckout::Revision { rev } => {
                    let revision = repository.revparse_single(rev)?;
                    repository.set_head_detached(revision.peel_to_commit()?.id())?;
                }
            }
            repository.checkout_head(Some(&mut CheckoutBuilder::default().force()))?;
        }

        let src_path = self.get_src_path(&repository_path);

        let kind = self.get_kind(&src_path)?;
        let load_strategy = self.get_load_strategy(kind);

        Ok(ExecutorResolution {
            src: src_path.to_owned(),
            load_strategy,
            state: to_value(State {
                kind,
                repository_path,
                src_path,
            })?,
        })
    }

    fn update(&self, url: &Url, state: &Value) -> Result<ExecutorUpdate> {
        let state = State::deserialize(state)?;
        let repository = git2::Repository::open(&state.repository_path)?;
        let logger = self.context.logger;

        let no_update = || ExecutorUpdate {
            load_strategy: self.get_load_strategy(state.kind),
            new_state: None,
            update: None,
        };

        if !self.git_options.pull() {
            return Ok(no_update());
        }

        let refspecs = match &self.git_options.checkout() {
            Some(checkout) => match checkout {
                GitCheckout::Branch { branch } => {
                    vec![format!("refs/heads/{branch}")]
                }
                GitCheckout::Tag { tag } => {
                    vec![format!("refs/tags/{tag}")]
                }
                GitCheckout::Revision { rev } => {
                    vec![rev.to_owned()]
                }
            },
            None => vec!["HEAD".to_owned()],
        };

        let remote_callbacks = self.default_remote_callbacks();
        let mut fetch_options = self.default_fetch_options(remote_callbacks);
        let mut remote = repository.find_remote("origin")?;

        remote.fetch(&refspecs, Some(&mut fetch_options), None)?;

        logger.debug(format!("fetched refspecs {:?} for {}", refspecs, url));

        let fetch_head = repository.find_reference("FETCH_HEAD")?;
        let mut head = repository.head()?;

        let fetch_head_commit = fetch_head
            .resolve()?
            .target()
            .ok_or_else(|| anyhow!("could not resolve commit id for FETCH_HEAD"))?;
        let head_commit = head
            .resolve()?
            .target()
            .ok_or_else(|| anyhow!("could not resolve commit id for HEAD"))?;

        if fetch_head_commit == head_commit {
            logger.debug(format!(
                "{url} fetch head commit has not changed ({fetch_head_commit})"
            ));
            return Ok(no_update());
        }

        head.set_target(
            fetch_head_commit,
            &format!(
                "Blaze repository update: {:?} to {:?}",
                head.name(),
                fetch_head_commit
            ),
        )?;
        repository.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;

        logger.debug(format!(
            "repository files were updated for {url} (now at {fetch_head_commit})"
        ));

        let src_path = self.get_src_path(&state.repository_path);

        let kind = self.get_kind(&src_path)?;

        Ok(ExecutorUpdate {
            update: Some(src_path.to_owned()),
            new_state: Some(to_value(State {
                kind,
                repository_path: state.repository_path,
                src_path,
            })?),
            load_strategy: self.get_load_strategy(kind),
        })
    }
}
