use std::{io::Write, path::Path};

use anyhow::{anyhow, Context};
use blaze_common::{
    configuration_file::ConfigurationFileFormat, error::Result, unit_enum_from_str,
};
use serde::Serialize;
use strum_macros::{Display, EnumIter};

use crate::{
    workspace::project_handle::{ProjectHandle, ProjectOptions},
    GlobalOptions, WorkspaceGlobals,
};

#[derive(EnumIter, Display, Clone, Copy, Debug)]
pub enum RenderFormat {
    Json,
    Yaml,
}

unit_enum_from_str!(RenderFormat);

pub struct RenderOutput<O>
where
    O: Write,
{
    pub stream: O,
    pub format: Option<RenderFormat>,
}

pub struct RenderWorkspaceOptions<O>
where
    O: Write,
{
    pub output: RenderOutput<O>,
}

pub fn render_workspace<O>(
    root: &Path,
    options: RenderWorkspaceOptions<O>,
    global_options: GlobalOptions,
) -> Result<()>
where
    O: Write,
{
    let globals = WorkspaceGlobals::new(root, global_options)?;
    let workspace = globals.workspace_handle().inner();
    let file_format = workspace.configuration_file_format();
    render(
        workspace,
        options
            .output
            .format
            .unwrap_or_else(|| configuration_file_format_to_render_format(file_format)),
        options.output.stream,
    )?;
    Ok(())
}

pub struct RenderProjectOptions<O>
where
    O: Write,
{
    pub name: String,
    pub output: RenderOutput<O>,
}

pub fn render_project<O>(
    root: &Path,
    options: RenderProjectOptions<O>,
    global_options: GlobalOptions,
) -> Result<()>
where
    O: Write,
{
    let globals = WorkspaceGlobals::new(root, global_options)?;
    let workspace = globals.workspace_handle().inner();
    let project = globals
        .workspace_handle()
        .inner()
        .projects()
        .get(&options.name)
        .map(|reference| {
            ProjectHandle::from_root(
                workspace.root().join(reference.path()),
                ProjectOptions {
                    name: &options.name,
                    deserialization_context: globals.deserialization_context(),
                },
            )
        })
        .transpose()
        .with_context(|| format!("could not open project \"{}\"", options.name))?
        .ok_or_else(|| anyhow!("project does not exist"))?;
    let project = project.unwrap_inner();
    let file_format = project.configuration_file_format();
    render(
        project,
        options
            .output
            .format
            .unwrap_or_else(|| configuration_file_format_to_render_format(file_format)),
        options.output.stream,
    )?;
    Ok(())
}

fn configuration_file_format_to_render_format(format: ConfigurationFileFormat) -> RenderFormat {
    match format {
        ConfigurationFileFormat::Json => RenderFormat::Json,
        ConfigurationFileFormat::Jsonnet => RenderFormat::Json,
        ConfigurationFileFormat::Yaml => RenderFormat::Yaml,
    }
}

fn render<T, O>(value: T, format: RenderFormat, output: O) -> Result<()>
where
    T: Serialize,
    O: Write,
{
    match format {
        RenderFormat::Json => serde_json::to_writer_pretty(output, &value)?,
        RenderFormat::Yaml => serde_yaml::to_writer(output, &value)?,
    };
    Ok(())
}
