use std::path::Path;

use anyhow::{bail, Context};
use blaze_common::error::Result;

use crate::{
    executors::builder::ExecutorBuilder,
    system::{npm::npm, process::ProcessOptions},
};

use super::package::NodeExecutorPackage;

pub struct NodeExecutorBuilder;

impl ExecutorBuilder for NodeExecutorBuilder {
    fn build(&self, root: &Path) -> Result<()> {
        let package = NodeExecutorPackage::from_root(root)?;
        if package.install() {
            let install_status = npm(
                ["install"],
                ProcessOptions {
                    cwd: Some(root.to_path_buf()),
                    display_output: true,
                    ..Default::default()
                },
            )
            .context("could not start node executor install process")?
            .wait()?;

            if !install_status.success {
                bail!(
                    "node executor installation failed (path={}, exitcode={:?})",
                    root.display(),
                    install_status.code
                );
            }
        }

        if let Some(script) = &package.build() {
            let build_status = npm(
                ["run", script],
                ProcessOptions {
                    cwd: Some(root.to_path_buf()),
                    display_output: true,
                    ..Default::default()
                },
            )
            .context("could not start node executor build process")?
            .wait()?;

            if !build_status.success {
                bail!(
                    "node executor build failed (path={}, exitcode={:?})",
                    root.display(),
                    build_status.code
                );
            }
        }

        Ok(())
    }
}
