use std::{
    fs::{self, create_dir_all, File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::Mutex,
};

use anyhow::{anyhow, bail, Context};
use blaze_common::{error::Result, util::path_to_string, value::Value};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

use crate::{
    executors::bridge::bridge_executor,
    system::{
        env::Env,
        locks::ProcessLock,
        process::{Process, ProcessOptions},
        random::random_string,
    },
};

use super::{
    bridge::BridgeProcessParams,
    loader::{ExecutorLoader, DynCustomExecutor, LoadContext},
    Executor, ExecutorContext,
};

#[derive(Serialize, Deserialize)]
struct RustExecutorPackageMetadata {
    name: String,
    exported: String,
}

#[derive(Serialize)]
struct BridgeMetadata<'a> {
    #[serde(flatten)]
    executor_ref: &'a RustExecutor,
}

const BRIDGE_LOCATION: &str = ".blaze/rust";
const EXECUTORS_LOCATION: &str = ".blaze/rust/lib";

#[cfg(not(windows))]
const BRIDGE_EXECUTABLE_FILENAME: &str = "bridge";

#[cfg(windows)]
const BRIDGE_EXECUTABLE_FILENAME: &str = "bridge.exe";

const BRIDGE_CHECKSUM_FILENAME: &str = "checksum.txt";

const CARGO_TOML: &str = "Cargo.toml";

const CARGO_LOCATION_ENV: &str = "BLAZE_CARGO_LOCATION";
const DEFAULT_CARGO_LOCATION: &str = "cargo";

const BRIDGE_INSTALL_LOCK_ID: u64 = 1;

