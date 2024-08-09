use std::path::PathBuf;

use hash_value::Value;
use serde::{de::Error, Deserialize};

use crate::{configuration_file::ConfigurationFileFormat, util::normalize_path};

pub struct ExtraVariablesFileEntry {
    pub path: PathBuf,
    pub optional: bool,
}

impl From<PathBuf> for ExtraVariablesFileEntry {
    fn from(path: PathBuf) -> Self {
        Self {
            path,
            optional: false,
        }
    }
}

impl<'de> Deserialize<'de> for ExtraVariablesFileEntry {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(remote = "ExtraVariablesFileEntry")]
        struct ExtraVariablesFileEntryObject {
            path: PathBuf,
            optional: bool,
        }

        #[derive(Deserialize)]
        #[serde(untagged)]
        enum ExtraVariablesFileEntryDeserializationMode {
            AsPath(PathBuf),
            #[serde(with = "ExtraVariablesFileEntryObject")]
            AsObject(ExtraVariablesFileEntry),
        }

        Ok(
            match ExtraVariablesFileEntryDeserializationMode::deserialize(deserializer)? {
                ExtraVariablesFileEntryDeserializationMode::AsObject(mut obj) => {
                    obj.path = normalize_path(&obj.path).map_err(D::Error::custom)?;
                    obj
                }
                ExtraVariablesFileEntryDeserializationMode::AsPath(path) => {
                    normalize_path(path).map_err(D::Error::custom)?.into()
                }
            },
        )
    }
}

#[derive(Deserialize)]
pub struct VariablesConfiguration {
    #[serde(default)]
    pub vars: Value,
    #[serde(default)]
    pub include: Vec<ExtraVariablesFileEntry>,
}

#[derive(Clone)]
pub enum VariablesOverride {
    String {
        path: Vec<String>,
        value: String,
    },
    Code {
        format: ConfigurationFileFormat,
        code: String,
    },
    File {
        path: PathBuf,
    },
}
