use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    path::PathBuf,
    sync::mpsc::{channel, Receiver},
};

use anyhow::{bail, Context};
use blaze_common::error::Result;

use crate::{executors::std::options::UseShell, system::process::ProcessStatus};

use super::command::{Command, OnFailure, RunningCommand};

pub struct Termination {
    pub status: ProcessStatus,
    pub command: usize,
}

pub struct CommandsRunnerOptions {
    pub use_shell: Option<UseShell>,
    pub default_cwd: PathBuf,
    pub default_environment: HashMap<String, String>,
}

type CommandStartedHandler<'a> = Box<dyn Fn(&Command) + 'a>;
type CommandTerminatedHandler<'a> = Box<dyn Fn(&Command, &ProcessStatus) + 'a>;

pub struct CommandsRunner<'a> {
    options: CommandsRunnerOptions,
    command_started_handler: CommandStartedHandler<'a>,
    command_terminated_handler: CommandTerminatedHandler<'a>,
}

impl<'a> CommandsRunner<'a> {
    pub fn new(options: CommandsRunnerOptions) -> Self {
        Self {
            options,
            command_started_handler: Box::new(|_| {}),
            command_terminated_handler: Box::new(|_, _| {}),
        }
    }

    pub fn on_command_started<F: Fn(&Command) + 'a>(&mut self, started: F) {
        self.command_started_handler = Box::new(started);
    }

    pub fn on_command_terminated<F: Fn(&Command, &ProcessStatus) + 'a>(&mut self, terminated: F) {
        self.command_terminated_handler = Box::new(terminated);
    }

    pub fn run_all(&self, commands: &[Command]) -> Result<()> {
        let mut pending = VecDeque::from_iter(commands);
        let mut running: BTreeMap<usize, RunningCommand> = BTreeMap::new();
        let (termination_send, termination_recv) = channel::<Result<Termination>>();

        let on_terminate_for = |i: usize| {
            let termination_send_clone = termination_send.clone();
            move |status: Result<ProcessStatus>| -> Result<()> {
                termination_send_clone
                    .send(status.map(|status| Termination { status, command: i }))?;
                Ok(())
            }
        };

        loop {
            while running.values().all(|r| r.command.detach) && !pending.is_empty() {
                let mut next = pending.pop_front().unwrap().to_owned();
                let command_index = commands.len() - pending.len() - 1;
                self.configure_cmd(&mut next)
                    .with_context(|| format!("error while configuring command \"{next}\""))?;
                running.insert(
                    command_index,
                    next.to_owned().run(
                        self.options.use_shell.as_ref(),
                        on_terminate_for(command_index),
                    )?,
                );
                (self.command_started_handler)(&next);
            }

            if running.is_empty() {
                break;
            }

            let termination = termination_recv
                .recv()
                .context("error while waiting for command termination signal")??;

            let terminating = running.remove(&termination.command).unwrap();
            let terminated_cmd = terminating.command.to_owned();
            terminating
                .join()
                .with_context(|| format!("error while joining command {terminated_cmd} thread"))?;
            (self.command_terminated_handler)(&terminated_cmd, &termination.status);

            if !termination.status.success {
                match terminated_cmd.on_failure {
                    OnFailure::Restart => {
                        let _ = running.insert(
                            termination.command,
                            terminated_cmd.run(
                                self.options.use_shell.as_ref(),
                                on_terminate_for(termination.command),
                            )?,
                        );
                    }
                    OnFailure::Ignore => (),
                    OnFailure::ForceExit => return Self::kill_all(terminated_cmd, running),
                    OnFailure::Exit => {
                        return Self::command_failure(terminated_cmd, running, termination_recv)
                    }
                }
            }
        }

        Ok(())
    }

    fn kill_all(cause: Command, running: BTreeMap<usize, RunningCommand>) -> Result<()> {
        let kills = running
            .into_values()
            .map(RunningCommand::kill)
            .collect::<Vec<Result<()>>>();

        let killed = kills.iter().filter(|kill| kill.is_ok()).count();
        let errored = kills.len() - killed;

        let error_details = if killed + errored == 0 {
            None
        } else {
            let mut details: Vec<String> = vec![];
            if killed > 0 {
                details.push(format!("{killed} other processes were killed"));
            }
            if errored > 0 {
                details.push(format!("{errored} other processes could not be killed"));
            }
            Some(details.join(", "))
        };

        bail!(
            "command {} failed {}",
            cause,
            error_details
                .map(|details| format!("({details})"))
                .unwrap_or(String::default())
        )
    }

    fn command_failure(
        cause: Command,
        mut running: BTreeMap<usize, RunningCommand>,
        termination_recv: Receiver<Result<Termination>>,
    ) -> Result<()> {
        let detached = running.len();
        let statuses = (0..running.len())
            .map(|_| {
                let termination = termination_recv.recv()??;
                let terminated = running.remove(&termination.command).unwrap();
                terminated.join()?;
                Ok(termination.status)
            })
            .collect::<Result<Vec<ProcessStatus>>>()?;
        let failed = statuses.iter().filter(|status| !status.success).count();
        let mut error_msg = format!("command \"{cause}\" failed");
        if detached > 0 {
            error_msg.push(' ');
            error_msg.push_str(&if failed > 0 {
                format!("({failed} detached processes failed after initial failure)")
            } else {
                "(all detached processes exited successfully)".into()
            })
        }
        bail!(error_msg)
    }

    fn configure_cmd(&self, command: &mut Command) -> Result<()> {
        if let Some(cwd) = &mut command.cwd {
            if cwd.is_relative() {
                *cwd = self.options.default_cwd.join(&*cwd);
            }
        } else {
            let _ = command.cwd.insert(self.options.default_cwd.to_owned());
        }

        command
            .environment
            .extend(self.options.default_environment.to_owned());

        Ok(())
    }
}
