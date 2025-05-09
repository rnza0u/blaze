use std::path::Path;

use anyhow::bail;
use blaze_common::{error::Result, parallelism::Parallelism, selector::ProjectSelector};
use blaze_core::{run, GlobalOptions, RunOptions, SelectorSource};
use clap::Parser;
use possibly::possibly;

use crate::{subcommand::BlazeSubCommandExecution, subcommands::help::parallelism_input_hint};

use super::{
    double::Double,
    selection_args::{project_selection_opts_without, SelectionArgs},
};

const PARALLELISM_HELP: &str =
    "The level of parallelism to use when running the target across multiple projects.";

fn parallelism_long_help() -> String {
    format!("{} {}", PARALLELISM_HELP, parallelism_input_hint())
}

#[derive(Parser, Debug)]
#[command(
    display_name = "run",
    name = "run",
    about("Run a target on a selection of projects."),
    long_about(
        "Run a target on a selection of projects and report the results. \
For each selected project, the target will be executed (if available) using its corresponding executor. \
By default, each execution might be skipped and its result get directly retrieved from cache. \
The run command will fail and return a non-zero status code if any of the targets that correspond to a selected project fail (or cannot be executed due to unfullfilled dependencies). \
It will also fail if an internal error occurs while executing any target (due to a configuration error, an IO error, or any other external cause)."
    )
)]
pub struct RunCommand {
    #[arg(
        help = "The target name.",
        long_help = "The target name. Must be a valid target name. For example, `build`, or `test`. \
Selected projects that don't have any target matching this value will be ignored.",
        short = 't',
        long = "target",
        required_unless_present = "double"
    )]
    target: Option<String>,

    #[arg(
        help = PARALLELISM_HELP,
        long_help = parallelism_long_help(),
        long = "parallelism"
    )]
    parallelism: Option<Parallelism>,

    #[command(flatten)]
    selection: SelectionArgs,

    #[arg(
        help = "Skip execution of all targets.",
        long_help = "Skip execution of all targets. This can be used for debugging purpose, for example if you simply want to checkout the execution graph.",
        long = "dry-run"
    )]
    dry_run: bool,

    #[arg(
        help = "Disables the display of a tree-style execution graph after all targets have been executed.",
        long = "no-graph"
    )]
    no_graph: bool,

    #[arg(
        help = "An execution double consisting of an optional project name and a target name (in that specific order).",
        long_help = "An execution double consisting of an optional project name and a target name (in that specific order). \
Parts of the execution double must be separated with a colon. For e.g : build, or app:build. \
Using an execution double only allows to execute the target on a single project. \
Only the target name is mandatory, if the project name is not provided, then the default project selector will be used.",
        index = 1,
        required_unless_present = "target",
        conflicts_with_all = vec![project_selection_opts_without([]), vec!["target"]].concat()
    )]
    double: Option<Double>,

    #[arg(
        help = "Set a maximum depth of dependencies when executing targets.",
        long_help = "Set a maximum depth of dependencies when executing targets. \
By default, every target dependencies are resolved resursively, no matter how deep. \
When providing zero, then no dependencies will be resolved for each target.",
        long = "depth"
    )]
    dependencies_depth: Option<usize>,
}

impl BlazeSubCommandExecution for RunCommand {
    fn execute(self: Box<Self>, root: &Path, globals: GlobalOptions) -> Result<()> {
        let target = if let Some(double) = &self.double {
            &double.target
        } else if let Some(target) = &self.target {
            target
        } else {
            unreachable!()
        };

        let mut options = RunOptions::new(target);

        if let Some(project) = self.double.as_ref().and_then(|t| t.project.as_ref()) {
            options = options
                .with_selector_source(SelectorSource::Provided(ProjectSelector::array([project])))
        } else if let Some(selector) = self.selection.get_selector_source() {
            options = options.with_selector_source(selector);
        }

        if let Some(parallelism) = &self.parallelism {
            options = options.with_parallelism(*parallelism);
        }

        if self.dry_run {
            options = options.as_dry_run();
        }

        if !self.no_graph {
            options = options.displaying_graph();
        }

        if let Some(max_depth) = self.dependencies_depth {
            options = options.with_dependencies_depth(max_depth);
        }

        let run_result = run(root, options, globals)?;

        let root_failures = run_result
            .root_executions()
            .values()
            .filter_map(|execution_result| possibly!(&execution_result.result, Some(Err(_))|None => execution_result.execution.get_double()))
            .collect::<Vec<_>>();

        if !self.dry_run && !root_failures.is_empty() {
            bail!(
                "run failed for target(s): \n\n{}",
                root_failures.into_iter().collect::<Vec<_>>().join("\n")
            )
        }
        Ok(())
    }
}
