use std::path::{Path, PathBuf};

use anyhow::bail;
use blaze_common::{
    error::Result,
    util::path_to_string,
    value::{to_value, Value},
};
use serde::Deserialize;

use crate::{
    executors::{
        loader::{ExecutorLoader, ExecutorWithMetadata},
        rust::executor::RustExecutor,
        DynExecutor,
    },
    system::{
        env::Env,
        process::{Process, ProcessOptions},
        random::random_string,
    },
};

use super::package::RustExecutorPackage;

const CARGO_LOCATION_ENV: &str = "BLAZE_CARGO_LOCATION";
const DEFAULT_CARGO_LOCATION: &str = "cargo";

const EXECUTORS_LOCATION: &str = ".blaze/rust/lib";

pub struct LocalRustExecutorLoader {
    workspace_root: PathBuf,
}

impl LocalRustExecutorLoader {
    pub fn new(workspace_root: &Path) -> Self {
        Self {
            workspace_root: workspace_root.to_owned(),
        }
    }
}

impl ExecutorLoader for LocalRustExecutorLoader {
    fn load_from_metadata(&self, metadata: &Value) -> Result<DynExecutor> {
        Ok(Box::new(RustExecutor::deserialize(metadata)?))
    }

    fn load_from_src(&self, root: &Path) -> Result<ExecutorWithMetadata> {
        let package = RustExecutorPackage::from_root(root)?;

        let build_process = Process::run_with_options(
            Env::get_as_str(CARGO_LOCATION_ENV)?
                .unwrap_or_else(|| DEFAULT_CARGO_LOCATION.to_owned()),
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

        fn format_lib_path(root: &Path, name: &str) -> PathBuf {
            let formatted_name = name.replace('-', "_");
            #[cfg(not(windows))]
            return root.join(format!("target/release/lib{formatted_name}.so"));
            #[cfg(windows)]
            root.join(format!("target\\release\\{formatted_name}.dll"))
        }

        let library_target_path = format_lib_path(root, &package.name);
        let executors_location = self.workspace_root.join(EXECUTORS_LOCATION);

        std::fs::create_dir_all(&executors_location)?;

        let library_name = format!(
            "{}.{}",
            random_string(16),
            library_target_path.extension().unwrap().to_str().unwrap()
        );

        let library_path = executors_location.join(library_name);

        std::fs::copy(library_target_path, &library_path)?;

        if !Process::run_with_options(
            Env::get_as_str(CARGO_LOCATION_ENV)?
                .unwrap_or_else(|| DEFAULT_CARGO_LOCATION.to_owned()),
            ["clean"],
            ProcessOptions {
                cwd: Some(root.to_path_buf()),
                display_output: true,
                environment: [(
                    "CARGO_TARGET_DIR".into(),
                    path_to_string(root.join("target"))?,
                )]
                .into(),
            },
        )?
        .wait()?
        .success
        {
            bail!(
                "could not clean Rust executor files at \"{}\"",
                root.display()
            );
        }

        let executor = RustExecutor::new(&library_path, &package.exported_fn);
        let metadata = to_value(&executor)?;

        Ok(ExecutorWithMetadata {
            executor: Box::new(executor),
            metadata,
        })
    }
}
