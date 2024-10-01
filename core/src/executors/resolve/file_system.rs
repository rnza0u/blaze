use std::{
    collections::BTreeSet,
    io,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context};
use blaze_common::{
    cache::{FileChangesMatcher, MatchingBehavior},
    error::Result,
    executor::{ExecutorKind, FileSystemOptions, RebuildStrategy},
    value::{to_value, Value},
    workspace::Workspace,
};

use serde::{Deserialize, Serialize};
use url::Url;

use crate::system::file_changes::{MatchedFiles, MatchedFilesState};

use super::{
    kinds::infer_local_executor_type,
    loader::{ExecutorLoadStrategy, ExecutorLoader, ExecutorWithMetadata, LoaderContext},
    resolver::{ExecutorResolution, ExecutorResolver, ExecutorUpdate},
};

#[derive(Serialize, Deserialize)]
pub struct State {
    kind: ExecutorKind,
    metadata: Value,
    files: MatchedFilesState,
}

fn default_file_changes_matchers(root: &Path) -> BTreeSet<FileChangesMatcher> {
    [FileChangesMatcher::new("**")
        .with_exclude([
            "node_modules/**",
            "target/**",
            ".git/**",
            ".vscode/**",
            "dist/**",
            "build/**",
        ])
        .with_root(root)
        .with_behavior(MatchingBehavior::Mixed)]
    .into()
}

pub struct FileSystemResolverContext<'a> {
    pub workspace: &'a Workspace,
}

/// Resolves an executor based on a file URL.
pub struct FileSystemResolver<'a> {
    options: FileSystemOptions,
    context: FileSystemResolverContext<'a>,
}

impl<'a> FileSystemResolver<'a> {
    pub fn new(options: FileSystemOptions, context: FileSystemResolverContext<'a>) -> Self {
        Self { options, context }
    }

    fn get_canonical_root_path(&self, url: &Url) -> Result<PathBuf> {
        let url_path = Path::new(url.path());
        let absolute = if url_path.is_absolute() {
            url_path.to_path_buf()
        } else {
            self.context.workspace.root().join(url_path)
        };

        let is_dir = match std::fs::metadata(&absolute) {
            Ok(metadata) => metadata.is_dir(),
            Err(err) if err.kind() == io::ErrorKind::NotFound => false,
            Err(err) => return Err(err.into()),
        };

        if !is_dir {
            bail!(
                "{} is not a directory. file:// URLs must point to the source files root directory of your executor.", 
                absolute.display()
            )
        }

        Ok(dunce::canonicalize(absolute)?)
    }

    fn get_matched_files(&self, root: &Path) -> Result<MatchedFiles> {
        let default = default_file_changes_matchers(root);
        MatchedFiles::try_new(root, self.options.watch().unwrap_or(&default))
    }


    fn build(&self, root: &Path) -> Result<(ExecutorWithMetadata, ExecutorKind)> {
        let kind = if let Some(kind) = self.options.kind() {
            kind
        } else {
            infer_local_executor_type(root)?
        };

        let loader = self.get_loader(kind);

        let executor_with_metadata = loader
            .load_from_src(root)
            .with_context(|| format!("could not load executor {}", root.display()))?;

        Ok((executor_with_metadata, kind))
    }
}

impl ExecutorResolver for FileSystemResolver<'_> {
    fn resolve(&self, url: &Url) -> Result<ExecutorResolution> {
        let root = self
            .get_canonical_root_path(url)
            .with_context(|| format!("could not get canonical executor path from {url}"))?;

        let (ExecutorWithMetadata { executor, metadata }, kind) = self.build_and_load(&root)?;

        Ok(ExecutorResolution {
            executor,
            state: to_value(State {
                files: MatchedFilesState::from_files(self.get_matched_files(&root)?)?,
                kind,
                metadata,
            })?,
        })
    }

    fn update(&self, url: &Url, state: &Value) -> Result<ExecutorUpdate> {
        let root = self
            .get_canonical_root_path(url)
            .with_context(|| format!("could not get canonical executor path from {url}"))?;
        let state = State::deserialize(state)?;

        let matched_files = self.get_matched_files(&root)?;
        let merged_state = state.files.merge(matched_files)?;

        let update = match self.options.rebuild() {
            RebuildStrategy::OnChanges if merged_state.changes.is_empty() => {
                let loader = self.get_loader(state.kind);
                ExecutorUpdate {
                    executor: loader.load_from_metadata(&state.metadata)?,
                    new_state: Some(to_value(State {
                        files: merged_state.files_state,
                        kind: state.kind,
                        metadata: state.metadata,
                    })?),
                    updated: false,
                }
            }
            _ => {
                let (ExecutorWithMetadata { executor, metadata }, kind) =
                    self.build_and_load(&root)?;
                ExecutorUpdate {
                    executor,
                    new_state: Some(to_value(State {
                        kind,
                        files: merged_state.files_state,
                        metadata,
                    })?),
                    updated: true,
                }
            }
        };

        Ok(update)
    }
}
