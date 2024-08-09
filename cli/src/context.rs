use std::path::PathBuf;

use anyhow::Context;
use blaze_common::error::Result;

#[derive(Debug)]
pub struct CliContext {
    pub cwd: PathBuf,
}

impl CliContext {
    pub fn try_new() -> Result<Self> {
        let cwd =
            std::env::current_dir().context("could not get current directory for CLI context")?;

        Ok(CliContext { cwd })
    }
}
