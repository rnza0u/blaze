use std::path::Path;

use anyhow::anyhow;
use blaze_common::{error::Result, parallelism::Parallelism};

use crate::{
    executions::{
        check::ExecutionCacheState,
        graph::{ExecutionGraph, ExecutionGraphOptions},
    },
    workspace::selection::Selection,
    GlobalOptions, SelectorSource, WorkspaceGlobals,
};

pub struct RmExecutionCacheOptions {
    target: String,
    selector_source: Option<SelectorSource>,
    depth: Option<usize>,
}

impl RmExecutionCacheOptions {
    pub fn new(target: &str) -> Self {
        Self {
            target: target.to_owned(),
            depth: Some(0),
            selector_source: None,
        }
    }

    pub fn with_selector_source(mut self, source: SelectorSource) -> Self {
        self.selector_source = Some(source);
        self
    }

    pub fn with_depth(mut self, depth: usize) -> Self {
        self.depth = Some(depth);
        self
    }
}

pub fn rm_execution_caches(
    root: &Path,
    options: RmExecutionCacheOptions,
    global_options: GlobalOptions,
) -> Result<()> {
    let globals = WorkspaceGlobals::new(root, global_options)?;

    let graph = ExecutionGraph::try_new(
        &options
            .selector_source
            .clone()
            .map(Selection::from_source)
            .unwrap_or_default(),
        &options.target,
        ExecutionGraphOptions {
            deserialization_context: globals.deserialization_context(),
            workspace: globals.workspace_handle().inner(),
            max_depth: options.depth,
        },
    )?;

    let cache = globals
        .cache()
        .ok_or_else(|| anyhow!("cache unavailable"))?;

    let results = graph.execute(Parallelism::None, |execution, _| {
        let key = execution.get_cache_key();
        Ok(cache
            .restore::<ExecutionCacheState>(&key)?
            .map(|_| cache.invalidate(&execution.get_cache_key()))
            .transpose()?
            .is_some())
    })?;

    results.fmt(&mut std::io::stdout(), |node| {
        format!(
            "{} ({})",
            node.execution.get_double(),
            match &node.result {
                Some(Ok(true)) => "cache removed".into(),
                Some(Ok(false)) => "no cache".into(),
                Some(Err(err)) => format!("failed: {err}"),
                None => "ignored".into(),
            }
        )
    })?;

    Ok(())
}
