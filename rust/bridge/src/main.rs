use anyhow::{anyhow, Context};
use blaze_common::{
    error::Result,
    logger::{LogLevel, Logger, LoggingStrategy},
    project::Project,
    value::Value,
    workspace::Workspace,
};
use blaze_devkit::{ExecutorContext, ExecutorFn};
use interprocess::local_socket::{
    traits::Stream as StreamTrait, GenericFilePath, Stream, ToFsName,
};
use libloading::{Library, Symbol};
use serde::{Deserialize, Serialize};
use std::{
    io::Write,
    panic::catch_unwind,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

#[derive(Deserialize)]
struct BridgeContext {
    workspace: Workspace,
    project: Project,
    target: String,
    logger: PathBuf,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BridgeInputMessage {
    metadata: RustExecutorMetadata,
    executor_params: (BridgeContext, Value),
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RustExecutorMetadata {
    library_path: PathBuf,
    exported_symbol_name: String,
}

pub struct BridgedLoggingStrategy {
    connection: Arc<Mutex<Stream>>,
}

impl BridgedLoggingStrategy {
    pub fn connect(path: &Path) -> Result<Self> {
        Ok(Self {
            connection: Arc::new(Mutex::new(Stream::connect(ToFsName::to_fs_name::<
                GenericFilePath,
            >(path)?)?)),
        })
    }
}

#[derive(Serialize)]
pub struct Log<'a> {
    message: &'a str,
    level: LogLevel,
}

impl LoggingStrategy for BridgedLoggingStrategy {
    fn log(&self, message: &str, level: blaze_common::logger::LogLevel)
    where
        Self: Sized,
    {
        let log_entry = Log { message, level };
        let log_entry_json = serde_json::to_string(&log_entry).unwrap();
        let _ = writeln!(self.connection.lock().unwrap(), "{}", log_entry_json);
    }
}

fn main() -> Result<()> {
    let bridge_message_value = &std::env::args()
        .nth(1)
        .ok_or_else(|| anyhow!("bridge input message not found in args"))?;

    let bridge_message: BridgeInputMessage = serde_json::from_str(bridge_message_value)
        .context("could not parse bridge input message.")?;

    let metadata = bridge_message.metadata;
    let (context, options) = bridge_message.executor_params;

    let logger = Logger::new(
        BridgedLoggingStrategy::connect(&context.logger).with_context(|| {
            format!(
                "could not connect to logger ({}).",
                context.logger.display()
            )
        })?,
    );

    let library: Library;
    let executor: Symbol<ExecutorFn>;

    unsafe {
        library = Library::new(&metadata.library_path).with_context(|| {
            format!(
                "could not load library at {}.",
                metadata.library_path.display()
            )
        })?;
        executor = library
            .get::<ExecutorFn>(metadata.exported_symbol_name.as_bytes())
            .with_context(|| {
                format!(
                    "could not load executor exported function \"{}\".",
                    metadata.exported_symbol_name
                )
            })?;
    }

    if let Err(error) = execute(
        *executor,
        ExecutorContext {
            project: &context.project,
            workspace: &context.workspace,
            target: &context.target,
            logger: &logger,
        },
        options,
    ) {
        logger.error(format!("executor error: {error:?}."));
        return Err(error);
    }

    Ok(())
}

fn execute(executor: ExecutorFn, context: ExecutorContext, options: Value) -> Result<()> {
    catch_unwind(|| executor(context, options))
        .map_err(|panic_error| {
            if panic_error.is::<String>() {
                anyhow!(
                    "executor panicked: {}.",
                    panic_error.downcast::<String>().unwrap().as_str()
                )
            } else if panic_error.is::<&str>() {
                anyhow!(
                    "executor panicked: {}.",
                    panic_error.downcast::<&str>().unwrap()
                )
            } else {
                anyhow!("executor panicked (unknown value received {panic_error:?}).")
            }
        })?
        .map_err(|executor_error| anyhow!(executor_error))
}
