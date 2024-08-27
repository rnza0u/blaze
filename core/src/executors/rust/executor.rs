use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::Mutex,
};

use anyhow::Context;
use blaze_common::{error::Result, util::path_to_string, value::Value};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

use crate::{
    executors::{
        bridge::{bridge_executor, BridgeProcessParams},
        Executor, ExecutorContext,
    },
    system::locks::ProcessLock,
};

const BRIDGE_INSTALL_LOCK_ID: u64 = 1;

const BRIDGE_LOCATION: &str = ".blaze/rust";

#[cfg(not(windows))]
const BRIDGE_EXECUTABLE_FILENAME: &str = "bridge";

#[cfg(windows)]
const BRIDGE_EXECUTABLE_FILENAME: &str = "bridge.exe";

const BRIDGE_CHECKSUM_FILENAME: &str = "checksum.txt";

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustExecutor {
    library_path: PathBuf,
    exported_symbol_name: String,
}

impl RustExecutor {
    pub fn new(library_path: &Path, export_fn: &str) -> Self {
        Self {
            library_path: library_path.to_owned(),
            exported_symbol_name: export_fn.to_owned(),
        }
    }
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

#[derive(Serialize)]
struct BridgeMetadata<'a> {
    #[serde(flatten)]
    executor_ref: &'a RustExecutor,
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

        std::fs::create_dir_all(&workspace_bridge_location).with_context(|| {
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
