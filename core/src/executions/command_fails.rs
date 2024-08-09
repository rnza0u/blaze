use std::{collections::HashMap, time::UNIX_EPOCH};

use blaze_common::{cache::CommandFailsOptions, error::Result, value::Value};

use crate::system::process::{Process, ProcessOptions};

use super::{
    check::{CacheInvalidationCheck, ExecutionCacheState},
    execution::TargetExecution,
};

pub const PROJECT: &str = "BLAZE_PROJECT";
pub const TARGET: &str = "BLAZE_TARGET";
pub const OPTIONS: &str = "BLAZE_OPTIONS";
pub const LAST_EXECUTION_TIME: &str = "BLAZE_LAST_EXECUTION_TIME";
pub const FRESH_EXECUTION: &str = "BLAZE_FRESH_EXECUTION";

pub struct CommandFailsCheck<'a> {
    options: &'a CommandFailsOptions,
    is_launched: bool,
}

impl<'a> CommandFailsCheck<'a> {
    pub fn new(options: &'a CommandFailsOptions) -> Self {
        Self {
            options,
            is_launched: false,
        }
    }

    fn launch(
        &self,
        execution: &TargetExecution,
        maybe_state: Option<&ExecutionCacheState>,
    ) -> Result<bool> {
        let mut environment = HashMap::from([
            (
                OPTIONS.into(),
                serde_json::to_string(execution.get_target().options())?,
            ),
            (TARGET.into(), execution.get_target_name().to_owned()),
            (PROJECT.into(), execution.get_project().name().to_owned()),
        ]);

        if let Some(state) = maybe_state {
            environment.insert(
                LAST_EXECUTION_TIME.into(),
                state
                    .time
                    .duration_since(UNIX_EPOCH)?
                    .as_millis()
                    .to_string(),
            );
        } else {
            environment.insert(FRESH_EXECUTION.into(), true.to_string());
        }

        environment.extend(self.options.environment().clone());

        let process = Process::run_with_options(
            self.options.program(),
            self.options.arguments(),
            ProcessOptions {
                environment,
                display_output: self.options.verbose(),
                cwd: Some(
                    self.options
                        .cwd()
                        .unwrap_or(execution.get_project().root())
                        .to_owned(),
                ),
            },
        )?;

        let status = process.wait()?;

        Ok(status.success)
    }
}

impl CacheInvalidationCheck for CommandFailsCheck<'_> {
    fn validate(
        &mut self,
        execution: &TargetExecution,
        state: &ExecutionCacheState,
    ) -> Result<bool> {
        let invalidated = self.launch(execution, Some(state));
        self.is_launched = true;
        invalidated
    }

    fn state(&self, execution: &TargetExecution) -> Result<Option<Value>> {
        if !self.is_launched {
            self.launch(execution, None)?;
        }
        Ok(None)
    }
}
