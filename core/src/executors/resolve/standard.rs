use anyhow::bail;
use blaze_common::error::Result;
use url::Url;

use crate::executors::{
    std::{CommandsExecutor, ExecExecutor},
    DynExecutor,
};

/// Resolves an executor with the std scheme.
pub fn resolve_standard_executor(url: &Url) -> Result<DynExecutor> {
    let name = url.path();
    Ok(match name {
        "commands" => Box::new(CommandsExecutor {}),
        "exec" => Box::new(ExecExecutor {}),
        _ => bail!("{name} is not a standard executor"),
    })
}
