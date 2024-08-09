use std::path::Path;

use anyhow::{anyhow, Context};
use blaze_common::{error::Result, selector::ProjectSelector, util::path_to_string};
use tabled::{
    settings::{Style, Width},
    Table, Tabled,
};
use terminal_size::terminal_size;

use crate::{
    workspace::{
        project_handle::{ProjectHandle, ProjectOptions},
        selection::{Selection, SelectionContext, SelectorSource},
    },
    GlobalOptions, WorkspaceGlobals,
};

fn display_table<T>(rows: &[T])
where
    T: Tabled,
{
    let mut table = Table::new(rows);
    table.with(Style::modern_rounded());

    let size = terminal_size();
    if let Some((terminal_size::Width(w), _)) = size {
        table.with(Width::wrap(w as usize));
    }

    println!("{table}");
}

pub struct DescribeProjectOptions {
    summary: bool,
    project: String,
    targets: Option<Vec<String>>,
}

impl DescribeProjectOptions {
    pub fn new<P: AsRef<str>>(project: P) -> Self {
        Self {
            summary: false,
            project: project.as_ref().to_owned(),
            targets: None,
        }
    }

    pub fn as_summary(mut self) -> Self {
        self.summary = true;
        self
    }

    pub fn with_targets<T: AsRef<str>, I: IntoIterator<Item = T>>(mut self, targets: I) -> Self {
        self.targets = Some(targets.into_iter().map(|t| t.as_ref().to_owned()).collect());
        self
    }
}

pub fn describe_project(
    root: &Path,
    options: DescribeProjectOptions,
    global_options: GlobalOptions,
) -> Result<()> {
    let globals =
        WorkspaceGlobals::new(root, global_options).context("error while loading globals.")?;

    let workspace = globals.workspace_handle().inner();

    let project_ref = workspace
        .projects()
        .get(&options.project)
        .ok_or_else(|| anyhow!("project {} was not found.", options.project))?;

    let project = ProjectHandle::from_root(
        workspace.root().join(project_ref.path()),
        ProjectOptions {
            name: &options.project,
            deserialization_context: globals.deserialization_context(),
        },
    )
    .with_context(|| format!("error while loading project {}.", options.project))?
    .unwrap_inner();

    let target_filter = |name: &String| {
        if let Some(targets) = &options.targets {
            targets.contains(name)
        } else {
            true
        }
    };

    if options.summary {
        println!(
            "{}",
            project
                .targets()
                .keys()
                .filter(|name| target_filter(name))
                .map(String::as_str)
                .collect::<Vec<_>>()
                .join("\n")
        );
        return Ok(());
    }

    #[derive(Tabled)]
    struct TargetsTableRow<'a> {
        name: &'a str,
        description: &'a str,
        dependencies: String,
    }

    let rows = project
        .targets()
        .iter()
        .filter(|(name, _)| target_filter(name))
        .map(|(name, target)| {
            Ok(TargetsTableRow {
                name,
                description: target.description().unwrap_or(""),
                dependencies: target
                    .dependencies()
                    .iter()
                    .map(|dependency| {
                        Ok(match dependency.projects() {
                            None => dependency.target().to_owned(),
                            Some(selector) => {
                                Selection::from_source(SelectorSource::Provided(selector.clone()))
                                    .select(SelectionContext { workspace })?
                                    .into_iter()
                                    .map(|(name, project_ref)| {
                                        ProjectHandle::from_root(
                                            workspace.root().join(project_ref.path()),
                                            ProjectOptions {
                                                name,
                                                deserialization_context: globals
                                                    .deserialization_context(),
                                            },
                                        )
                                    })
                                    .collect::<Result<Vec<_>>>()?
                                    .into_iter()
                                    .filter_map(|project_handle| {
                                        let project = project_handle.unwrap_inner();
                                        project.targets().get(dependency.target()).map(|_| {
                                            format!("{}:{}", project.name(), dependency.target())
                                        })
                                    })
                                    .collect::<Vec<_>>()
                                    .join("\n")
                            }
                        })
                    })
                    .collect::<Result<Vec<_>>>()?
                    .join("\n"),
            })
        })
        .collect::<Result<Vec<_>>>()?;

    if rows.is_empty() {
        println!("no targets to display");
        return Ok(());
    }

    display_table(&rows[..]);

    Ok(())
}

pub struct DescribeWorkspaceOptions {
    summary: bool,
    selector_source: SelectorSource,
}

impl Default for DescribeWorkspaceOptions {
    fn default() -> Self {
        Self {
            summary: false,
            selector_source: SelectorSource::Provided(ProjectSelector::All),
        }
    }
}

impl DescribeWorkspaceOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn as_summary(mut self) -> Self {
        self.summary = true;
        self
    }

    pub fn with_selector_source(mut self, source: SelectorSource) -> Self {
        self.selector_source = source;
        self
    }
}

pub fn describe_workspace(
    root: &Path,
    options: DescribeWorkspaceOptions,
    global_options: GlobalOptions,
) -> Result<()> {
    let globals = WorkspaceGlobals::new(root, global_options)?;

    let workspace = globals.workspace_handle().inner();

    let projects =
        Selection::from_source(options.selector_source).select(SelectionContext { workspace })?;

    if options.summary {
        println!(
            "{}",
            projects
                .into_keys()
                .map(|name| name.as_str())
                .collect::<Vec<_>>()
                .join("\n")
        );
        return Ok(());
    }

    #[derive(Tabled)]
    struct ProjectTableRow<'a> {
        name: &'a str,
        path: String,
        description: &'a str,
        tags: String,
    }

    let rows = projects
        .into_iter()
        .map(|(name, project_ref)| {
            Ok(ProjectTableRow {
                name: name.as_str(),
                path: path_to_string(project_ref.path())?,
                description: project_ref.description().unwrap_or(""),
                tags: project_ref
                    .tags()
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join("\n"),
            })
        })
        .collect::<Result<Vec<_>>>()?;

    display_table(&rows[..]);

    Ok(())
}