pub fn is_rust_executor(root: &Path) -> Result<bool> {
    Ok(match fs::metadata(root.join(CARGO_TOML)) {
        Ok(metadata) => metadata.is_file(),
        Err(err) if err.kind() == io::ErrorKind::NotFound => false,
        Err(err) => return Err(err.into()),
    })
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustExecutor {
    root: PathBuf,
    library_path: PathBuf,
    exported_symbol_name: String,
}

pub struct RustExecutorLoader;

impl ExecutorLoader for RustExecutorLoader {
    fn load_from_metadata(&self, metadata: &Value) -> Result<DynCustomExecutor> {
        Ok(Box::new(RustExecutor::deserialize(metadata)?))
    }

    fn load_from_src(&self, root: &Path, context: LoadContext<'_>) -> Result<DynCustomExecutor> {
        let cargo_file_path = root.join(CARGO_TOML);

        let package = read_rust_executor_metadata(&cargo_file_path)?;

        if !Process::run_with_options(
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
        )?
        .wait()?
        .success
        {
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
        let executors_location = context.workspace.root().join(EXECUTORS_LOCATION);

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

        Ok(Box::new(RustExecutor {
            root: root.to_owned(),
            library_path,
            exported_symbol_name: package.exported,
        }))
    }
}

static INSTALL_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

fn install_bridge_executable(workspace_root: &Path) -> Result<PathBuf> {
    let _mutex = INSTALL_MUTEX.lock().unwrap();

    let process_lock = ProcessLock::try_new(workspace_root, BRIDGE_INSTALL_LOCK_ID)?;

    process_lock.locked(|| {
        let embedded_bridge_executable_bytes =
            include_bytes!(env!("BLAZE_RUST_BRIDGE_EXECUTABLE_PATH"));
        let embedded_bridge_executable_checksum = env!("BLAZE_RUST_BRIDGE_CHECKSUM");

        let workspace_bridge_location = workspace_root.join(BRIDGE_LOCATION);
        let bin_path = workspace_bridge_location.join(BRIDGE_EXECUTABLE_FILENAME);
        let checksum_path = workspace_bridge_location.join(BRIDGE_CHECKSUM_FILENAME);

        match File::open(&checksum_path) {
            Ok(mut checksum_file) => {
                let mut checksum = String::with_capacity(64);
                checksum_file.read_to_string(&mut checksum)?;

                if checksum == embedded_bridge_executable_checksum {
                    return Ok(bin_path);
                }
            }
            Err(err) => {
                if err.kind() != std::io::ErrorKind::NotFound {
                    return Err(err).with_context(|| {
                        format!("could not read checksum at {}", checksum_path.display())
                    });
                }
            }
        }

        create_dir_all(&workspace_bridge_location).with_context(|| {
            format!(
                "error while creating Rust executor bridge directory at {}",
                workspace_bridge_location.display()
            )
        })?;

        let mut bridge_bin_options = OpenOptions::new();
        bridge_bin_options.create(true);
        bridge_bin_options.write(true);
        bridge_bin_options.truncate(true);

        #[cfg(unix)]
        {
            use std::os::unix::prelude::OpenOptionsExt;
            bridge_bin_options.mode(0o744);
        }

        let mut bridge_bin = bridge_bin_options.open(&bin_path).with_context(|| {
            format!(
                "error while opening rust bridge executable at {}",
                bin_path.display()
            )
        })?;

        let mut checksum = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&checksum_path)
            .with_context(|| {
                format!(
                    "could not open rust bridge executable checksum file at {}",
                    checksum_path.display()
                )
            })?;

        bridge_bin
            .write_all(embedded_bridge_executable_bytes)
            .context("error while writing Rust executor bridge binary")?;

        checksum
            .write_all(embedded_bridge_executable_checksum.as_bytes())
            .context("error while writing Rust executor bridge binary checksum")?;

        Ok(bin_path)
    })?
}

impl Executor for RustExecutor {
    fn execute(&self, context: ExecutorContext, options: Value) -> Result<()> {
        let bridge = install_bridge_executable(context.workspace.root())?;

        bridge_executor(
            (context, &options),
            BridgeProcessParams {
                program: path_to_string(bridge)?.as_str(),
                arguments: &[],
                input: None,
            },
            BridgeMetadata { executor_ref: self },
        )
    }
}

const CRATE_TYPE: &str = "lib.crate-type";
const NAME: &str = "package.name";
const EXPORTED: &str = "package.metadata.blaze.exported";
const TYPE: &str = "package.metadata.blaze.type";
const VERSION: &str = "package.metadata.blaze.version";

fn read_rust_executor_metadata(cargo_file_path: &Path) -> Result<RustExecutorPackageMetadata> {
    let content = std::fs::read_to_string(cargo_file_path)
        .with_context(|| format!("could not read {}", cargo_file_path.display()))?;
    let manifest = toml::from_str::<Value>(&content).with_context(|| {
        format!(
            "could not parse executor manifest located at {}",
            cargo_file_path.display()
        )
    })?;
    let crate_type = manifest
        .at(CRATE_TYPE)
        .and_then(|crate_type| crate_type.as_vec_and_then(|v| v.as_str().map(str::to_string)))
        .ok_or_else(|| {
            anyhow!(
                "[{CRATE_TYPE}] is missing from your executor file (in {})",
                cargo_file_path.display()
            )
        })?;

    let required_types = ["rlib", "dylib"];

    if required_types
        .iter()
        .any(|t| !crate_type.contains(&t.to_string()))
    {
        bail!("[{CRATE_TYPE}] must contain {required_types:?} types");
    }

    let name = manifest.at(NAME).and_then(Value::as_str).ok_or_else(|| {
        anyhow!(
            "[{NAME}] must contain a name for your executor (in {}).",
            cargo_file_path.display()
        )
    })?;

    let exported = manifest
        .at(EXPORTED)
        .and_then(Value::as_str)
        .ok_or_else(|| {
            anyhow!(
                "[{EXPORTED}] must be your executor function name as declared in your Rust code (in {}).",
                cargo_file_path.display()
            )
        })?;

    if !matches!(manifest.at(TYPE).and_then(Value::as_str), Some("executor")) {
        bail!(
            "[{TYPE}] must be \"executor\" in {}.",
            cargo_file_path.display()
        )
    }

    if !matches!(manifest.at(VERSION).and_then(Value::as_str), Some("1")) {
        bail!(
            "[{VERSION}] must have value \"1\" in {}.",
            cargo_file_path.display()
        )
    }

    Ok(RustExecutorPackageMetadata {
        name: name.to_owned(),
        exported: exported.to_owned(),
    })
}
