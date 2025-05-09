use std::path::Path;

use blaze_common::error::Result;
use blaze_core::{
    describe_project, describe_workspace, DescribeProjectOptions, DescribeWorkspaceOptions,
    GlobalOptions,
};
use clap::{Parser, Subcommand};

use crate::subcommand::BlazeSubCommandExecution;

use super::selection_args::SelectionArgs;

#[derive(Debug, Subcommand)]
enum DescribeSubCommand {
    #[command(
        display_name = "project",
        name = "project",
        about("Display targets information within a project."),
        long_about = "Display targets information within a project.\
By default, the results will be printed in a table with information such as : \
- The target's name
- The target's description
- A summary of the target's dependencies"
    )]
    Project {
        #[arg(index = 1, help = "The name of the project to be described.")]
        project: String,

        #[arg(
            index = 2,
            help = "Filter with specific target name(s) to be included in the description."
        )]
        targets: Option<Vec<String>>,

        #[arg(
            short,
            long,
            default_value_t,
            help = "Display each target name, line per line, instead of a full table with details."
        )]
        summary: bool,
    },
    #[command(
        display_name = "workspace",
        name = "workspace",
        about("Display projects information for the workspace"),
        long_about = "Display targets information within a project.\
By default, the results will be printed in a table with information such as :\
\n\
- The project's name
- The project's description
- The project's relative path from the workspace root
- The project's declared tags
\n\
By default, all projects are described."
    )]
    Workspace {
        #[arg(
            long,
            default_value_t,
            help = "Display each project name, line per line, instead of a full table with details."
        )]
        summary: bool,

        #[command(flatten)]
        selection: SelectionArgs,
    },
}

#[derive(Debug, Parser)]
#[command(
    display_name = "describe",
    name = "describe",
    about("Display information about the workspace."),
    long_about(
        "Display information about the workspace. \
The next subcommand specifies which kind of item needs to be described. Also, specific options are supported for each subcommand."
    )
)]
pub struct DescribeCommand {
    #[command(subcommand)]
    subcommand: DescribeSubCommand,
}

impl BlazeSubCommandExecution for DescribeCommand {
    fn execute(self: Box<Self>, root: &Path, global_options: GlobalOptions) -> Result<()> {
        match self.subcommand {
            DescribeSubCommand::Project {
                project,
                targets,
                summary,
            } => {
                let mut options = DescribeProjectOptions::new(project);

                if summary {
                    options = options.as_summary();
                }

                if let Some(targets) = targets {
                    options = options.with_targets(targets);
                }

                describe_project(root, options, global_options)?;
            }
            DescribeSubCommand::Workspace { summary, selection } => {
                let mut options = DescribeWorkspaceOptions::new();

                if summary {
                    options = options.as_summary();
                }

                if let Some(source) = selection.get_selector_source() {
                    options = options.with_selector_source(source);
                }

                describe_workspace(root, options, global_options)?;
            }
        }

        Ok(())
    }
}
