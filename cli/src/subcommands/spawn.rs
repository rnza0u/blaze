use std::path::{Path, PathBuf};

use anyhow::bail;
use blaze_common::{
    error::Result,
    parallelism::Parallelism,
    shell::{Shell, ShellKind},
};
use blaze_core::{spawn, workspace_spawn, GlobalOptions, SpawnOptions, WorkspaceSpawnOptions};
use clap::Parser;

use crate::{subcommand::BlazeSubCommandExecution, subcommands::help::parallelism_input_hint};

use super::selection_args::{project_selection_opts_without, SelectionArgs};

const PARALLELISM_HELP: &str =
    "The level of parallelism to use when spawning a command across multiple projects.";

pub fn parallelism_long_help() -> String {
    format!("{} {}", PARALLELISM_HELP, parallelism_input_hint())
}

#[derive(Parser, Debug)]
#[command(
    display_name = "spawn",
    name = "spawn",
    about("Run system commands on a selection of projects."),
    long_about(
        "Run system commands on a selection of projects (or at the workspace root) and report the results.

For each selected project, the command will be launched at its corresponding root directory. \
If no project selection option such as `--projects` or `--all` is provided, the workspace-level default project selector will be used. \
If you use the --workspace option, then the command will be launched at the workspace root directory.

The command will return a non-zero status code if any of the commands fails. \
Command failure means that a problem occured while spawning the process (configuration problem, IO error, system errors etc...) or that a non-zero exit code was returned by the command after it was terminated, also it could mean that the command was interrupted by a termination signal."
    )
)]
pub struct SpawnCommand {
    #[command(flatten)]
    selection: SelectionArgs,

    #[arg(
        help = PARALLELISM_HELP,
        long_help = parallelism_long_help(),
        long = "parallelism"
    )]
    parallelism: Option<Parallelism>,

    #[arg(
        help = "The system command to run.",
        long_help = "The system command to run. \
This command will be executed at the root folder of each project, or at the workspace root if you use the --workspace option.",
        required(true)
    )]
    command: Vec<String>,

    #[arg(
        default_value_t,
        help = "Run the command at the workspace root.",
        long_help = "Run the command only once, at the workspace root directory.",
        short,
        long,
        conflicts_with_all=project_selection_opts_without([])
    )]
    workspace: bool,

    #[arg(
        short = 'k',
        long = "shell-kind",
        help = "Specify the shell kind that is being used.",
        long_help = "The shell kind to use. \
You can use this option if Blaze cannot infer what kind of shell is provided through the -s (or --shell) option. \
If no -s (or --shell) option is provided, then this option will be ignored."
    )]
    shell_kind: Option<ShellKind>,

    #[arg(
        long = "shell",
        help = "The shell program to use.",
        long_help = "The shell program to use.
A shell will always be used when spawning the command on each selected project. \
If you want to use a different shell than the default one, you can pass this --shell option."
    )]
    shell: Option<PathBuf>,

    #[arg(
        short,
        long,
        default_value_t,
        help = "Disable command output.",
        long_help = "Disable command output."
    )]
    quiet: bool,
}

impl BlazeSubCommandExecution for SpawnCommand {
    fn execute(&self, root: &Path, global_options: GlobalOptions) -> Result<()> {
        if self.workspace {
            let mut options = WorkspaceSpawnOptions::new(&self.command.join(" "));

            if let Some(shell) = &self.shell {
                options = options.shell(Shell::new(shell, self.shell_kind));
            }

            if self.quiet {
                options = options.quiet();
            }

            workspace_spawn(root, options, global_options)?;

            return Ok(());
        }

        let mut options = SpawnOptions::new(&self.command.join(" "));

        if let Some(source) = self.selection.get_selector_source() {
            options = options.with_selector_source(source);
        }

        if let Some(parallelism) = &self.parallelism {
            options = options.with_parallelism(*parallelism);
        }

        if let Some(shell_path) = &self.shell {
            options = options.with_shell(Shell::new(shell_path, self.shell_kind));
        }

        if self.quiet {
            options = options.quietly();
        }

        let results = spawn(root, options, global_options)?;

        let failures = results.failures();

        if !failures.is_empty() {
            bail!(
                "command execution failed for the following projects: {}",
                failures.into_iter().collect::<Vec<_>>().join(", ")
            );
        }

        Ok(())
    }
}
