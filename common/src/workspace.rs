use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

use serde::{de::Error, Deserialize, Serialize};

use crate::{
    configuration_file::ConfigurationFileFormat, settings::GlobalSettings, util::normalize_path,
};

/// Main workspace configuration object.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Workspace {
    root: PathBuf,
    configuration_file_path: PathBuf,
    configuration_file_format: ConfigurationFileFormat,
    #[serde(flatten)]
    configuration: WorkspaceConfiguration,
}

#[derive(Debug, Serialize)]
pub struct ProjectRef {
    path: PathBuf,
    tags: BTreeSet<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
}

impl ProjectRef {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn tags(&self) -> &BTreeSet<String> {
        &self.tags
    }

    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}

impl From<PathBuf> for ProjectRef {
    fn from(path: PathBuf) -> Self {
        Self {
            description: None,
            tags: BTreeSet::new(),
            path,
        }
    }
}

impl<'de> Deserialize<'de> for ProjectRef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(remote = "ProjectRef")]
        struct ProjectRefAsObject {
            path: PathBuf,
            #[serde(default)]
            tags: BTreeSet<String>,
            description: Option<String>,
        }

        #[derive(Deserialize)]
        #[serde(untagged)]
        enum ProjectRefDeserializationModes {
            SinglePath(PathBuf),
            #[serde(with = "ProjectRefAsObject")]
            Full(ProjectRef),
        }

        Ok(
            match ProjectRefDeserializationModes::deserialize(deserializer)? {
                ProjectRefDeserializationModes::SinglePath(path) => {
                    normalize_path(path).map_err(D::Error::custom)?.into()
                }
                ProjectRefDeserializationModes::Full(mut project_ref) => {
                    project_ref.path =
                        normalize_path(&project_ref.path).map_err(D::Error::custom)?;
                    project_ref
                }
            },
        )
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkspaceConfiguration {
    name: String,
    #[serde(default)]
    projects: BTreeMap<String, ProjectRef>,
    #[serde(default)]
    settings: GlobalSettings,
}

impl Workspace {
    /// Create a [`Workspace`] from configuration file metadata and deserialized content.
    pub fn from_configuration_and_metadata<P: AsRef<Path>>(
        source: (P, ConfigurationFileFormat),
        configuration: WorkspaceConfiguration,
    ) -> Self {
        let mut root = source.0.as_ref().to_path_buf();
        let _ = root.pop();

        Self {
            root,
            configuration_file_path: source.0.as_ref().to_path_buf(),
            configuration_file_format: source.1,
            configuration,
        }
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

    pub fn name(&self) -> &str {
        &self.configuration.name
    }

    pub fn projects(&self) -> &BTreeMap<String, ProjectRef> {
        &self.configuration.projects
    }

    pub fn settings(&self) -> &GlobalSettings {
        &self.configuration.settings
    }
}
