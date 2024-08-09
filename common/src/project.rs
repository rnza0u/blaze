use serde::{Deserialize, Serialize};

use crate::{configuration_file::ConfigurationFileFormat, target::Target};

use std::{
    collections::BTreeMap,
    hash::Hash,
    path::{Path, PathBuf},
};

/// A project within the workspace
#[derive(Debug, Serialize, Deserialize, Hash)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    name: String,
    root: PathBuf,
    configuration_file_path: PathBuf,
    configuration_file_format: ConfigurationFileFormat,
    #[serde(flatten)]
    configuration: ProjectConfiguration,
}

impl Project {
    pub fn from_configuration_and_metadata<P: AsRef<Path>>(
        name: &str,
        source: (P, ConfigurationFileFormat),
        configuration: ProjectConfiguration,
    ) -> Self {
        let mut root = source.0.as_ref().to_owned();
        let _ = root.pop();

        Project {
            name: name.to_owned(),
            root,
            configuration_file_path: source.0.as_ref().to_owned(),
            configuration_file_format: source.1,
            configuration,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn configuration_file_path(&self) -> &Path {
        &self.configuration_file_path
    }

    pub fn configuration_file_format(&self) -> ConfigurationFileFormat {
        self.configuration_file_format
    }

    pub fn targets(&self) -> &BTreeMap<String, Target> {
        &self.configuration.targets
    }
}

#[derive(Debug, Serialize, Hash, Deserialize)]
pub struct ProjectConfiguration {
    #[serde(default)]
    targets: BTreeMap<String, Target>,
}
