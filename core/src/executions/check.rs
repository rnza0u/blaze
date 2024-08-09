use std::time::SystemTime;

use blaze_common::{error::Result, time::system_time_as_timestamps, value::Value};

use serde::{Deserialize, Serialize};

use super::execution::TargetExecution;

#[derive(Debug, Deserialize, Serialize)]
pub struct ExecutionCacheState {
    pub nonce: u64,
    pub hash: u64,
    #[serde(with = "system_time_as_timestamps")]
    pub time: SystemTime,
    pub metadata: Value,
}

pub trait CacheInvalidationCheck {
    /// Retrieve cache state for this check.
    /// This method will be called for every cached execution after target is successfully executed, whether cache already existed or not.
    /// The returned value, if present, will be written in the cache state value using the [`Value::overwrite`] method.
    /// By default, nothing is returned.
    fn state(&self, _: &TargetExecution) -> Result<Option<Value>> {
        Ok(None)
    }

    /// Try to validate the cache state. Returns false if cache is invalidated.
    /// The implementation can internally store the new state between the invocation of this method and the invocation of the [`Self::state()`] method.
    /// This method might be called if cache does not exist yet for this [`CacheInvalidationCheck`].
    fn validate(
        &mut self,
        execution: &TargetExecution,
        current_state: &ExecutionCacheState,
    ) -> Result<bool>;
}
