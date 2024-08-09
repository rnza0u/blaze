use std::collections::HashMap;

use blaze_common::{
    dependency::CachePropagation,
    error::Result,
    value::{to_value, Value},
};
use possibly::possibly;
use serde::{Deserialize, Serialize};

use crate::executions::execution::CachedExecutionState;

use super::{
    check::{CacheInvalidationCheck, ExecutionCacheState},
    execution::{CachedDependencyExecution, TargetExecution},
};

pub struct PropagatingChildrenCheck<'a, T> {
    children: &'a [CachedDependencyExecution<'a, T>],
    computed_state: Option<State>,
}

impl<'a, T> PropagatingChildrenCheck<'a, T> {
    pub fn new(children: &'a [CachedDependencyExecution<T>]) -> Self {
        Self {
            children,
            computed_state: None,
        }
    }

    fn get_state(&self) -> State {
        State {
            children: self.children.iter()
            .filter_map(|child| {

                if child.source.cache_propagation() == CachePropagation::Never {
                    return None
                }

                possibly!(
                    child.state,
                    Some(Ok(CachedExecutionState::Cached(hash)|CachedExecutionState::New(hash, _))) => (child.double.to_owned(), *hash)
                )
            })
            .collect()
        }
    }
}

const CHILD_EXECUTIONS_KEY: &str = "child-executions";

#[derive(Serialize, Deserialize)]
struct State {
    children: HashMap<String, u64>,
}

impl<'a, T> CacheInvalidationCheck for PropagatingChildrenCheck<'a, T> {
    fn state(&self, _: &TargetExecution) -> Result<Option<Value>> {
        Ok(Some(Value::object([(
            CHILD_EXECUTIONS_KEY,
            match &self.computed_state {
                Some(state) => to_value(state)?,
                None => to_value(self.get_state())?,
            },
        )])))
    }

    fn validate(&mut self, _: &TargetExecution, cache_state: &ExecutionCacheState) -> Result<bool> {
        let maybe_state = cache_state
            .metadata
            .at(CHILD_EXECUTIONS_KEY)
            .map(State::deserialize)
            .and_then(|state| state.ok());

        if maybe_state.is_none() {
            return Ok(false);
        }

        let old_state = maybe_state.unwrap();
        let new_state = self.get_state();

        let valid = new_state.children == old_state.children;

        self.computed_state = Some(new_state);

        Ok(valid)
    }
}
