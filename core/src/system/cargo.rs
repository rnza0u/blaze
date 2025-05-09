use std::path::PathBuf;

use blaze_common::error::Result;

use super::{
    env::Env,
    process::{Process, ProcessOptions},
};

const CARGO_LOCATION_ENV: &str = "BLAZE_CARGO_LOCATION";
const DEFAULT_CARGO_LOCATION: &str = "cargo";

pub fn cargo<S: AsRef<str>, A: IntoIterator<Item = S>>(
    arguments: A,
    options: ProcessOptions,
) -> Result<Process> {
    let arguments: Vec<String> = arguments
        .into_iter()
        .map(|arg| arg.as_ref().to_owned())
        .collect();
    let (process_program, process_args) = format_cmd(arguments)?;

    Process::run_with_options(process_program, process_args, options)
}

fn format_cmd(arguments: Vec<String>) -> Result<(PathBuf, Vec<String>)> {
    Ok((PathBuf::from(get_location()?), arguments))
}

fn get_location() -> Result<String> {
    Ok(Env::get_as_str(CARGO_LOCATION_ENV)?.unwrap_or_else(|| DEFAULT_CARGO_LOCATION.to_owned()))
}
