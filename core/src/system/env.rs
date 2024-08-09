use std::{collections::HashMap, env::VarError, io::ErrorKind, path::Path};

use anyhow::{bail, Context};
use serde::de::DeserializeOwned;

use blaze_common::{error::Result, value::Value};

pub const MAIN_ENV_FILE: &str = ".env";
pub const USER_ENV_FILE: &str = ".user.env";

pub struct Env;

impl Env {
    pub fn get_as_str(name: &str) -> Result<Option<String>> {
        match std::env::var(name) {
            Err(VarError::NotPresent) => Ok(None),
            Ok(s) => Ok(Some(s)),
            Err(err) => bail!(err),
        }
    }

    pub fn get_and_deserialize<T>(name: &str) -> Result<Option<T>>
    where
        T: DeserializeOwned,
    {
        Self::get_as_str(name)?
            .map(|serialized| {
                serde_json::from_str(&serialized)
                    .with_context(|| format!("could not parse environment variable \"{name}\""))
            })
            .transpose()
    }

    pub fn load_dotenv_files(root: &Path) -> Result<()> {
        for path in [MAIN_ENV_FILE, USER_ENV_FILE]
            .into_iter()
            .map(|f| root.join(f))
        {
            match std::fs::metadata(&path)
                .map(|m| m.is_file())
                .map_err(|err| err.kind())
            {
                Ok(true) => dotenv::from_path(&path).with_context(|| {
                    format!("could not load dotenv file from {}", path.display())
                })?,
                Ok(false) | Err(ErrorKind::NotFound) => continue,
                Err(other_err) => bail!(other_err),
            }
        }

        Ok(())
    }

    pub fn get_all() -> HashMap<String, String> {
        HashMap::from_iter(std::env::vars())
    }

    pub fn get_all_as_value() -> Value {
        Value::Object(
            Self::get_all()
                .into_iter()
                .map(|(key, val)| (key, Value::string(val)))
                .collect(),
        )
    }

    pub fn create_dotenv_files(root: &Path) -> Result<()> {
        for (path, content) in [
            (
                root.join(MAIN_ENV_FILE),
                "\
# This environment variables file is versioned through Git and meant to be available publicly.\n\
# It is a good place to store default values for mandatory environment variables when they are not supplied by the user.\n\
# Do not store secrets in this file. Use the \".user.env\" file instead.\n"
            ),
            (
                root.join(USER_ENV_FILE),
                "\
# This environment variables file is ignored by Git and must contain values provided by the user on his own machine.\n"
            )
        ] {
            std::fs::write(&path, content).with_context(|| format!("failed to write to {}", path.display()))?;
        }
        Ok(())
    }
}
