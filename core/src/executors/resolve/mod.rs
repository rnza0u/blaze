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
pub mod builder;

use std::{
    collections::{HashMap, HashSet},
    hash::{Hash, Hasher},
};

use anyhow::Context;
use blaze_common::{
    error::Result,
    executor::{ExecutorReference, Location},
    logger::Logger,
    value::Value,
    workspace::Workspace,
};
use possibly::possibly;
use rand::{thread_rng, RngCore};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{
    executors::DynExecutor,
    system::{hash::hasher, locks::ProcessLock, parallel_executor::ParallelRunner},
    workspace::cache_store::CacheStore,
};

use standard::resolve_standard_executor;

use self:: resolver::{resolver_for_location, ExecutorResolver};

/// Extra data needed in order to resolve an executor.
#[derive(Clone, Copy)]
pub struct CustomResolutionContext<'a> {
    pub workspace: &'a Workspace,
    pub cache: Option<&'a CacheStore>,
    pub logger: &'a Logger,
}

pub struct ResolvedExecutors {
    executors: HashMap<u64, ResolvedExecutor>,
}

impl ResolvedExecutors {
    pub fn get_for_reference(&self, reference: &ExecutorReference) -> Option<&ResolvedExecutor> {
        self.executors.get(&get_executor_package_id(reference))
    }
}

pub enum ResolvedExecutor {
    Standard(DynExecutor),
    Custom(CustomExecutorResolution),
}

impl ResolvedExecutor {
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
            HashMap::<u64, ResolvedExecutor>::with_capacity(references_by_package_id.len());
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
                                    ResolvedExecutor::Standard(
                                        resolve_standard_executor(url).with_context(|| {
                                            format!("standard executor \"{url}\" does not exist")
                                        })?,
                                    ),
                                ))
                            }
                            ExecutorReference::Custom { url, location } => (url, location),
                        };

                        let resolution_lock = ProcessLock::try_new(context.workspace.root(), package_id)?;

                        let custom_executor_resolution = resolution_lock.locked(|| {
                            resolve_custom_executor(url, location, package_id, context)
                        })??;

                        match custom_executor_resolution.state {
                            ExecutorCacheState::New | ExecutorCacheState::Updated => {
                                return Ok((
                                    package_id,
                                    ResolvedExecutor::Custom(custom_executor_resolution),
                                ))
                            }
                            ExecutorCacheState::Cached => {
                                let _ = cached.insert(custom_executor_resolution);
                            }
                        }
                    }
                    Ok((package_id, ResolvedExecutor::Custom(cached.unwrap())))
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

#[derive(Serialize, Deserialize)]
struct ResolutionCacheMetadata {
    resolution_state: Value,
    nonce: u64,
}

fn resolve_custom_executor(
    url: &Url,
    location: &Location,
    package_id: u64,
    context: CustomResolutionContext<'_>,
) -> Result<CustomExecutorResolution> {
    
    let state_key = format!("executors/{package_id}");
    let resolver: Box<dyn ExecutorResolver> = resolver_for_location(location.clone(), context);
    
    let maybe_existing_metadata = context
        .cache
        .and_then(|cache| cache.restore::<ResolutionCacheMetadata>(&state_key).transpose())
        .transpose()
        .with_context(|| format!("failed to restore resolution state for executor {url}, cache might be corrupted"))?;

    let gen_nonce = || thread_rng().next_u64();

    let (nonce, maybe_new_state, executor) = match &maybe_existing_metadata {
        Some(existing_metadata) => {
            let update = resolver.update(url, &existing_metadata.resolution_state)
                .with_context(|| format!("failed to update executor {url}"))?;

            (
                if update.state.is_some() { 
                    gen_nonce()
                } else {  
                    existing_metadata.nonce
                }, 
                update.state, 
                update.executor
            )
        },
        None => {
            let resolution = resolver.resolve(url)
                .with_context(|| format!("failed to resolve executor {url}"))?;

            (
                gen_nonce(),
                Some(resolution.state),
                resolution.executor
            )
        }
    };

    if let Some((store, new_state)) = context.cache.zip(maybe_new_state.as_ref()){
        store.cache(&state_key, &ResolutionCacheMetadata {
            nonce,
            resolution_state: new_state.to_owned()
        })
        .with_context(|| format!("failed to cache executor metadata for {url}"))?
    }

    Ok(CustomExecutorResolution {
        executor,
        nonce,
        state: if maybe_existing_metadata.is_none(){
            ExecutorCacheState::Cached
        } else if maybe_new_state.is_some(){
            ExecutorCacheState::Updated
        } else {
            ExecutorCacheState::New
        }
    })
   
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
