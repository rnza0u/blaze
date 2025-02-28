use std::path::Path;

use anyhow::bail;
use blaze_common::{error::Result, util::path_to_string};

use crate::{
    executors::builder::ExecutorBuilder,
    system::{cargo::cargo, process::ProcessOptions},
};

pub struct RustExecutorBuilder;

impl ExecutorBuilder for RustExecutorBuilder {
    fn build(&self, root: &Path) -> Result<()> {
        let build_process = cargo(
            ["build", "--release", "--lib"],
            ProcessOptions {
                cwd: Some(root.to_path_buf()),
                display_output: true,
                environment: [(
                    "CARGO_TARGET_DIR".into(),
                    path_to_string(root.join("target"))?,
                )]
                .into(),
            },
        )?;

        let build_status = build_process.wait()?;

        if !build_status.success {
            bail!("could not build Rust executor at \"{}\"", root.display());
        }
        Ok(())
    }
}
