use std::time::{Duration, SystemTime};

use anyhow::bail;
use blaze_common::{
    cache::{TimeUnit, TtlOptions},
    error::Result,
    logger::Logger,
    time::system_time_as_timestamps,
    value::{to_value, Value},
};
use serde::{Deserialize, Serialize};

use crate::system::time::now;

use super::{
    check::{CacheInvalidationCheck, ExecutionCacheState},
    execution::TargetExecution,
};

#[derive(Serialize, Deserialize)]
struct TtlState {
    #[serde(with = "system_time_as_timestamps")]
    at: SystemTime,
}

pub struct TtlCheck<'a> {
    options: &'a TtlOptions,
    logger: &'a Logger,
}

impl<'a> TtlCheck<'a> {
    pub fn new(options: &'a TtlOptions, logger: &'a Logger) -> Self {
        Self { options, logger }
    }
}

const TTL_STATE_KEY: &str = "ttl";

impl CacheInvalidationCheck for TtlCheck<'_> {
    fn state(&self, _: &TargetExecution) -> Result<Option<Value>> {
        Ok(Some(Value::object([(
            TTL_STATE_KEY,
            to_value(TtlState { at: now() })?,
        )])))
    }

    fn validate(
        &mut self,
        execution: &TargetExecution,
        state: &ExecutionCacheState,
    ) -> Result<bool> {
        let now = now();
        let maybe_last_state = state
            .metadata
            .at(TTL_STATE_KEY)
            .map(TtlState::deserialize)
            .transpose()?;

        let last_state = match maybe_last_state {
            Some(state) => state,
            None => return Ok(false),
        };

        let amount_u64: u64 = self.options.amount().try_into()?;

        if amount_u64 == 0 {
            bail!("ttl cannot be zero")
        }

        let duration = match self.options.unit() {
            TimeUnit::Milliseconds => Duration::from_millis(amount_u64),
            TimeUnit::Seconds => Duration::from_secs(amount_u64),
            TimeUnit::Minutes => Duration::from_secs(amount_u64 * 60),
            TimeUnit::Hours => Duration::from_secs(amount_u64 * 60 * 60),
            TimeUnit::Days => Duration::from_secs(amount_u64 * 60 * 60 * 24),
        };

        let is_expired = last_state.at + duration <= now;

        if is_expired {
            self.logger.debug(format!(
                "target {execution} TTL has expired (elapsed={})",
                {
                    let elapsed = now.duration_since(last_state.at)?;
                    match self.options.unit() {
                        TimeUnit::Milliseconds => format!("{}ms", elapsed.as_millis()),
                        TimeUnit::Seconds => format!("{}s", elapsed.as_secs()),
                        TimeUnit::Minutes => format!("{}m", elapsed.as_secs() / 60),
                        TimeUnit::Hours => format!("{}h", elapsed.as_secs() / 60 / 60),
                        TimeUnit::Days => format!("{}d", elapsed.as_secs() / 60 / 60 / 24),
                    }
                }
            ));
        }

        Ok(!is_expired)
    }
}
