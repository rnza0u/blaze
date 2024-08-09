use std::{convert::Infallible, str::FromStr};

use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumIter};

use crate::{selector::ProjectSelector, unit_enum_deserialize, unit_enum_from_str};

/// A target dependency object.
#[derive(Debug, Clone, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Dependency {
    target: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    projects: Option<ProjectSelector>,
    cache_propagation: CachePropagation,
    optional: bool,
}

impl Dependency {
    pub fn target(&self) -> &str {
        &self.target
    }

    pub fn projects(&self) -> Option<&ProjectSelector> {
        self.projects.as_ref()
    }

    pub fn cache_propagation(&self) -> CachePropagation {
        self.cache_propagation
    }

    pub fn optional(&self) -> bool {
        self.optional
    }
}

impl FromStr for Dependency {
    type Err = Infallible;

    fn from_str(s: &str) -> std::result::Result<Self, Infallible> {
        let (project, target) = match s.split_once(':') {
            Some((project, target)) => (Some(project), target),
            None => (None, s),
        };

        Ok(Self {
            target: target.to_string(),
            cache_propagation: CachePropagation::default(),
            optional: false,
            projects: project.map(|project| ProjectSelector::array([project])),
        })
    }
}

impl<'de> Deserialize<'de> for Dependency {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase", remote = "Dependency")]
        struct DependencyObject {
            target: String,
            projects: Option<ProjectSelector>,
            #[serde(default)]
            cache_propagation: CachePropagation,
            #[serde(default)]
            optional: bool,
        }

        #[derive(Deserialize)]
        #[serde(untagged)]
        enum DependencyDeserializationMode {
            AsString(String),
            #[serde(with = "DependencyObject")]
            Full(Dependency),
        }

        Ok(
            match DependencyDeserializationMode::deserialize(deserializer)? {
                DependencyDeserializationMode::AsString(target) => {
                    Dependency::from_str(target.as_str()).unwrap()
                }
                DependencyDeserializationMode::Full(dependency) => dependency,
            },
        )
    }
}

#[derive(Default, Debug, Clone, Copy, Hash, Serialize, PartialEq, Eq, EnumIter, Display)]
pub enum CachePropagation {
    /// Cache will invalidate if:
    /// - The dependency state changes between success, failure or canceled (can occur only when dependency is optional)
    /// - The dependency is freshly
    #[default]
    Always,

    // Dependencies will not be considered when invalidating cache
    Never,
}

unit_enum_from_str!(CachePropagation);
unit_enum_deserialize!(CachePropagation);
