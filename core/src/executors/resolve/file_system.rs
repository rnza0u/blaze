use std::{
    borrow::Cow,
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
    builder::get_builder_for_executor_kind,
    kinds::infer_local_executor_type,
    resolver::{ExecutorResolution, ExecutorResolver, ExecutorUpdate, SourceInfo},
};

#[derive(Serialize, Deserialize)]
pub struct State {
    files: MatchedFilesState,
}

fn default_file_changes_matchers(root: &Path) -> BTreeSet<FileChangesMatcher> {
    ["src/**", "Cargo.toml", "package.json"]
        .into_iter()
        .map(|pattern| {
            FileChangesMatcher::new(pattern)
                .with_root(root)
                .with_behavior(MatchingBehavior::Mixed)
        })
        .collect()
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
        let matchers = self
            .options
            .watch()
            .map(Cow::Borrowed)
            .unwrap_or_else(|| Cow::Owned(default_file_changes_matchers(root)));
        MatchedFiles::try_new(root, &matchers)
    }

    fn get_kind(&self, root: &Path) -> Result<ExecutorKind> {
        let kind = if let Some(kind) = self.options.kind() {
            kind
        } else {
            infer_local_executor_type(root)?
        };
        Ok(kind)
    }
}

impl ExecutorResolver for FileSystemResolver<'_> {
    fn resolve(&self, url: &Url) -> Result<ExecutorResolution> {
        let root = self
            .get_canonical_root_path(url)
            .with_context(|| format!("could not get canonical executor path from {url}"))?;

        let kind = self.get_kind(&root)?;
        let builder = get_builder_for_executor_kind(kind);
        builder.build(&root)?;

        Ok(ExecutorResolution {
            state: to_value(State {
                files: MatchedFilesState::from_files(self.get_matched_files(&root)?)?,
            })?,
            source: SourceInfo { kind, root },
        })
    }

    fn update(&self, url: &Url, state: &Value) -> Result<ExecutorUpdate> {
        let root = self
            .get_canonical_root_path(url)
            .with_context(|| format!("could not get canonical executor path from {url}"))?;
        let state = State::deserialize(state)?;

        let matched_files = self.get_matched_files(&root)?;
        let merged_state = state.files.merge(matched_files)?;

        let new_src = match self.options.rebuild() {
            RebuildStrategy::OnChanges if merged_state.changes.is_empty() => None,
            _ => {
                let kind = self.get_kind(&root)?;
                let builder = get_builder_for_executor_kind(kind);
                builder.build(&root)?;
                Some(SourceInfo { root, kind })
            }
        };

        Ok(ExecutorUpdate {
            new_state: Some(to_value(State {
                files: merged_state.files_state,
            })?),
            new_source: new_src,
        })
    }
}
