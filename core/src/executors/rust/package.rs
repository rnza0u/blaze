use std::path::Path;

use anyhow::{anyhow, bail, Context};
use blaze_common::{error::Result, value::Value};

const CARGO_TOML: &str = "Cargo.toml";

const CRATE_TYPE: &str = "lib.crate-type";
const NAME: &str = "package.name";
const EXPORTED: &str = "package.metadata.blaze.exported";
const TYPE: &str = "package.metadata.blaze.type";
const VERSION: &str = "package.metadata.blaze.version";

pub fn is_rust_executor(root: &Path) -> Result<bool> {
    Ok(match std::fs::metadata(root.join(CARGO_TOML)) {
        Ok(metadata) => metadata.is_file(),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => false,
        Err(err) => return Err(err.into()),
    })
}

pub struct RustExecutorPackage {
    pub name: String,
    pub exported_fn: String,
}

impl RustExecutorPackage {
    pub fn from_root(root: &Path) -> Result<Self> {
        let cargo_file_path = root.join(CARGO_TOML);

        let content = std::fs::read_to_string(&cargo_file_path)
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
                "[{TYPE}] must be \"executor\" (in {}).",
                cargo_file_path.display()
            )
        }

        if !matches!(manifest.at(VERSION).and_then(Value::as_str), Some("1")) {
            bail!(
                "[{VERSION}] must have value \"1\" (in {}).",
                cargo_file_path.display()
            )
        }

        Ok(Self {
            name: name.to_owned(),
            exported_fn: exported.to_owned(),
        })
    }
}
