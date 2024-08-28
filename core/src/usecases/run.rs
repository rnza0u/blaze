use std::{
    hash::{Hash, Hasher},
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::Context;
use blaze_common::{error::Result, logger::Logger, parallelism::Parallelism};
use colored::{ColoredString, Colorize};

use crate::{
    executions::{
        execution::{
            CachedDependencyExecution, CachedExecutionContext, CachedExecutionState,
            TargetExecution,
        },
        graph::{ExecutedGraph, ExecutedNode, ExecutionGraph, ExecutionGraphOptions},
    },
    executors::{resolve_executors, CustomResolutionContext, ExecutorContext},
    global_init,
    logging::{colorize, get_contextual_logger},
    system::{hash::hasher, locks::ProcessLock},
    workspace::selection::{Selection, SelectorSource},
    WorkspaceGlobals,
};

use super::GlobalOptions;

#[derive(Default)]
pub struct RunOptions {
    selector: Option<SelectorSource>,
    target: String,
    parallelism: Option<Parallelism>,
    is_dry_run: bool,
    display_graph: bool,
    dependencies_depth: Option<usize>,
}

impl RunOptions {
    pub fn new<T: AsRef<str>>(target: T) -> Self {
        Self {
            target: target.as_ref().to_owned(),
            ..Default::default()
        }
    }

    pub fn with_target<T: AsRef<str>>(mut self, target: T) -> Self {
        target.as_ref().clone_into(&mut self.target);
        self
    }

    pub fn with_selector_source(mut self, source: SelectorSource) -> Self {
        self.selector = Some(source);
        self
    }

    pub fn with_parallelism(mut self, parallelism: Parallelism) -> Self {
        let _ = self.parallelism.insert(parallelism);
        self
    }

    pub fn as_dry_run(mut self) -> Self {
        self.is_dry_run = true;
        self
    }

    pub fn displaying_graph(mut self) -> Self {
        self.display_graph = true;
        self
    }

    pub fn with_dependencies_depth(mut self, max: usize) -> Self {
        self.dependencies_depth = Some(max);
        self
    }
}

#[derive(Debug)]
pub enum ExecutionDetails {
    Cached,
    Noop,
    Executed { execution_time: Duration },
}

pub type RunResult = Result<ExecutedGraph<ExecutionDetails>>;

/// Run a target across a selection of projects.
pub fn run<R: AsRef<Path>>(
    root: R,
    options: RunOptions,
    globals_options: GlobalOptions,
) -> RunResult {
    let globals = WorkspaceGlobals::new(root.as_ref(), globals_options)?;
    global_init(&globals)?;

    let workspace = globals.workspace_handle().inner();
    let logger = globals.logger();
    let cache = globals.cache();

    let execution_graph = ExecutionGraph::try_new(
        &options
            .selector
            .map(Selection::from_source)
            .unwrap_or_default(),
        &options.target,
        ExecutionGraphOptions {
            workspace,
            deserialization_context: globals.deserialization_context(),
            max_depth: options.dependencies_depth,
        },
    )
    .context("could not build execution graph")?;

    let targets_to_be_executed = execution_graph.targets();

    if targets_to_be_executed.is_empty() {
        logger.warn("nothing to execute");
        return Ok(ExecutedGraph::empty());
    }

    logger.info(format!(
        "{} target(s) will be executed ({:?})",
        targets_to_be_executed.len(),
        targets_to_be_executed
    ));

    let parallelism = options
        .parallelism
        .or(workspace.settings().parallelism())
        .unwrap_or_default();

    let execution_results = if options.is_dry_run {
        execution_graph.ignore_all()?
    } else {
        let executor_references = execution_graph.get_executor_references();

        logger.info(format!(
            "{} executor reference(s) will be resolved ({:?})",
            executor_references.len(),
            executor_references
                .iter()
                .map(|reference| reference.to_string())
                .collect::<Vec<_>>()
        ));

        let executor_resolutions = Arc::new(
            resolve_executors(
                &executor_references,
                CustomResolutionContext {
                    cache,
                    workspace,
                    logger: &logger,
                },
            )
            .context("error while resolving executors")?,
        );

        let cache_arc_0 = Arc::new(cache);

        let arc_workspace = Arc::new(workspace);
        let logger_2 = logger.clone();
        let logger_3 = logger.clone();

        let log_level = globals.log_level();

        let execute = |execution: &TargetExecution| {
            let executor_reference = match execution.get_target().executor() {
                Some(reference) => reference,
                None => return Ok(ExecutionDetails::Noop),
            };

            let double = execution.get_double();
            let executor_resolution = executor_resolutions
                .get_for_reference(executor_reference)
                .unwrap();

            logger_2.debug(format!("executing target {double}..."));

            let executor_logger = get_contextual_logger(log_level, double.as_str());

            let start = Instant::now();
            Ok(ExecutionDetails::Executed {
                execution_time: executor_resolution
                    .executor()
                    .execute(
                        ExecutorContext {
                            project: &execution.get_project(),
                            workspace: &arc_workspace.clone(),
                            logger: &executor_logger,
                            target: execution.get_target_name(),
                        },
                        execution.get_target().options().clone(),
                    )
                    .with_context(|| format!("executor failed for target {double}"))
                    .map(|_| start.elapsed())?,
            })
        };

        fn maybe_locked<T, F>(
            root: &Path,
            execution: &TargetExecution,
            logger: Logger,
            f: F,
        ) -> Result<T>
        where
            F: FnOnce() -> T,
        {
            let target = execution.get_target();
            if target.stateless() {
                return Ok(f());
            }
            let double = execution.get_double();
            let mut hasher = hasher();
            double.hash(&mut hasher);
            let mut lock = ProcessLock::try_new(root, hasher.finish())?;
            lock.on_wait(move || {
                logger.warn(format!(
                    "waiting for {double} to terminate in another process"
                ))
            });
            let result = lock.locked(f)?;
            Ok(result)
        }

        match cache_arc_0.as_ref() {
            None => execution_graph.execute(parallelism, |execution, _| {
                maybe_locked(arc_workspace.root(), execution, logger_3.clone(), || {
                    let result = execute(execution);

                    let double = execution.get_double();

                    match &result {
                        Ok(_) => logger_3.debug(format!("target {double} is done")),
                        Err(err) => logger_3.error(format!("target {double} has failed: {err:?}")),
                    };

                    result
                })?
            })?,
            Some(cache) => execution_graph
                .execute(parallelism, |execution, child_executions| {
                    maybe_locked(arc_workspace.root(), execution, logger_2.clone(), || {
                        let double = execution.get_double();

                        let cached_execution_result = execution
                            .cached(
                                child_executions
                                    .iter()
                                    .map(|child| CachedDependencyExecution {
                                        double: child.execution.get_double(),
                                        state: child.result,
                                        source: child.dependency.as_ref(),
                                    })
                                    .collect::<Vec<_>>()
                                    .as_slice(),
                                execution
                                    .get_target()
                                    .executor()
                                    .and_then(|reference| {
                                        executor_resolutions.get_for_reference(reference)
                                    })
                                    .and_then(|resolution| resolution.resolution_cache()),
                                CachedExecutionContext {
                                    cache,
                                    logger: &logger,
                                    workspace: &arc_workspace,
                                },
                                || execute(execution),
                            )
                            .with_context(|| {
                                format!("cached execution failed unexpectedly for target {double}")
                            });

                        let double = execution.get_double();

                        match &cached_execution_result {
                            Ok(CachedExecutionState::Cached(hash)) => {
                                logger_2.debug(format!("target {double} is cached ({hash:0>16x})"))
                            }
                            Ok(_) => logger_2.debug(format!("target {double} is done")),
                            Err(err) => {
                                logger_2.error(format!("target {double} has failed: {err:?}"))
                            }
                        };

                        cached_execution_result
                    })?
                })
                .map(|executed_graph| {
                    executed_graph.map_inner(
                        |cached_execution_result| match cached_execution_result {
                            CachedExecutionState::Cached(_) => ExecutionDetails::Cached,
                            CachedExecutionState::New(_, details)
                            | CachedExecutionState::NoCache(details) => details,
                        },
                    )
                })?,
        }
    };

    if options.display_graph {
        print!("\nExecution graph results:\n\n");
        execution_results.fmt(
            &mut std::io::stdout(),
            |execution_result| match &execution_result.result {
                Some(Ok(ExecutionDetails::Executed { execution_time })) => format!(
                    "{} (executed in {execution_time:?})",
                    colorize(execution_result.execution.get_double(), |colored| colored
                        .bold()
                        .bright_green())
                ),
                Some(Ok(ExecutionDetails::Noop)) => format!(
                    "{} (done)",
                    colorize(
                        execution_result.execution.get_double(),
                        ColoredString::green
                    )
                ),
                Some(Ok(ExecutionDetails::Cached)) => format!(
                    "{} (cached)",
                    colorize(
                        execution_result.execution.get_double(),
                        ColoredString::green
                    )
                ),
                Some(Err(err)) => format!(
                    "{} (failed: {})",
                    colorize(
                        execution_result.execution.get_double(),
                        ColoredString::bright_red
                    ),
                    err.root_cause()
                ),
                None => format!("{} (ignored)", execution_result.execution.get_double()),
            },
        )?;
        println!();
    }

    let stats = RunStats::new(&execution_results);
    logger.debug(format!("executed target(s): {}", stats.executed));
    logger.debug(format!("failed target(s): {}", stats.failed));
    logger.debug(format!("cached target(s): {}", stats.cached));
    logger.debug(format!("pending target(s): {}", stats.pending));

    Ok(execution_results)
}

#[derive(Debug, Default)]
struct RunStats {
    executed: usize,
    cached: usize,
    failed: usize,
    pending: usize,
}

impl RunStats {
    fn new(graph: &ExecutedGraph<ExecutionDetails>) -> Self {
        let mut stats: RunStats = Default::default();

        for result in graph.execution().values() {
            *match result {
                ExecutedNode {
                    result: Some(Ok(ExecutionDetails::Cached)),
                    ..
                } => &mut stats.cached,
                ExecutedNode {
                    result: Some(Ok(ExecutionDetails::Executed { .. } | ExecutionDetails::Noop)),
                    ..
                } => &mut stats.executed,
                ExecutedNode {
                    result: Some(Err(_)),
                    ..
                } => &mut stats.failed,
                ExecutedNode { result: None, .. } => &mut stats.pending,
            } += 1;
        }

        stats
    }
}
