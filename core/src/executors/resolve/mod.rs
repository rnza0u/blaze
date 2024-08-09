pub mod file_system;
pub mod git;
pub mod git_common;
pub mod http_git;
pub mod kinds;
pub mod loader;
pub mod npm;
pub mod resolver;
pub mod ssh_git;
pub mod standard;

use std::{
    collections::{HashMap, HashSet},
    hash::{Hash, Hasher},
};

use anyhow::Context;
use blaze_common::{
    error::Result,
    executor::{ExecutorKind, ExecutorReference, Location},
    logger::Logger,
    value::Value,
    workspace::Workspace,
};
use possibly::possibly;
use rand::{thread_rng, RngCore};
use resolver::ExecutorSource;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{
    executors::DynExecutor,
    system::{hash::hasher, locks::ProcessLock, parallel_executor::ParallelRunner},
    workspace::cache_store::CacheStore,
};

use standard::resolve_standard_executor;

use self::{
    loader::{loader_for_executor_kind, LoadContext},
    resolver::{resolver_for_location, ExecutorResolver},
};

/// Extra data needed in order to resolve an executor.
#[derive(Clone, Copy)]
pub struct CustomResolutionContext<'a> {
    pub workspace: &'a Workspace,
    pub cache: Option<&'a CacheStore>,
    pub logger: &'a Logger,
}

#[derive(Serialize, Deserialize)]
pub struct CachedMetadata {
    pub kind: ExecutorKind,
    pub resolution_state: Value,
    pub executor_metadata: Value,
    pub nonce: u64,
}

pub struct ResolvedExecutors {
    executors: HashMap<u64, ExecutorResolution>,
}

impl ResolvedExecutors {
    pub fn get_for_reference(&self, reference: &ExecutorReference) -> Option<&ExecutorResolution> {
        self.executors.get(&get_executor_package_id(reference))
    }
}

pub enum ExecutorResolution {
    Standard(DynExecutor),
    Custom(CustomExecutorResolution),
}

impl ExecutorResolution {
    pub fn executor(&self) -> &DynExecutor {
        match self {
            Self::Standard(executor) => executor,
            Self::Custom(CustomExecutorResolution { executor, .. }) => executor,
        }
    }

    pub fn resolution_cache(&self) -> Option<(ExecutorCacheState, u64)> {
        possibly!(self, Self::Custom(CustomExecutorResolution { state, nonce, .. }) => (*state, *nonce))
    }
}

/// Try to resolve executors from their references.
pub fn resolve_executors<'a, I>(
    references: I,
    context: CustomResolutionContext<'_>,
) -> Result<ResolvedExecutors>
where
    I: IntoIterator<Item = &'a ExecutorReference>,
{
    std::thread::scope(|scope| {
        let references = references
            .into_iter()
            .map(|reference| (get_executor_package_id(reference), reference))
            .collect::<HashSet<_>>();

        let mut references_by_package_id =
            HashMap::<u64, HashSet<&ExecutorReference>>::with_capacity(references.len());

        for (package_id, reference) in references {
            if let Some(refs) = references_by_package_id.get_mut(&package_id) {
                refs.insert(reference);
            } else {
                references_by_package_id.insert(package_id, HashSet::from([reference]));
            }
        }
        let mut resolutions =
            HashMap::<u64, ExecutorResolution>::with_capacity(references_by_package_id.len());
        let mut references_drain = references_by_package_id.drain();
        let mut runner = ParallelRunner::new(
            scope,
            context
                .workspace
                .settings()
                .resolution_parallelism()
                .unwrap_or_default(),
        )?;

        loop {
            runner.push_available(|| {
                let (package_id, references) = references_drain.next()?;
                Some(move || {
                    let mut cached: Option<CustomExecutorResolution> = None;
                    for reference in references {
                        let (url, location) = match reference {
                            ExecutorReference::Standard { url } => {
                                return Ok((
                                    package_id,
                                    ExecutorResolution::Standard(
                                        resolve_standard_executor(url).with_context(|| {
                                            format!("standard executor \"{url}\" does not exist")
                                        })?,
                                    ),
                                ))
                            }
                            ExecutorReference::Custom { url, location } => (url, location),
                        };

                        let lock = ProcessLock::try_new(context.workspace.root(), package_id)?;

                        let custom_executor_resolution = lock.locked(|| {
                            resolve_custom_executor(url, location, package_id, context)
                        })??;

                        match custom_executor_resolution.state {
                            ExecutorCacheState::New | ExecutorCacheState::Updated => {
                                return Ok((
                                    package_id,
                                    ExecutorResolution::Custom(custom_executor_resolution),
                                ))
                            }
                            ExecutorCacheState::Cached => {
                                let _ = cached.insert(custom_executor_resolution);
                            }
                        }
                    }
                    Ok((package_id, ExecutorResolution::Custom(cached.unwrap())))
                })
            });

            if !runner.is_running() {
                break;
            }

            resolutions.extend(runner.drain()?.into_iter().collect::<Result<Vec<_>>>()?);
        }

        Ok(ResolvedExecutors {
            executors: resolutions,
        })
    })
}

