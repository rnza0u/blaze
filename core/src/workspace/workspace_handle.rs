use anyhow::{anyhow, Context};
use jsonschema::JSONSchema;
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use blaze_common::{
    error::Result,
    util::normalize_path,
    value::Value,
    workspace::{Workspace, WorkspaceConfiguration},
};

use super::{
    configurations::{
        deserialize_configuration, infer_configuration_file_path, DeserializationContext,
    },
    schemas::{create_schema, validate_json},
    template::TemplateData,
};

/// The workspace configuration filename.
pub const WORKSPACE_FILENAME: &str = "workspace";

pub static WORKSPACE_JSON_SCHEMA: Lazy<JSONSchema> =
    Lazy::new(|| create_schema!("workspace-schema.json"));

pub struct OpenWorkspaceOptions<'a> {
    pub template_data: &'a TemplateData<'a>,
    pub jpath: &'a HashSet<PathBuf>,
}

/// Provides file system related functions and validation.
pub struct WorkspaceHandle(Workspace);

impl WorkspaceHandle {
    /// Check if a workspace exists in a specific directory.
    pub fn exists_at_root<P: AsRef<Path>>(root: P) -> Result<bool> {
        Ok(infer_configuration_file_path(root.as_ref(), WORKSPACE_FILENAME)?.is_some())
    }

    /// Load a workspace from its root directory. This will infer the correct configuration file path.
    pub fn from_root<P: AsRef<Path>>(root: P, options: OpenWorkspaceOptions<'_>) -> Result<Self> {
        let (file_type, configuration_file_path) =
            infer_configuration_file_path(root.as_ref(), WORKSPACE_FILENAME)?
                .ok_or_else(|| anyhow!("could not find any workspace configuration file"))?;
        let configuration_file_path = normalize_path(configuration_file_path)?;

        let workspace_raw_value = deserialize_configuration::<Value>(
            &configuration_file_path,
            file_type,
            DeserializationContext {
                jpath: options.jpath,
                template_data: options.template_data,
            },
        )
        .with_context(|| {
            format!(
                "could not deserialize workspace configuration file at {}.",
                configuration_file_path.display()
            )
        })?;

        validate_json(&WORKSPACE_JSON_SCHEMA, &workspace_raw_value)?;

        let deserialized =
            WorkspaceConfiguration::deserialize(workspace_raw_value).with_context(|| {
                format!(
                    "bad workspace configuration at {}",
                    configuration_file_path.display()
                )
            })?;

        let workspace = Workspace::from_configuration_and_metadata(
            (configuration_file_path, file_type),
            deserialized,
        );

        Ok(Self(workspace))
    }

    pub fn unwrap_inner(self) -> Workspace {
        self.0
    }

    pub fn inner(&self) -> &Workspace {
        &self.0
    }
}
