use anyhow::bail;
use rand::{thread_rng, RngCore};
use std::{
    convert::identity,
    fmt::Display,
    hash::{Hash, Hasher},
    sync::Arc,
};

use blaze_common::{
    dependency::Dependency, error::Result, logger::Logger, project::Project, target::Target,
    value::Value, workspace::Workspace,
};

use crate::{
    executions::{
        check::{CacheInvalidationCheck, ExecutionCacheState},
        command_fails::CommandFailsCheck,
        file_changes::InputFileChangesCheck,
        files_missing::FilesMissingCheck,
        propagating_children::PropagatingChildrenCheck,
        ttl::TtlCheck,
    },
    executors::ExecutorCacheState,
    system::{hash::hasher, time::now},
    workspace::cache_store::CacheStore,
};

use super::{
    env_changes::EnvChangesCheck, executor_update::ExecutorUpdateCheck,
    file_changes::OutputFileChangesCheck,
};

const EXECUTIONS_STATE_KEY_PREFIX: &str = "executions";

#[derive(Debug)]
pub struct TargetExecution {
    project: Arc<Project>,
    target_name: String,
}

impl Display for TargetExecution {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.get_double())
    }
}

impl PartialEq for TargetExecution {
    fn eq(&self, other: &Self) -> bool {
        self.project.name() == other.project.name() && self.target_name == other.target_name
    }
}

pub enum CachedExecutionState<T> {
    Cached(u64),
    New(u64, T),
    NoCache(T),
}

pub struct CachedDependencyExecution<'a, T> {
    pub double: String,
    pub state: Option<&'a Result<CachedExecutionState<T>>>,
    pub source: &'a Dependency,
}

#[derive(Clone, Copy)]
pub struct CachedExecutionContext<'a> {
    pub cache: &'a CacheStore,
    pub logger: &'a Logger,
    pub workspace: &'a Workspace,
}

impl TargetExecution {
    /// Create a new [`TargetExecution`] object from raw data.
    pub fn try_new(project: Arc<Project>, target_name: &str) -> Option<TargetExecution> {
        let _ = project.as_ref().targets().get(target_name)?;

        Some(TargetExecution {
            target_name: target_name.to_owned(),
            project,
        })
    }

    pub fn get_cache_key(&self) -> String {
        let double = self.get_double();
        format!("{EXECUTIONS_STATE_KEY_PREFIX}/{double}")
    }

