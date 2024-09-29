use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    path::Path,
    thread::scope,
};

use anyhow::Context;
use blaze_common::{error::Result, parallelism::Parallelism, shell::Shell};

use crate::{
    system::{
        parallel_executor::ParallelRunner,
        process::{Process, ProcessOptions, ProcessStatus},
        shell::ShellFormatter,
    },
    workspace::selection::{Selection, SelectionContext, SelectorSource},
    GlobalOptions, WorkspaceGlobals,
};

pub struct SpawnResults {
    executions: HashMap<String, Result<ProcessStatus>>,
}

impl SpawnResults {
    pub fn failures(&self) -> HashSet<&str> {
        self.executions
            .iter()
            .filter_map(|(name, result)| match result.as_ref() {
                Err(_) | Ok(ProcessStatus { success: false, .. }) => Some(name.as_str()),
                _ => None,
            })
            .collect()
    }

    pub fn successes(&self) -> HashSet<&str> {
        self.executions
            .iter()
            .filter_map(|(name, result)| {
                result
                    .as_ref()
                    .ok()
                    .filter(|status| status.success)
                    .map(|_| name.as_str())
            })
            .collect()
    }
}

#[derive(Debug)]
pub struct SpawnOptions<'a> {
    command: &'a str,
    shell: Option<Shell>,
    selector_source: Option<SelectorSource>,
    parallelism: Option<Parallelism>,
    quiet: bool,
}

impl <'a> SpawnOptions<'a> {
    pub fn new(command: &'a str) -> Self {
        Self {
            command,
            shell: None,
            selector_source: None,
            parallelism: None,
            quiet: false,
        }
    }

    pub fn with_shell(mut self, shell: Shell) -> Self {
        self.shell = Some(shell);
        self
    }

    pub fn with_parallelism(mut self, parallelism: Parallelism) -> Self {
        self.parallelism = Some(parallelism);
        self
    }

    pub fn with_selector_source(mut self, source: SelectorSource) -> Self {
        self.selector_source = Some(source);
        self
    }

    pub fn quietly(mut self) -> Self {
        self.quiet = true;
        self
    }
}

/// Run arbitrary commands across projects.
pub fn spawn(
    root: &Path,
    options: SpawnOptions,
    global_options: GlobalOptions,
) -> Result<SpawnResults> {
    let globals = WorkspaceGlobals::new(root, global_options)?;
    let logger = globals.logger();

    let workspace = globals.workspace_handle().inner();

    let mut projects_refs = options
        .selector_source
        .map(Selection::from_source)
        .unwrap_or_default()
        .select(SelectionContext { workspace })
        .context("error while selecting project references")?
        .into_iter()
        .collect::<Vec<_>>();

    let shell_formatter = options
        .shell
        .as_ref()
        .map(ShellFormatter::from_shell)
        .unwrap_or_default();

    let parallelism = options
        .parallelism
        .or(workspace.settings().parallelism())
        .unwrap_or_default();

    logger.debug(format!(
        "{shell_formatter} will be used as a shell when launching commands"
    ));

    logger.debug(format!(
        "{parallelism} will be used for scheduling executions"
    ));

    logger.debug(format!(
        "command \"{}\" will be launched for the following projects: {}",
        options.command,
        projects_refs
            .iter()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    ));

    let mut executions = HashMap::with_capacity(projects_refs.len());

    let mut commands = projects_refs
        .iter()
        .map(|(name, reference)| {
            let template_data = globals
                .deserialization_context()
                .template_data
                .with_project(name.as_str(), &workspace.root().join(reference.path()))?;
            let interpolated_command = template_data.render_str(options.command)?;
            Ok((
                name.as_str(),
                shell_formatter.format_command(&interpolated_command)?,
            ))
        })
        .collect::<Result<HashMap<_, _>>>()?;

    scope(move |scope| {
        let mut parallel_executor = ParallelRunner::new(scope, parallelism)?;

        loop {
            parallel_executor.push_available(|| {
                let (name, project_ref) = projects_refs.pop()?;
                let process_options = ProcessOptions {
                    display_output: !options.quiet,
                    cwd: Some(workspace.root().join(project_ref.path())),
                    ..Default::default()
                };
                let logger_1 = logger.clone();

                let (program, arguments) = commands.remove(name.as_str()).unwrap();

                Some(move || {
                    logger_1.debug(format!("spawning command for project {name}"));
                    let process_result =
                        Process::run_with_options(program, arguments, process_options)
                            .and_then(|process| process.wait());

                    match process_result.as_ref() {
                        Err(err) => logger_1.error(format!("spawn failure for {name}: {err:?}")),
                        Ok(ProcessStatus {
                            code,
                            success: false,
                        }) => {
                            logger_1.error(format!(
                                "command failed for {name} ({})",
                                code.map(|code| Cow::Owned(format!(
                                    "with non-zero exit code {code}"
                                )))
                                .unwrap_or(Cow::Borrowed("terminated"))
                            ));
                        }
                        _ => {}
                    }

                    (name.to_owned(), process_result)
                })
            });

            if !parallel_executor.is_running() && projects_refs.is_empty() {
                break;
            }

            executions.extend(parallel_executor.drain()?);
        }

        Ok(SpawnResults { executions })
    })
}

pub struct WorkspaceSpawnOptions {
    command: String,
    shell: Option<Shell>,
    quiet: bool,
}

impl WorkspaceSpawnOptions {
    pub fn new(command: &str) -> Self {
        Self {
            command: command.to_owned(),
            shell: None,
            quiet: false,
        }
    }

    pub fn quiet(mut self) -> Self {
        self.quiet = true;
        self
    }

    pub fn shell(mut self, shell: Shell) -> Self {
        self.shell = Some(shell);
        self
    }
}

pub fn workspace_spawn(
    root: &Path,
    options: WorkspaceSpawnOptions,
    global_options: GlobalOptions,
) -> Result<()> {
    let globals = WorkspaceGlobals::new(root, global_options)?;
    let logger = globals.logger();

    let shell_formatter = options
        .shell
        .as_ref()
        .map(ShellFormatter::from_shell)
        .unwrap_or_default();

    let workspace = globals.workspace_handle().inner();

    logger.debug(format!(
        "{shell_formatter} will be used as a shell when launching command"
    ));

    logger.debug(format!(
        "command \"{}\" will be launched at the workspace root {}",
        options.command,
        workspace.root().display()
    ));

    let interpolated_command = globals
        .deserialization_context()
        .template_data
        .render_str(&options.command)?;

    let (program, arguments) = shell_formatter.format_command(&interpolated_command)?;

    let process_result = Process::run_with_options(
        program,
        &arguments[..],
        ProcessOptions {
            display_output: !options.quiet,
            cwd: Some(workspace.root().to_owned()),
            ..Default::default()
        },
    )
    .and_then(|process| process.wait());

    match process_result.as_ref() {
        Err(err) => logger.error(format!("spawn failure: {err:?}")),
        Ok(ProcessStatus {
            code,
            success: false,
        }) => {
            logger.error(format!(
                "command failed ({})",
                code.map(|code| Cow::Owned(format!("with non-zero exit code {code}")))
                    .unwrap_or(Cow::Borrowed("terminated"))
            ));
        }
        _ => {}
    }

    Ok(())
}
