use blaze_common::{error::Result, logger::Logger, value::Value};
use serde::{Deserialize, Serialize};

use crate::executors::ExecutorCacheState;

use super::{
    check::{CacheInvalidationCheck, ExecutionCacheState},
    execution::TargetExecution,
};

pub struct ExecutorUpdateCheck<'a> {
    state: ExecutorCacheState,
    nonce: u64,
    logger: &'a Logger,
}

#[derive(Serialize, Deserialize)]
struct State {
    nonce: u64,
}

const EXECUTOR_STATE_KEY: &str = "executor_state";

impl<'a> ExecutorUpdateCheck<'a> {
    pub fn new(state: ExecutorCacheState, nonce: u64, logger: &'a Logger) -> Self {
        Self {
            logger,
            state,
            nonce,
        }
    }
}

impl CacheInvalidationCheck for ExecutorUpdateCheck<'_> {
    fn state(&self, _: &TargetExecution) -> Result<Option<Value>> {
        Ok(Some(Value::object([(
            EXECUTOR_STATE_KEY,
            Value::unsigned(self.nonce),
        )])))
    }

    fn validate(&mut self, _: &TargetExecution, cache_state: &ExecutionCacheState) -> Result<bool> {
        let maybe_state = cache_state
            .metadata
            .at(EXECUTOR_STATE_KEY)
            .map(State::deserialize)
            .and_then(|state| state.ok());

        if maybe_state.is_none() {
            return Ok(false);
        }

        let current_state = maybe_state.unwrap();

        let valid =
            matches!(self.state, ExecutorCacheState::Cached) && self.nonce == current_state.nonce;

        if !valid {
            self.logger
                .debug("executor was updated, target cache will be invalidated");
        }

        Ok(valid)
    }
}
