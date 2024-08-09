use anyhow::Context;
use blaze_common::{error::Result, value::Value};
use serde::Deserialize;

use crate::{
    executors::{env::get_executor_env, Executor, ExecutorContext},
    system::process::ProcessStatus,
};

use self::{
    command::Command,
    runner::{CommandsRunner, CommandsRunnerOptions},
};

use super::options::UseShell;

mod command;
mod runner;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CommandsExecutorOptions {
    commands: Vec<Command>,
    #[serde(rename = "shell")]
    use_shell: Option<UseShell>,
}

pub struct CommandsExecutor;

impl Executor for CommandsExecutor {
    fn execute(&self, ctx: ExecutorContext, raw_options: Value) -> Result<()> {
        let options = CommandsExecutorOptions::deserialize(&raw_options).with_context(|| {
            format!("error while converting commands executor options from {raw_options}")
        })?;

        let mut runner = CommandsRunner::new(CommandsRunnerOptions {
            use_shell: options.use_shell,
            default_cwd: ctx.project.root().to_owned(),
            default_environment: get_executor_env(&ctx)?,
        });

        runner.on_command_started(|command| {
            ctx.logger.info(format!("+ {command}"));
        });

        runner.on_command_terminated(|command, status| match *status {
            ProcessStatus { success: true, .. } => {
                ctx.logger
                    .debug(format!("command \"{command}\" was successful"));
            }
            ProcessStatus {
                code: Some(code),
                success: false,
            } => {
                ctx.logger
                    .error(format!("\"{command}\" has failed with status code {code}"));
            }
            ProcessStatus {
                code: None,
                success: false,
            } => {
                ctx.logger
                    .error(format!("\"{command}\" exited without any status code"));
            }
        });

        runner
            .run_all(&options.commands)
            .context("error while running commands")?;

        Ok(())
    }
}
