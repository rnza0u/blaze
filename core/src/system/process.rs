use std::{
    collections::HashMap,
    fmt::Display,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Command, ExitStatus, Stdio},
    sync::{RwLock, RwLockWriteGuard},
    thread::JoinHandle,
};

use anyhow::{anyhow, Context};

use blaze_common::error::Result;
use shared_child::SharedChild;

use super::thread::{join, thread};

// both read operations for Stdout and Stderr are done in separate threads.
type ReadThreadHandles = [JoinHandle<Result<()>>; 2];

/// A child process
pub struct Process {
    child: SharedChild,
    read_thread_handles: RwLock<Option<ReadThreadHandles>>,
}

/// Information about a process that has signaled its status.
pub struct ProcessStatus {
    pub code: Option<i32>,
    pub success: bool,
}

impl From<ExitStatus> for ProcessStatus {
    fn from(exit_status: ExitStatus) -> Self {
        ProcessStatus {
            code: exit_status.code(),
            success: exit_status.success(),
        }
    }
}

impl Display for Process {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "process {}", self.child.id())
    }
}

/// Options to create a [`Process`].
#[derive(Default, Clone)]
pub struct ProcessOptions {
    /// The current working directory.
    pub cwd: Option<PathBuf>,
    /// Should we display output (stderr and stdout).
    pub display_output: bool,
    /// Environment variables for the process.
    pub environment: HashMap<String, String>,
}

impl Process {
    /// Spawn a new process and return the associated [`Process`] struct.
    pub fn run_with_options<P, I, S>(
        program: P,
        arguments: I,
        options: ProcessOptions,
    ) -> Result<Self>
    where
        P: AsRef<Path>,
        S: AsRef<str>,
        I: IntoIterator<Item = S>,
    {
        let mut builder = Command::new(program.as_ref());

        if let Some(cwd) = &options.cwd {
            builder.current_dir(cwd);
        }

        for arg in arguments.into_iter() {
            builder.arg(arg.as_ref());
        }

        for (name, val) in options.environment {
            builder.env(name, val);
        }

        builder.stdin(Stdio::piped());
        builder.stdout(Stdio::piped());
        builder.stderr(Stdio::piped());

        let child = SharedChild::spawn(&mut builder)?;

        let process = Process {
            child,
            read_thread_handles: RwLock::new(None),
        };

        if options.display_output {
            fn pipe<F, T>(mut src: F, mut dst: T) -> JoinHandle<Result<()>>
            where
                F: Read + Send + 'static,
                T: Write + Send + 'static,
            {
                thread!(move || {
                    let mut buffer = [0_u8; 512];
                    loop {
                        let read = src.read(&mut buffer)?;
                        if read == 0 {
                            break;
                        }

                        #[cfg(not(windows))]
                        dst.write_all(&buffer[..read])?;

                        // when using a console on Windows, ChildStdout and ChildStderr do not support non-UTF8 streams
                        #[cfg(windows)]
                        dst.write_all(String::from_utf8_lossy(&buffer[..read]).as_bytes())?;
                    }

                    Ok(())
                })
            }

            *process
                .read_thread_handles
                .write()
                .map_err(|_| anyhow!("poison error for process read thread."))? = Some([
                pipe(
                    process
                        .child
                        .take_stdout()
                        .take()
                        .ok_or_else(|| anyhow!("could not take stdout for {process}."))?,
                    std::io::stdout(),
                ),
                pipe(
                    process
                        .child
                        .take_stderr()
                        .take()
                        .ok_or_else(|| anyhow!("could not take stderr for {process}."))?,
                    std::io::stderr(),
                ),
            ])
        }

        Ok(process)
    }

    /// Write some data to standard input and close.
    pub fn stdin_write(&mut self, data: &[u8]) -> Result<()> {
        self.child
            .take_stdin()
            .ok_or_else(|| anyhow!("stdin already closed"))?
            .write_all(data)
            .with_context(|| format!("failed to write to stdin for {self}."))
    }

    /// Get stdout from child process
    pub fn stdout(&self) -> Result<impl Read> {
        self.child
            .take_stdout()
            .ok_or_else(|| anyhow!("could not get process {self} stdout"))
    }

    /// Wait indefinitely for process termination.
    /// This does not take ownership or a mutable reference.
    pub fn wait(&self) -> Result<ProcessStatus> {
        let status = self
            .child
            .wait()
            .map(ProcessStatus::from)
            .with_context(|| format!("could not wait for {self}"))?;

        if let Some(threads) = self.thread_handle_write()?.take() {
            for thread in threads {
                join!(thread).context("error in command output processing")?;
            }
        }

        Ok(status)
    }

    /// Force termination of the process.
    pub fn kill(&self) -> Result<()> {
        self.child.kill()?;
        Ok(())
    }

    /// Get the process id.
    pub fn pid(&self) -> u32 {
        self.child.id()
    }

    fn thread_handle_write(&self) -> Result<RwLockWriteGuard<Option<ReadThreadHandles>>> {
        self.read_thread_handles
            .write()
            .map_err(|_| anyhow!("poison error (RwLock on process read thread)."))
    }
}
