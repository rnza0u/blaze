use std::path::Path;

use blaze_common::error::Result;
use blaze_core::{
    render_project, render_workspace, GlobalOptions, RenderFormat, RenderOutput,
    RenderProjectOptions, RenderWorkspaceOptions,
};
use clap::{Parser, Subcommand};

use crate::subcommand::BlazeSubCommandExecution;

#[derive(Debug, Subcommand)]
pub enum RenderSubcommand {
    #[command(
        display_name = "project",
        name = "project",
        about = "Render a project configuration file.",
        long_about = "Render a project configuration file."
    )]
    Project {
        #[arg(
            help = "Name of the project to render.",
            long_help = "Name of the project to render.",
            index = 1
        )]
        name: String,
    },
    #[command(
        display_name = "workspace",
        name = "workspace",
        about = "Render the workspace configuration file.",
        long_about = "Render the workspace configuration file."
    )]
    Workspace,
}

#[derive(Debug, Parser)]
#[command(
    display_name = "render",
    name = "render",
    about = "Render a configuration file within the workspace.",
    long_about = "Render a configuration file within the workspace. \
The next subcommand specifies which kind of file you want to render (for e.g a project file)."
)]
pub struct RenderCommand {
    #[arg(
        short,
        long,
        help = "Rendering output format.",
        long_help = "Rendering output format. \
Can be either <code>Json</code> or <code>Yaml</code>. \
By default, the output format will be based on the rendered configuration file format."
    )]
    format: Option<RenderFormat>,

    #[command(subcommand)]
    subcommand: RenderSubcommand,
}

impl BlazeSubCommandExecution for RenderCommand {
    fn execute(self: Box<Self>, root: &Path, global_options: GlobalOptions) -> Result<()> {
        let output = RenderOutput {
            format: self.format,
            stream: std::io::stdout(),
        };

        match self.subcommand {
            RenderSubcommand::Project { name } => {
                render_project(
                    root,
                    RenderProjectOptions {
                        name,
                        output,
                    },
                    global_options,
                )?;
            }
            RenderSubcommand::Workspace => {
                render_workspace(root, RenderWorkspaceOptions { output }, global_options)?;
            }
        }
        Ok(())
    }
}