    /// Run the function *f* if this execution is not cached.
    /// The function must return a result so that this wrapper can update the target execution cache state according to success or failure.
    pub fn cached<T, F>(
        &self,
        child_executions: &[CachedDependencyExecution<T>],
        executor_cache: Option<(ExecutorCacheState, u64)>,
        context: CachedExecutionContext<'_>,
        f: F,
    ) -> Result<CachedExecutionState<T>>
    where
        F: FnOnce() -> Result<T>,
    {
        let target = self.get_target();

        let target_cache = match target.cache() {
            Some(c) => c,
            None => return Ok(CachedExecutionState::NoCache(f()?)),
        };

        let cache_state_key = self.get_cache_key();

        let invalidation_strategy = target_cache.invalidate_when();
        let mut checks: Vec<(&str, Box<dyn CacheInvalidationCheck>)> = vec![(
            "child cache propagation",
            Box::new(PropagatingChildrenCheck::new(child_executions)),
        )];
        checks.extend(
            vec![
                executor_cache.map(|(state, nonce)| {
                    (
                        "executor was updated",
                        Box::new(ExecutorUpdateCheck::new(state, nonce, context.logger))
                            as Box<dyn CacheInvalidationCheck>,
                    )
                }),
                invalidation_strategy.expired().map(|options| {
                    (
                        "ttl expired",
                        Box::new(TtlCheck::new(options, context.logger))
                            as Box<dyn CacheInvalidationCheck>,
                    )
                }),
                invalidation_strategy.files_missing().map(|options| {
                    (
                        "files were missing",
                        Box::new(FilesMissingCheck::new(options, context.logger))
                            as Box<dyn CacheInvalidationCheck>,
                    )
                }),
                invalidation_strategy.input_changes().map(|options| {
                    (
                        "input file(s) changed",
                        Box::new(InputFileChangesCheck::new(options, context.logger))
                            as Box<dyn CacheInvalidationCheck>,
                    )
                }),
                invalidation_strategy.output_changes().map(|options| {
                    (
                        "output file(s) changed",
                        Box::new(OutputFileChangesCheck::new(options, context.logger))
                            as Box<dyn CacheInvalidationCheck>,
                    )
                }),
                invalidation_strategy.command_fails().map(|options| {
                    (
                        "cache invalidation command failed",
                        Box::new(CommandFailsCheck::new(options))
                            as Box<dyn CacheInvalidationCheck>,
                    )
                }),
                invalidation_strategy.env_changes().map(|options| {
                    (
                        "environment variables changed",
                        Box::new(EnvChangesCheck::new(options, context.logger))
                            as Box<dyn CacheInvalidationCheck>,
                    )
                }),
            ]
            .into_iter()
            .flatten(),
        );

        let mut hasher = hasher();
        self.project.root().hash(&mut hasher);
        self.get_target().hash(&mut hasher);

        let mut hasher_before_nonce = hasher.clone();

        let execute_and_cache =
            |checks: Vec<Box<dyn CacheInvalidationCheck>>| -> Result<CachedExecutionState<T>> {
                let execution_result = match f() {
                    Ok(value) => value,
                    Err(err) => {
                        context.cache.invalidate(&cache_state_key)?;
                        bail!(err)
                    }
                };

                let mut metadata = Value::default();
                for check in checks {
                    if let Some(state) = check.state(self)? {
                        metadata.overwrite(state);
                    }
                }

                let nonce = thread_rng().next_u64();
                nonce.hash(&mut hasher_before_nonce);
                let new_hash = hasher_before_nonce.finish();
                context.cache.cache(
                    &cache_state_key,
                    &ExecutionCacheState {
                        nonce,
                        hash: new_hash,
                        metadata,
                        time: now(),
                    },
                )?;

                Ok(CachedExecutionState::New(new_hash, execution_result))
            };

        let maybe_last_execution = context
            .cache
            .restore::<ExecutionCacheState>(&cache_state_key)?;

        if maybe_last_execution.is_none() {
            context.logger.debug(format!("{self} was not cached."));
            return execute_and_cache(checks.into_iter().map(|(_, check)| check).collect());
        }

        let last_execution_state = maybe_last_execution.unwrap();

        let is_cache_valid = checks
            .iter_mut()
            .map(|(reason, check)| {
                let validated = check.validate(self, &last_execution_state)?;

                if !validated {
                    context
                        .logger
                        .debug(format!("{self} cache will be invalidated ({reason})"))
                }

                Ok(validated)
            })
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .all(identity);

        if !is_cache_valid {
            return execute_and_cache(checks.into_iter().map(|(_, check)| check).collect());
        }

        last_execution_state.nonce.hash(&mut hasher);
        let current_execution_hash = hasher.finish();

        if last_execution_state.hash != current_execution_hash {
            context.logger.debug(format!(
                "{self} configuration changed, cache will be invalidated"
            ));
            return execute_and_cache(checks.into_iter().map(|(_, check)| check).collect());
        }

        Ok(CachedExecutionState::Cached(current_execution_hash))
    }

    /// Synchronized pointer to the project data.
    pub fn get_project(&self) -> Arc<Project> {
        self.project.clone()
    }

    /// The name of the target for this execution.
    pub fn get_target_name(&self) -> &str {
        &self.target_name
    }

    /// Returns a formatted string to identify this execution: <project-name>:<target-name>
    pub fn get_double(&self) -> String {
        [self.project.name(), self.target_name.as_str()].join(":")
    }

    pub fn get_target(&self) -> &Target {
        &self.project.targets()[&self.target_name]
    }
}
