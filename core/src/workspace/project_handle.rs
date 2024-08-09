use std::path::Path;

use blaze_common::{
    error::Result,
    project::{Project, ProjectConfiguration},
    util::normalize_path,
    value::Value,
};
use jsonschema::JSONSchema;
use once_cell::sync::Lazy;
use serde::Deserialize;

use crate::workspace::schemas::create_schema;

use super::{
    configurations::{
        deserialize_configuration, infer_configuration_file_path, DeserializationContext,
    },
    schemas::validate_json,
};
use anyhow::{anyhow, Context};

/// Project configuration file name.
pub const PROJECT_FILENAME: &str = "project";

pub static PROJECT_JSON_SCHEMA: Lazy<JSONSchema> =
    Lazy::new(|| create_schema!("project-schema.json"));

/// A handle to a [`Project`] struct which provides file system related functions and validation.
#[derive(Debug)]
pub struct ProjectHandle(Project);

pub struct ProjectOptions<'a> {
    pub name: &'a str,
    pub deserialization_context: DeserializationContext<'a>,
}

impl ProjectHandle {
    /// Load a project from its root directory
    pub fn from_root<R: AsRef<Path>>(root: R, options: ProjectOptions) -> Result<Self> {
        let normalized_root = normalize_path(root)?;
        let (file_type, configuration_file_path) =
            infer_configuration_file_path(&normalized_root, PROJECT_FILENAME)?.ok_or_else(
                || {
                    anyhow!(
                        "could not find a valid project configuration file at {}",
                        normalized_root.display()
                    )
                },
            )?;

        let configuration_file_path = normalize_path(configuration_file_path)?;

        let mut root = configuration_file_path.clone();
        // Remove filename from configuration file path to get root
        let _ = root.pop();

        let deserialized_project_value = deserialize_configuration::<Value>(
            &configuration_file_path,
            file_type,
            DeserializationContext {
                jpath: options.deserialization_context.jpath,
                template_data: &options
                    .deserialization_context
                    .template_data
                    .with_project(options.name, &root)?,
            },
        )?;

        validate_json(&PROJECT_JSON_SCHEMA, &deserialized_project_value).with_context(|| {
            format!(
                "invalid project configuration at {}",
                configuration_file_path.display()
            )
        })?;

        let project_configuration = ProjectConfiguration::deserialize(deserialized_project_value)
            .with_context(|| {
            format!(
                "could not deserialize project configuration file at {}",
                configuration_file_path.display()
            )
        })?;

        Ok(Self(Project::from_configuration_and_metadata(
            options.name,
            (configuration_file_path, file_type),
            project_configuration,
        )))
    }

    pub fn unwrap_inner(self) -> Project {
        self.0
    }
}
