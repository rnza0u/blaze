use std::collections::BTreeMap;

use crate::{logger::LogLevel, parallelism::Parallelism, selector::ProjectSelector};
use serde::{Deserialize, Serialize};

/// Global settings for the workspace.
#[derive(Default, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobalSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    default_selector: Option<ProjectSelector>,
    #[serde(default)]
    selectors: BTreeMap<String, ProjectSelector>,
    #[serde(skip_serializing_if = "Option::is_none")]
    parallelism: Option<Parallelism>,
    #[serde(skip_serializing_if = "Option::is_none")]
    log_level: Option<LogLevel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    resolution_parallelism: Option<Parallelism>,
}

impl GlobalSettings {
    pub fn default_selector(&self) -> Option<&ProjectSelector> {
        self.default_selector.as_ref()
    }

    pub fn selectors(&self) -> &BTreeMap<String, ProjectSelector> {
        &self.selectors
    }

    pub fn parallelism(&self) -> Option<Parallelism> {
        self.parallelism
    }

    pub fn log_level(&self) -> Option<LogLevel> {
        self.log_level
    }

    pub fn resolution_parallelism(&self) -> Option<Parallelism> {
        self.resolution_parallelism
    }
}