#[derive(Copy, Clone)]
pub enum ExecutorCacheState {
    New,
    Updated,
    Cached,
}

pub struct CustomExecutorResolution {
    state: ExecutorCacheState,
    executor: DynExecutor,
    nonce: u64,
}

fn resolve_custom_executor(
    url: &Url,
    location: &Location,
    package_id: u64,
    context: CustomResolutionContext<'_>,
) -> Result<CustomExecutorResolution> {
    let resolver: Box<dyn ExecutorResolver> = resolver_for_location(location.clone(), context);

    let state_key = format!("executors/{package_id}");

    let maybe_cached_metadata = context
        .cache
        .and_then(|cache| cache.restore::<CachedMetadata>(&state_key).transpose())
        .transpose()
        .with_context(|| format!("failed to restore solution state for executor {url}"))?;

    let load_context = LoadContext {
        workspace: context.workspace,
    };

    let (executor, next_cached_metadata) = match maybe_cached_metadata {
        Some(cached_metadata) => {
            context.logger.debug(format!("{url} exists in cache"));
            let executor_update = resolver
                .update(url, &cached_metadata.resolution_state)
                .with_context(|| {
                    format!(
                    "failed to validate executor resolution for {url}. cache might be corrupted."
                )
                })?;

            match executor_update {
                Some(ExecutorSource {
                    state,
                    load_metadata,
                }) => {
                    let reloaded_executor = loader_for_executor_kind(load_metadata.kind)
                        .load_from_src(&load_metadata.src, load_context)?;
                    context
                        .logger
                        .debug(format!("{url} was reloaded from source"));
                    let reloaded_executor_metadata = reloaded_executor.metadata()?;
                    let nonce = thread_rng().next_u64();
                    (
                        CustomExecutorResolution {
                            executor: reloaded_executor.to_dyn(),
                            state: ExecutorCacheState::Updated,
                            nonce,
                        },
                        CachedMetadata {
                            kind: load_metadata.kind,
                            executor_metadata: reloaded_executor_metadata,
                            resolution_state: state,
                            nonce,
                        },
                    )
                }
                None => {
                    let cached_executor = loader_for_executor_kind(cached_metadata.kind)
                        .load_from_metadata(&cached_metadata.executor_metadata)?;
                    context
                        .logger
                        .debug(format!("{url} was reloaded from cache"));
                    (
                        CustomExecutorResolution {
                            executor: cached_executor.to_dyn(),
                            state: ExecutorCacheState::Cached,
                            nonce: cached_metadata.nonce,
                        },
                        cached_metadata,
                    )
                }
            }
        }
        None => {
            let resolution = resolver
                .resolve(url)
                .with_context(|| format!("failed to resolve executor {url}"))?;

            context.logger.debug(format!("{url} was resolved"));

            let executor = loader_for_executor_kind(resolution.load_metadata.kind)
                .load_from_src(&resolution.load_metadata.src, load_context)?;

            context
                .logger
                .debug(format!("{url} was loaded from source"));

            let executor_metadata = executor.metadata()?;
            let nonce: u64 = thread_rng().next_u64();

            (
                CustomExecutorResolution {
                    executor: executor.to_dyn(),
                    state: ExecutorCacheState::New,
                    nonce,
                },
                CachedMetadata {
                    kind: resolution.load_metadata.kind,
                    executor_metadata,
                    resolution_state: resolution.state,
                    nonce,
                },
            )
        }
    };

    if let Some(cache) = context.cache {
        cache
            .cache(&state_key, &next_cached_metadata)
            .with_context(|| format!("failed to cache executor metadata for {url}"))?;
    }

    Ok(executor)
}

pub fn get_executor_package_id(reference: &ExecutorReference) -> u64 {
    let mut hasher = hasher();
    match reference {
        ExecutorReference::Standard { url } => {
            url.hash(&mut hasher);
        }
        ExecutorReference::Custom { url, location } => {
            url.hash(&mut hasher);
            match location {
                Location::GitOverHttp {
                    transport,
                    git_options,
                    authentication,
                } => {
                    transport.headers().hash(&mut hasher);
                    git_options.checkout().hash(&mut hasher);
                    authentication.hash(&mut hasher);
                }
                Location::GitOverSsh {
                    git_options,
                    authentication,
                    ..
                } => {
                    git_options.checkout().hash(&mut hasher);
                    authentication.hash(&mut hasher);
                }
                Location::TarballOverHttp {
                    transport,
                    authentication,
                    ..
                } => {
                    transport.headers().hash(&mut hasher);
                    authentication.hash(&mut hasher);
                }
                Location::LocalFileSystem { .. } => {}
                Location::Npm { options } => {
                    options.version().hash(&mut hasher);
                    options.token().hash(&mut hasher);
                }
                Location::Cargo { options } => {
                    options.version().hash(&mut hasher);
                    options.token().hash(&mut hasher);
                }
                Location::Git { options } => {
                    options.checkout().hash(&mut hasher);
                }
            }
        }
    }
    hasher.finish()
}
