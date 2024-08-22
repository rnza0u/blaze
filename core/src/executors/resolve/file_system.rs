use std::{
    collections::BTreeSet,
    io,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context};
use blaze_common::{
    cache::{FileChangesMatcher, MatchingBehavior}, error::Result, executor::{ExecutorKind, FileSystemOptions, RebuildStrategy}, util::normalize_path, value::{to_value, Value}
};

use serde::{Deserialize, Serialize};
use url::Url;

use crate::system::file_changes::{MatchedFiles, MatchedFilesState};

use super::{
    kinds::infer_local_executor_type, loader::{loader_for_executor_kind, LoadMetadata}, resolver::{ExecutorResolution, ExecutorSource},
    ExecutorResolver,
};

#[derive(Serialize, Deserialize)]
pub struct State {
    files: MatchedFilesState,
}

fn default_file_changes_matchers(root: &Path) -> BTreeSet<FileChangesMatcher> {
    [FileChangesMatcher::new("**")
        .with_exclude(["node_modules/**", "target/**"])
        .with_root(root)
        .with_behavior(MatchingBehavior::Mixed)]
    .into()
}

/// Resolves an executor based on a file URL.
pub struct FileSystemResolver {
    options: FileSystemOptions,
    default_cwd: PathBuf,
}

impl FileSystemResolver {
    pub fn new(default_cwd: &Path, options: FileSystemOptions) -> Self {
        Self {
            options,
            default_cwd: default_cwd.to_owned(),
        }
    }

    fn get_canonical_root_path(&self, url: &Url) -> Result<PathBuf> {
        let url_path = normalize_path(Path::new(url.path()))?;
        let absolute = if url_path.is_absolute() {
            url_path.to_path_buf()
        } else {
            self.default_cwd.join(url_path)
        };
        Ok(dunce::canonicalize(absolute)?)
    }

    fn get_matched_files(&self, root: &Path) -> Result<MatchedFiles> {
        let default = default_file_changes_matchers(root);
        MatchedFiles::try_new(root, self.options.watch().unwrap_or(&default))
    }
}

impl ExecutorResolver for FileSystemResolver {
    fn resolve(&self, url: &Url) -> Result<ExecutorResolution> {
        let root = self.get_canonical_root_path(url)
            .with_context(|| format!("could not get canonical executor path from {url}"))?;

        let is_dir = match std::fs::metadata(&root) {
            Ok(metadata) => metadata.is_dir(),
            Err(err) if err.kind() == io::ErrorKind::NotFound => false,
            Err(err) => return Err(err.into()),
        };

        if !is_dir {
            bail!("{url} is not a directory. file:// URLs must point to the source files root directory of your executor.")
        }

        let kind = if let Some(kind) = self.options.kind() {
            kind
        } else {
            infer_local_executor_type(&root)?
        };

        let loader = loader_for_executor_kind(kind);

        Ok(ExecutorSource {
            state: to_value(State {
                files: MatchedFilesState::from_files(self.get_matched_files(&root)?)?,
            })?,
            load_metadata: LoadMetadata {
                kind: if let Some(kind) = self.options.kind() {
                    kind
                } else {
                    infer_local_executor_type(&root)?
                },
                src: root,
            },
        })
    }

    fn update(&self, url: &Url, state: &Value) -> Result<Option<ExecutorSource>> {
        let root = get_canonical_root_path(url, &self.default_cwd)
            .with_context(|| format!("could not get canonical executor path from {url}"))?;
        let state = State::deserialize(state)?;

        let matched_files = self.get_matched_files(&root)?;
        let merged_state = state.files.merge(matched_files)?;

        let new_state = State {
            files: merged_state.state,
        };

        let update = || {
            Ok(Some(ExecutorSource {
                state: to_value(new_state)?,
                load_metadata: LoadMetadata {
                    kind: infer_local_executor_type(&root)?,
                    src: root,
                },
            }))
        };

        match self.options.rebuild() {
            RebuildStrategy::Always => update(),
            RebuildStrategy::OnChanges if !merged_state.changes.is_empty() => update(),
            _ => Ok(None),
        }
    }
}