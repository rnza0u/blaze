use std::collections::HashMap;

use blaze_common::{
    cache::EnvChangesOptions,
    error::Result,
    logger::Logger,
    value::{to_value, Value},
};
use serde::{Deserialize, Serialize};

use crate::system::env::Env;

use super::{
    check::{CacheInvalidationCheck, ExecutionCacheState},
    execution::TargetExecution,
};

pub struct EnvChangesCheck<'a> {
    options: &'a EnvChangesOptions,
    logger: &'a Logger,
    computed_state: Option<WatchedVariablesState>,
}

#[derive(Deserialize, Serialize, PartialEq, Eq)]
struct WatchedVariablesState(HashMap<String, String>);

const WATCHED_VARIABLES_STATE_KEY: &str = "env";

impl<'a> EnvChangesCheck<'a> {
    pub fn new(options: &'a EnvChangesOptions, logger: &'a Logger) -> Self {
        Self {
            logger,
            options,
            computed_state: None,
        }
    }

    fn get_state(&self) -> Result<WatchedVariablesState> {
        Ok(WatchedVariablesState(
            self.options
                .variables()
                .iter()
                .filter_map(|key| {
                    Env::get_as_str(key.as_str())
                        .map(|res| res.map(|value| (key.to_owned(), value)))
                        .transpose()
                })
                .collect::<Result<_>>()?,
        ))
    }
}

impl CacheInvalidationCheck for EnvChangesCheck<'_> {
    fn validate(&mut self, _: &TargetExecution, state: &ExecutionCacheState) -> Result<bool> {
        let maybe_old_state = state
            .metadata
            .at(WATCHED_VARIABLES_STATE_KEY)
            .map(WatchedVariablesState::deserialize)
            .transpose()?;

        let old_state = match maybe_old_state {
            Some(state) => state,
            None => return Ok(false),
        };

        let new_state = self.get_state()?;

        let is_state_unchanged = new_state == old_state;

        if !is_state_unchanged {
            for name in old_state.0.keys() {
                if !new_state.0.contains_key(name) {
                    self.logger
                        .debug(format!("{name} was unset (previously set)"));
                }
            }
            for (name, value) in &new_state.0 {
                let old_value = match old_state.0.get(name) {
                    Some(old_value) => old_value,
                    None => {
                        self.logger
                            .debug(format!("{name} was set (previously unset)"));
                        continue;
                    }
                };
                if old_value == value {
                    continue;
                }
                self.logger.debug(format!("{name} has new value"));
            }
        }

        let _ = self.computed_state.insert(new_state);

        Ok(is_state_unchanged)
    }

    fn state(&self, _: &TargetExecution) -> Result<Option<Value>> {
        Ok(Some(Value::object([(
            WATCHED_VARIABLES_STATE_KEY,
            if let Some(computed) = &self.computed_state {
                to_value(computed)?
            } else {
                to_value(self.get_state()?)?
            },
        )])))
    }
}
