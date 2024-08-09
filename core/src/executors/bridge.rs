use std::{
    io::{BufRead, BufReader},
    path::PathBuf,
    thread::scope,
};

use anyhow::{anyhow, Context};
use blaze_common::{
    error::{Error, Result},
    logger::{LogLevel, Logger},
    value::Value,
};

use serde::{Deserialize, Serialize};

use crate::system::{
    ipc_server::IpcServer,
    process::{Process, ProcessOptions, ProcessStatus},
};

use super::{env::get_executor_env, ExecutorContext};

#[derive(Serialize)]
pub struct BridgedExecutorContext<'a> {
    #[serde(flatten)]
    context: ExecutorContext<'a>,
    logger: PathBuf,
}

pub type ExecutorParams<'a> = (ExecutorContext<'a>, &'a Value);

pub struct BridgeProcessParams<'p> {
    pub program: &'p str,
    pub arguments: &'p [String],
    pub input: Option<&'p [u8]>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeInputMessage<'a, T: Serialize> {
    executor_params: (BridgedExecutorContext<'a>, &'a Value),
    metadata: T,
}

pub fn bridge_executor<T: Serialize>(
    (context, options): ExecutorParams<'_>,
    process_params: BridgeProcessParams<'_>,
    metadata: T,
) -> Result<()> {
    scope(|scope| {
        let logger_0 = context.logger.clone();
        let logger_1 = context.logger.clone();

        let logs = IpcServer::create(
            scope,
            move |connection| {
                process_logs(BufReader::new(connection), &logger_0)
                    .context("failure while parsing executor bridge process logs")
            },
            move |err: Error| {
                logger_1.error(format!("executor bridge ipc error: {err:?}"));
            },
        )
        .context("error while creating executor logs pipe")?;

        let project_root_directory = context.project.root().to_path_buf();

        let process_env = get_executor_env(&context)?;

        let process_input = BridgeInputMessage {
            executor_params: (
                BridgedExecutorContext {
                    context,
                    logger: logs.get_path().to_path_buf(),
                },
                options,
            ),
            metadata,
        };

        let serialized_input_message = serde_json::to_string(&process_input)
            .context("could not serialize bridge parameters")?;

        let mut arguments = process_params.arguments.to_vec();
        arguments.push(serialized_input_message);

        let mut process = Process::run_with_options(
            process_params.program,
            arguments,
            ProcessOptions {
                cwd: Some(project_root_directory),
                display_output: true,
                environment: process_env,
            },
        )
        .context("error while creating executor process")?;

        if let Some(input) = process_params.input {
            process
                .stdin_write(input)
                .context("error while writing to process stdin.")?;
        }

        let result = match process
            .wait()
            .context("error while waiting for bridge process output.")?
        {
            ProcessStatus { success: true, .. } => Ok(()),
            ProcessStatus {
                success: false,
                code: Some(code),
            } => Err(anyhow!("bridge process failed with status code {code}.")),
            ProcessStatus {
                success: false,
                code: None,
            } => Err(anyhow!("bridge process was terminated.")),
        };

        logs.close()
            .context("could not close logging IPC server.")?;

        result
    })
}

#[derive(Deserialize)]
struct LogEntry {
    message: String,
    level: LogLevel,
}

pub fn process_logs(stream: impl BufRead, logger: &Logger) -> Result<()> {
    for line in stream.lines() {
        let log = line.context("error occured while processing log line")?;
        let entry = serde_json::from_str::<LogEntry>(&log)
            .with_context(|| format!("error occurred while parsing log line: {log}"))?;
        logger.log(&entry.message, entry.level)
    }
    Ok(())
}
