use blaze_common::error::Result;

use crate::system::{
    env::Env,
    process::{Process, ProcessOptions},
};
use std::path::PathBuf;

const NPM_LOCATION: &str = "npm";
const OVERRIDE_NPM_LOCATION_ENVIRONMENT_VARIABLE: &str = "BLAZE_NPM_LOCATION";

pub fn npm<S: AsRef<str>, A: IntoIterator<Item = S>>(
    arguments: A,
    options: ProcessOptions,
) -> Result<Process> {
    let normalized_args: Vec<String> = arguments
        .into_iter()
        .map(|arg| arg.as_ref().to_owned())
        .collect();
    let (process_program, process_args) = format_cmd(normalized_args)?;

    Process::run_with_options(process_program, process_args, options)
}

fn get_location() -> Result<String> {
    Ok(Env::get_as_str(OVERRIDE_NPM_LOCATION_ENVIRONMENT_VARIABLE)?
        .unwrap_or_else(|| NPM_LOCATION.to_owned()))
}

#[cfg(not(windows))]
fn format_cmd(arguments: Vec<String>) -> Result<(PathBuf, Vec<String>)> {
    Ok((PathBuf::from(get_location()?), arguments))
}

#[cfg(windows)]
fn format_cmd(arguments: Vec<String>) -> Result<(PathBuf, Vec<String>)> {
    Ok((
        PathBuf::from("powershell.exe"),
        vec![
            "-c".into(),
            format!("{} {}", get_location()?, arguments.join(" ")),
        ],
    ))
}
