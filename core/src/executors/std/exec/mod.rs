use std::{collections::HashMap, path::PathBuf};

use crate::{
    executors::{env::get_executor_env, Executor, ExecutorContext},
    system::{
        process::{Process, ProcessOptions},
        shell::ShellFormatter,
    },
};
use anyhow::{bail, Context};
use blaze_common::{error::Result, util::normalize_path, value::Value};
use serde::Deserialize;

use super::options::UseShell;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Options {
    program: PathBuf,
    #[serde(default)]
    arguments: Vec<String>,
    #[serde(default)]
    environment: HashMap<String, String>,
    cwd: Option<PathBuf>,
    #[serde(default)]
    quiet: bool,
    shell: Option<UseShell>,
}

pub struct ExecExecutor;

impl Executor for ExecExecutor {
    fn execute(&self, context: ExecutorContext, options: Value) -> Result<()> {
        let mut options =
            Options::deserialize(options).context("could not deserialize executor options")?;

        let normalized = normalize_path(&options.program)?;
        let program = dunce::canonicalize(if normalized.is_relative() {
            context.project.root().join(normalized)
        } else {
            normalized
        })
        .with_context(|| {
            format!(
                "could not get absolute path for {}",
                options.program.display()
            )
        })?;

        context
            .logger
            .debug(format!("using file located at {}.", program.display()));

        let shell_formatter = options.shell.as_ref().map(|use_shell| match use_shell {
            UseShell::Custom(shell) => ShellFormatter::from_shell(shell),
            UseShell::SystemDefault => ShellFormatter::default(),
        });

        if let Some(s) = &shell_formatter {
            context.logger.debug(format!("using shell: {s}"));
        }

        options.environment.extend(get_executor_env(&context)?);

        let cwd = options
            .cwd
            .unwrap_or_else(|| context.project.root().to_owned());

        context
            .logger
            .debug(format!("current working directory: {}", cwd.display()));

        if options.quiet {
            context.logger.debug("process output will be discarded");
        }

        context
            .logger
            .debug(format!("launching {}", program.display()));

        let (program, arguments) = if let Some(shell) = &shell_formatter {
            shell
                .format_script(&program, options.arguments)
                .context("error while formatting shell command")?
        } else {
            (program, options.arguments)
        };

        let result = Process::run_with_options(
            &program,
            arguments,
            ProcessOptions {
                cwd: Some(cwd),
                display_output: !options.quiet,
                environment: options.environment,
            },
        )
        .with_context(|| format!("could not create process for \"{}\"", program.display()))?
        .wait()
        .context("could not wait for process termination")?;

        if !result.success {
            let error = format!(
                "execution failed for \"{}\" ({})",
                program.display(),
                result
                    .code
                    .map(|c| format!("with status code {c}"))
                    .unwrap_or_else(|| "without any status code".into())
            );
            bail!(error)
        }

        context
            .logger
            .debug(format!("{} terminated successfully.", program.display()));

        Ok(())
    }
}
