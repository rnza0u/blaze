use blaze_common::selector::ProjectSelector;
use blaze_core::SelectorSource;
use clap::Parser;

use crate::subcommands::help::{exclude_projects_input_hint, include_projects_input_hint};

pub fn project_selection_opts_without<const N: usize>(names: [&str; N]) -> Vec<&'static str> {
    vec!["projects", "all", "selector", "include", "exclude"]
        .into_iter()
        .filter(|n| !names.contains(n))
        .collect()
}

const INCLUDE_HELP: &str = "A pattern to use for including projects.";

fn include_long_help() -> String {
    format!("{} {}", INCLUDE_HELP, include_projects_input_hint())
}

const EXCLUDE_HELP: &str =
    "A pattern to use for excluding projects that were selected using the --include option.";

fn exclude_long_help() -> String {
    format!("{} {}", EXCLUDE_HELP, exclude_projects_input_hint())
}

const TAGS_HELP: &str = "Tags to use when selecting projects.";

fn tags_long_help() -> String {
    format!("{} All projects that have at least one tag corresponding to one of the provided tags will be selected. You can pass multiple tags delimited by commas (for example, tag1,tag2,tag3).", TAGS_HELP)
}

#[derive(Parser, Debug)]
pub struct SelectionArgs {
    #[arg(
        help = "A comma-separated list of project names to select.",
        long_help = "A list of project names. \
Will be parsed as a comma-separated list of project names (for e.g project1,project2,project3).",
        short = 'p',
        long = "projects",
        value_delimiter = ',',
        conflicts_with_all(project_selection_opts_without(["projects"]))
    )]
    pub projects: Option<Vec<String>>,

    #[arg(
        help = "A named project selector.",
        long_help = "A named selector to use when selecting projects. It must be declared at the workspace level.",
        short = 's',
        long = "selector",
        conflicts_with_all(project_selection_opts_without(["selector"]))
    )]
    pub selector: Option<String>,

    #[arg(
        help = "Select all projects in the workspace.",
        short = 'a',
        long = "all",
        conflicts_with_all(project_selection_opts_without(["all"]))
    )]
    pub all: bool,

    #[arg(
        help = INCLUDE_HELP,
        long_help = include_long_help(),
        long = "include",
        conflicts_with_all(project_selection_opts_without(["include", "exclude"]))
    )]
    pub include: Option<Vec<String>>,

    #[arg(
        help = EXCLUDE_HELP,
        long_help = exclude_long_help(),
        long = "exclude",
        conflicts_with_all(project_selection_opts_without(["include", "exclude"]))
    )]
    pub exclude: Option<Vec<String>>,

    #[arg(
        help = TAGS_HELP,
        long_help = tags_long_help(),
        long = "tags",
        value_delimiter = ',',
        conflicts_with_all(project_selection_opts_without(["tags"]))
    )]
    pub tags: Option<Vec<String>>,
}

impl SelectionArgs {
    pub fn get_selector_source(&self) -> Option<SelectorSource> {
        self.projects
            .as_ref()
            .map(ProjectSelector::array)
            .or_else(|| self.all.then_some(ProjectSelector::all()))
            .or_else(|| self.tags.as_ref().map(ProjectSelector::tagged))
            .or_else(|| {
                self.include.as_ref().map(|patterns| {
                    ProjectSelector::include_exclude(
                        patterns,
                        self.exclude.as_ref().unwrap_or(&vec![]),
                    )
                })
            })
            .map(SelectorSource::Provided)
            .or_else(|| {
                self.selector
                    .as_ref()
                    .map(|s| s.to_owned())
                    .map(SelectorSource::Named)
            })
    }
}
