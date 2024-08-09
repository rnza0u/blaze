use std::hash::Hash;

use serde::{Deserialize, Serialize};

use hash_value::Value;

use crate::{cache::TargetCache, dependency::Dependency, executor::ExecutorReference};

/// A single target description
#[derive(Debug, Serialize, Deserialize, Hash)]
pub struct Target {
    #[serde(skip_serializing_if = "Option::is_none")]
    executor: Option<ExecutorReference>,
    #[serde(default)]
    options: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(default)]
    dependencies: Vec<Dependency>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cache: Option<TargetCache>,
    #[serde(default)]
    stateless: bool,
}

impl Target {
    pub fn executor(&self) -> Option<&ExecutorReference> {
        self.executor.as_ref()
    }

    pub fn options(&self) -> &Value {
        &self.options
    }

    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    pub fn dependencies(&self) -> &[Dependency] {
        &self.dependencies
    }

    pub fn cache(&self) -> Option<&TargetCache> {
        self.cache.as_ref()
    }

    pub fn stateless(&self) -> bool {
        self.stateless
    }
}
