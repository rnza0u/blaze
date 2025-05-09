use anyhow::{anyhow, bail, Context};
use blaze_common::{error::Result, value::Value};
use serde::{Deserialize, Serialize};
use std::{
    io,
    path::{Path, PathBuf},
};

pub fn is_node_executor(root: &Path) -> Result<bool> {
    match std::fs::metadata(root.join(PACKAGE_JSON)) {
        Ok(metadata) => Ok(metadata.is_file()),
        Err(err) => {
            if err.kind() == io::ErrorKind::NotFound {
                Ok(false)
            } else {
                Err(err.into())
            }
        }
    }
}

const PACKAGE_JSON: &str = "package.json";

const PACKAGE_METADATA_VERSION_KEY: &str = "blaze.version";
const PACKAGE_METADATA_VERSION: &str = "1";

const PACKAGE_METADATA_TYPE_KEY: &str = "blaze.type";
const PACKAGE_METADATA_TYPE: &str = "executor";

const PACKAGE_METADATA_PATH_KEY: &str = "blaze.path";
const PACKAGE_METADATA_INSTALL_KEY: &str = "blaze.install";
const PACKAGE_METADATA_BUILD_KEY: &str = "blaze.build";

const DEFAULT_BUILD_SCRIPT: &str = "build";
const PACKAGE_SCRIPTS_KEY: &str = "scripts";

#[derive(Clone, Serialize, Deserialize)]
pub struct NodeExecutorPackage {
    build: Option<String>,
    install: bool,
    path: PathBuf,
    version: String,
}

impl NodeExecutorPackage {
    pub fn from_root(executor_root: &Path) -> Result<Self> {
        let package_json_path = executor_root.join(PACKAGE_JSON);
        let content = std::fs::read_to_string(&package_json_path)
            .with_context(|| format!("could not read {}.", package_json_path.display()))?;
        let value = serde_json::from_str::<Value>(&content)
            .with_context(|| format!("could not parse executor {PACKAGE_JSON}."))?;

        let version = value
            .at(PACKAGE_METADATA_VERSION_KEY)
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("node executor package.json must contain a `{PACKAGE_METADATA_VERSION_KEY}` string property."))?;

        if version != PACKAGE_METADATA_VERSION {
            bail!(
                "`version` key must have value \"{PACKAGE_METADATA_VERSION}\", got \"{version}\"."
            );
        }

        let executor_type = value
            .at(PACKAGE_METADATA_TYPE_KEY)
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("node executor package.json must contain a `{PACKAGE_METADATA_TYPE_KEY}` string property."))?;

        if executor_type != PACKAGE_METADATA_TYPE {
            bail!("`{PACKAGE_METADATA_TYPE_KEY}` key must have value \"{PACKAGE_METADATA_TYPE}\", got \"{executor_type}\".");
        }

        let path = value
            .at(PACKAGE_METADATA_PATH_KEY)
            .and_then(Value::as_str)
            .ok_or_else(|| {
                anyhow!(
                    "node executor package.json must contain a `{PACKAGE_METADATA_PATH_KEY}` string property."
                )
            })?;

        let default_build_script_present = value
            .at(PACKAGE_SCRIPTS_KEY)
            .and_then(Value::as_object)
            .map(|scripts| scripts.contains_key(DEFAULT_BUILD_SCRIPT))
            .unwrap_or_default();

        let build = match value.at(PACKAGE_METADATA_BUILD_KEY){
            Some(Value::Bool(false)) => None,
            Some(Value::String(script)) => Some(script.to_owned()),
            Some(Value::Bool(true)) => {

                if !default_build_script_present {
                    bail!("default \"{DEFAULT_BUILD_SCRIPT}\" script is not present and {PACKAGE_METADATA_BUILD_KEY} is explicitely set to true")
                }

                Some(DEFAULT_BUILD_SCRIPT.to_owned())
            },
            None => if default_build_script_present {
                Some(DEFAULT_BUILD_SCRIPT.to_owned())
            } else {
                None
            },
            _ => bail!("invalid value in `{PACKAGE_METADATA_BUILD_KEY}`. must be either a boolean or a build script name.")
        };

        let install = value
            .at(PACKAGE_METADATA_INSTALL_KEY)
            .map(|value| {
                value.as_bool().ok_or_else(|| {
                    anyhow!("invalid value in `{PACKAGE_METADATA_INSTALL_KEY}`. must be a boolean")
                })
            })
            .transpose()?
            .unwrap_or(true);

        let relative_module_path = Path::new(path);

        if !relative_module_path.is_relative() {
            bail!("{PACKAGE_METADATA_PATH_KEY} must be relative to the executor root")
        }

        Ok(Self {
            build,
            install,
            path: executor_root.join(relative_module_path),
            version: version.to_owned(),
        })
    }

    pub fn build(&self) -> Option<&str> {
        self.build.as_deref()
    }

    pub fn install(&self) -> bool {
        self.install
    }

    pub fn module_path(&self) -> &Path {
        &self.path
    }
}
