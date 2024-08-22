use std::{
    collections::{BTreeSet, HashMap, HashSet},
    fs::metadata,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{anyhow, Context};
use blaze_common::{
    cache::{FileChangesMatcher, MatchingBehavior},
    error::Result,
    time::system_time_as_timestamps,
    value::{to_value, Value},
    IntoEnumIterator,
};

use serde::{Deserialize, Serialize};

use super::{glob::glob, hash::hash_file};

#[derive(PartialEq, Eq, Hash)]
pub enum FileChangeType {
    Modified,
    Created,
    Removed,
}

pub struct FileChange {
    pub path: PathBuf,
    pub change_type: FileChangeType,
}

#[derive(Debug)]
struct FileModificationState {
    pub state: Value,
    pub modified: bool,
}

impl FileModificationState {
    pub fn modified(state: Value) -> Self {
        Self {
            modified: true,
            state,
        }
    }

    pub fn not_modified(state: Value) -> Self {
        Self {
            modified: false,
            state,
        }
    }
}

/// Defines a strategy for retrieving and invalidating a single file changes state.
trait FileModificationCheck {
    /// Get the initial file state (when nothing was cached before).
    fn init(&self, path: &Path) -> Result<Value>;

    /// Check if a file has changed.
    fn check(&self, path: &Path, cached: &Value) -> Result<Option<FileModificationState>>;
}

#[derive(Serialize, Deserialize, Debug)]
struct TimestampsFileState {
    #[serde(with = "system_time_as_timestamps")]
    mtime: SystemTime,
}

/// Compares using only the "mtime" field from the file system.
struct TimestampsFileModificationCheck;

impl FileModificationCheck for TimestampsFileModificationCheck {
    fn init(&self, path: &Path) -> Result<Value> {
        Ok(to_value(TimestampsFileState {
            mtime: get_mtime(path)?,
        })?)
    }

    fn check(&self, path: &Path, cached: &Value) -> Result<Option<FileModificationState>> {
        let state = deserialize_state::<TimestampsFileState>(path, cached)?;
        Ok(check_timestamp_change(path, state.mtime)?
            .map(|new_timestamp| {
                to_value(TimestampsFileState {
                    mtime: new_timestamp,
                })
                .map(FileModificationState::modified)
            })
            .transpose()?)
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct HashFileState {
    hash: u64,
}

/// Compares using only the file's checksum.
struct HashFileModificationCheck;

impl FileModificationCheck for HashFileModificationCheck {
    fn init(&self, path: &Path) -> Result<Value> {
        Ok(to_value(HashFileState {
            hash: get_hash(path)?,
        })?)
    }

    fn check(&self, path: &Path, cached: &Value) -> Result<Option<FileModificationState>> {
        let state = deserialize_state::<HashFileState>(path, cached)?;

        Ok(check_hash_change(path, state.hash)?
            .map(|new_hash| {
                to_value(HashFileState { hash: new_hash }).map(FileModificationState::modified)
            })
            .transpose()?)
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct MixedFileState {
    #[serde(with = "system_time_as_timestamps")]
    mtime: SystemTime,
    hash: u64,
}

/// Compares using the "mtime" field from the file system, and lazily checks if the file's checksum is different.
struct MixedFileModificationCheck;

impl FileModificationCheck for MixedFileModificationCheck {
    fn init(&self, path: &Path) -> Result<Value> {
        Ok(to_value(MixedFileState {
            hash: get_hash(path)?,
            mtime: get_mtime(path)?,
        })?)
    }

    fn check(&self, path: &Path, cached: &Value) -> Result<Option<FileModificationState>> {
        let state: MixedFileState = deserialize_state::<MixedFileState>(path, cached)?;

        let new_timestamp = match check_timestamp_change(path, state.mtime)? {
            Some(new_timestamp) => new_timestamp,
            None => return Ok(None),
        };

        let new_hash = match check_hash_change(path, state.hash)? {
            Some(new_hash) => new_hash,
            None => {
                return Ok(Some(FileModificationState::not_modified(to_value(
                    MixedFileState {
                        mtime: new_timestamp,
                        hash: state.hash,
                    },
                )?)));
            }
        };

        let new_state = to_value(MixedFileState {
            mtime: new_timestamp,
            hash: new_hash,
        })?;

        Ok(Some(FileModificationState::modified(new_state)))
    }
}

fn check_hash_change(path: &Path, cached_hash: u64) -> Result<Option<u64>> {
    let current_hash = get_hash(path)?;
    if cached_hash != current_hash {
        Ok(Some(current_hash))
    } else {
        Ok(None)
    }
}

fn check_timestamp_change(path: &Path, cached_mtime: SystemTime) -> Result<Option<SystemTime>> {
    let current_mtime = get_mtime(path)?;
    if cached_mtime != current_mtime {
        Ok(Some(current_mtime))
    } else {
        Ok(None)
    }
}

fn get_mtime(path: &Path) -> Result<SystemTime> {
    let mtime = metadata(path)
        .with_context(|| {
            anyhow!(
                "file changes check: error while getting file metadata (path={}).",
                path.display()
            )
        })?
        .modified()
        .with_context(|| {
            anyhow!(
                "file changes check: error while getting file mtime (path={}).",
                path.display()
            )
        })?;

    let mtime_ms = Duration::from_millis(mtime.duration_since(UNIX_EPOCH)?.as_millis().try_into()?);

    Ok(UNIX_EPOCH + mtime_ms)
}

fn get_hash(path: &Path) -> Result<u64> {
    hash_file(path).with_context(|| {
        anyhow!(
            "file changes check: error while computing hash (path={}).",
            path.display()
        )
    })
}

fn modification_check_for_matching_behavior(
    behavior: MatchingBehavior,
) -> Box<dyn FileModificationCheck> {
    match behavior {
        MatchingBehavior::Hash => Box::new(HashFileModificationCheck),
        MatchingBehavior::Mixed => Box::new(MixedFileModificationCheck),
        MatchingBehavior::Timestamps => Box::new(TimestampsFileModificationCheck),
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MatchedFilesState(HashMap<MatchingBehavior, HashMap<PathBuf, Value>>);

pub struct MergeResult {
    pub changes: Vec<FileChange>,
    pub files_state: MatchedFilesState,
}

impl MatchedFilesState {
    pub fn from_files(files: MatchedFiles) -> Result<Self> {
        Ok(Self(
            files
                .0
                .into_iter()
                .map(|(behavior, paths)| {
                    let invalidator = modification_check_for_matching_behavior(behavior);
                    Ok((
                        behavior,
                        paths
                            .into_iter()
                            .map(|path| {
                                let data = invalidator.init(&path)?;
                                Ok((path, data))
                            })
                            .collect::<Result<HashMap<_, _>>>()?,
                    ))
                })
                .collect::<Result<_>>()?,
        ))
    }

    pub fn merge(mut self, next_matched_files: MatchedFiles) -> Result<MergeResult> {
        let missing_files = self
            .0
            .iter()
            .map(|(behavior, files)| {
                (
                    *behavior,
                    files
                        .keys()
                        .filter(|path| !next_matched_files.0[behavior].contains(*path))
                        .map(|path| path.to_owned())
                        .collect::<HashSet<_>>(),
                )
            })
            .collect::<HashMap<_, _>>();

        let mut changes = Vec::with_capacity(missing_files.values().map(|files| files.len()).sum());

        for (behavior, paths) in missing_files {
            for path in paths {
                let _ = self.0.get_mut(&behavior).unwrap().remove(&path);
                changes.push(FileChange {
                    path,
                    change_type: FileChangeType::Removed,
                });
            }
        }

        for (behavior, next_files) in next_matched_files.0 {
            let check = modification_check_for_matching_behavior(behavior);
            let cached_files_state = self.0.get_mut(&behavior).unwrap();
            for path in next_files {
                if let Some(existing_state) = cached_files_state.get_mut(&path) {
                    match check.check(&path, existing_state)? {
                        Some(FileModificationState { state, modified }) => {
                            if modified {
                                changes.push(FileChange {
                                    path,
                                    change_type: FileChangeType::Modified,
                                });
                            }
                            *existing_state = state
                        }
                        None => continue,
                    };
                } else {
                    cached_files_state.insert(path.to_owned(), check.init(&path)?);
                    changes.push(FileChange {
                        path,
                        change_type: FileChangeType::Created,
                    });
                }
            }
        }

        Ok(MergeResult {
            changes,
            files_state: Self(self.0),
        })
    }
}

pub struct MatchedFiles(HashMap<MatchingBehavior, HashSet<PathBuf>>);

impl MatchedFiles {
    pub fn try_new(default_root: &Path, matchers: &BTreeSet<FileChangesMatcher>) -> Result<Self> {
        struct MatcherSelection {
            behavior: MatchingBehavior,
            paths: HashSet<PathBuf>,
        }

        let mut selections = matchers
            .iter()
            .enumerate()
            .map(|(i, matcher)| {
                Ok((
                    i,
                    MatcherSelection {
                        behavior: matcher.behavior(),
                        paths: glob(
                            matcher.root().unwrap_or(default_root),
                            matcher.pattern(),
                            matcher.exclude().iter().map(|p| p.as_str()),
                        )?,
                    },
                ))
            })
            .collect::<Result<HashMap<_, _>>>()
            .context("failure while walking across files")?;

        struct BehaviorAggregationHint {
            total_paths: usize,
            selection_indexes: Vec<usize>,
        }

        let mut hints: HashMap<MatchingBehavior, BehaviorAggregationHint> = selections.iter().fold(
            HashMap::from_iter(MatchingBehavior::iter().map(|behavior| {
                (
                    behavior,
                    BehaviorAggregationHint {
                        total_paths: 0,
                        selection_indexes: Vec::with_capacity(selections.len()),
                    },
                )
            })),
            |mut hints, (i, selection)| {
                let hint = hints.get_mut(&selection.behavior).unwrap();
                hint.total_paths += selection.paths.len();
                hint.selection_indexes.push(*i);
                hints
            },
        );

        Ok(Self(HashMap::from_iter(MatchingBehavior::iter().map(
            |behavior| {
                let hint = hints.remove(&behavior).unwrap();
                let mut set = HashSet::with_capacity(hint.total_paths);
                for i in hint.selection_indexes {
                    set.extend(selections.remove(&i).unwrap().paths.iter().cloned());
                }
                (behavior, set)
            },
        ))))
    }
}

fn deserialize_state<'de, T: Deserialize<'de>>(path: &Path, value: &'de Value) -> Result<T> {
    T::deserialize(value)
        .with_context(|| format!("could not deserialize file state for {}", path.display()))
}
