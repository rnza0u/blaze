use std::{
    collections::HashMap,
    convert::Infallible,
    fmt::Display,
    path::PathBuf,
    str::FromStr,
    sync::{mpsc::channel, Arc},
    thread::JoinHandle,
    time::Duration,
};

use anyhow::{anyhow, Context};
use blaze_common::{error::Result, unit_enum_deserialize, unit_enum_from_str};
use serde::Deserialize;
use strum_macros::{Display, EnumIter};

use crate::system::{
    process::{Process, ProcessOptions, ProcessStatus},
    shell::ShellFormatter,
    thread::{join, thread},
};

use super::UseShell;

#[derive(Default, Clone, Display, EnumIter, Copy)]
pub enum OnFailure {
    Ignore,
    Restart,
    #[default]
    Exit,
    ForceExit,
}

unit_enum_from_str!(OnFailure);
unit_enum_deserialize!(OnFailure);

#[derive(Clone)]
pub enum Argv {
    Line(String),
    Vec(PathBuf, Vec<String>),
}

#[derive(Clone)]
pub struct Command {
    pub detach: bool,
    pub argv: Argv,
    pub on_failure: OnFailure,
    pub cwd: Option<PathBuf>,
    pub environment: HashMap<String, String>,
    pub quiet: bool,
}

impl FromStr for Command {
    type Err = Infallible;

    fn from_str(s: &str) -> std::result::Result<Self, Infallible> {
        Ok(Self {
            argv: Argv::Line(s.to_owned()),
            cwd: None,
            detach: false,
            environment: Default::default(),
            on_failure: OnFailure::default(),
            quiet: false,
        })
    }
}

impl<'de> Deserialize<'de> for Command {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        struct CommandObject {
            program: PathBuf,
            #[serde(default)]
            detach: bool,
            #[serde(default)]
            arguments: Vec<String>,
            #[serde(default)]
            on_failure: OnFailure,
            cwd: Option<PathBuf>,
            #[serde(default)]
            environment: HashMap<String, String>,
            #[serde(default)]
            quiet: bool,
        }

        #[derive(Deserialize)]
        #[serde(untagged)]
        enum CommandDeserializationMode {
            Raw(String),
            Full(CommandObject),
        }

        Ok(
            match CommandDeserializationMode::deserialize(deserializer)? {
                CommandDeserializationMode::Raw(single_command) => {
                    Command::from_str(&single_command).unwrap()
                }
                CommandDeserializationMode::Full(command) => Command {
                    argv: Argv::Vec(command.program, command.arguments),
                    cwd: command.cwd,
                    detach: command.detach,
                    environment: command.environment,
                    on_failure: command.on_failure,
                    quiet: command.quiet,
                },
            },
        )
    }
}

impl Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.argv {
            Argv::Line(line) => f.write_str(line),
            Argv::Vec(program, arguments) => {
                write!(f, "{} {}", program.display(), arguments.join(" "))
            }
        }
    }
}

impl Command {
    pub fn run<T: FnOnce(Result<ProcessStatus>) -> Result<()> + Send + 'static>(
        self,
        use_shell: Option<&UseShell>,
        termination: T,
    ) -> Result<RunningCommand> {
        let (process_send, process_recv) = channel::<Result<Arc<Process>>>();

        let self_clone = self.to_owned();

        let (program, arguments) = match (use_shell, self.argv.clone()) {
            (None, Argv::Vec(program, arguments)) => (program, arguments),
            (Some(use_shell), argv) => {
                let formatter = match use_shell {
                    UseShell::Custom(shell) => ShellFormatter::from_shell(shell),
                    UseShell::SystemDefault => ShellFormatter::default(),
                };
                match argv {
                    Argv::Line(line) => formatter.format_command(&line)?,
                    Argv::Vec(program, arguments) => {
                        formatter.format_program_and_args(program, arguments)?
                    }
                }
            }
            (None, Argv::Line(line)) => {
                let mut split = line.split_whitespace();
                let program = PathBuf::from(split.next().ok_or_else(|| anyhow!("command is empty"))?);
                (program, split.map(str::to_owned).collect())
            }
        };

        let process_thread = thread!(move || {
            let process = match Process::run_with_options(
                &program,
                &arguments,
                ProcessOptions {
                    cwd: self_clone.cwd.to_owned(),
                    display_output: !self_clone.quiet,
                    environment: self_clone.environment.to_owned(),
                },
            )
            .with_context(|| format!("error while creating process for command \"{self_clone}\"."))
            {
                Ok(process) => process,
                Err(err) => {
                    process_send.send(Err(err))?;
                    return Ok(());
                }
            };

            let pid = process.pid();

            let arc_process_0 = Arc::new(process);

            process_send
                .send(Ok(arc_process_0.clone()))
                .context("error while sending process to main thread.")?;

            termination(
                arc_process_0
                    .wait()
                    .with_context(|| format!("error while waiting for process {pid}.")),
            )
            .context("error in command thread termination routine.")?;

            Ok(())
        });

        Ok(RunningCommand {
            thread: process_thread,
            process: process_recv
                .recv_timeout(Duration::from_secs(120))
                .with_context(|| {
                    format!(
                    "error while waiting for process structure from command thread for \"{self}\"."
                )
                })??,
            command: self,
        })
    }
}

pub struct RunningCommand {
    pub command: Command,
    thread: JoinHandle<Result<()>>,
    process: Arc<Process>,
}

impl RunningCommand {
    pub fn kill(self) -> Result<()> {
        self.process.kill()?;
        self.join()?;
        Ok(())
    }

    pub fn join(self) -> Result<()> {
        join!(self.thread)
    }
}
