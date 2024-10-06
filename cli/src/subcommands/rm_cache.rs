use std::path::Path;

use blaze_common::{error::Result, selector::ProjectSelector};
use blaze_core::{rm_execution_caches, GlobalOptions, RmExecutionCacheOptions, SelectorSource};
use clap::Parser;

use crate::subcommand::BlazeSubCommandExecution;

use super::{
    double::Double,
    selection_args::{project_selection_opts_without, SelectionArgs},
};

#[derive(Debug, Parser)]
#[command(
    display_name = "rm-cache",
    name = "rm-cache",
    about("Remove target execution cache for a selection of projects."),
    long_about(
        "Remove target execution cache for a selection of projects. \
Projects are selected just like the `run` command and recursive cache removal across dependencies is supported. \
By default, dependencies cache is not removed."
    )
)]
pub struct RmCacheCommand {
    #[arg(
        help = "The target name which cache should be invalidated.",
        long_help = "The target name which cache should be invalidated. If the target does not exist for some selected projects, they will be ignored.",
        short,
        long
    )]
    target: Option<String>,

    #[command(flatten)]
    selection: SelectionArgs,

    #[arg(
        help = "Set a maximum depth of target dependencies which cache should be removed.",
        long_help = "Set a maximum depth of target dependencies which cache should be removed. If not set, dependencies cache will not be removed.",
        short,
        long = "depth"
    )]
    dependencies_depth: Option<usize>,

    #[arg(
        help = "An execution double consisting of an optional project name and a target name.",
        long_help = "An execution double consisting of an optional project name and a target name. Works the same as for the <code>run</code> command.",
        index = 1,
        required_unless_present = "target",
        conflicts_with_all = vec![project_selection_opts_without([]), vec!["target"]].concat()
    )]
    double: Option<Double>,
}

impl BlazeSubCommandExecution for RmCacheCommand {
    fn execute(self: Box<Self>, root: &Path, global_options: GlobalOptions) -> Result<()> {
        let mut options = RmExecutionCacheOptions::new(
            self.target
                .as_deref()
                .or_else(|| self.double.as_ref().map(|double| double.target.as_str()))
                .unwrap(),
        );

        if let Some(depth) = self.dependencies_depth {
            options = options.with_depth(depth);
        }

        if let Some(project) = self.double.and_then(|double| double.project) {
            options = options
                .with_selector_source(SelectorSource::Provided(ProjectSelector::array([project])))
        } else if let Some(source) = self.selection.get_selector_source() {
            options = options.with_selector_source(source);
        }

        rm_execution_caches(root, options, global_options)?;

        Ok(())
    }
}
