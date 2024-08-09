use std::path::{Path, PathBuf};

use serde::{de::Error, Deserialize, Serialize};
use strum_macros::{Display, EnumIter};

use crate::{
    enums::{unit_enum_deserialize, unit_enum_from_str},
    util::normalize_path,
};

#[derive(Debug, Serialize)]
pub struct Shell {
    program: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    kind: Option<ShellKind>,
}

impl Shell {
    pub fn new(program: &Path, kind: Option<ShellKind>) -> Self {
        Self {
            program: program.to_owned(),
            kind,
        }
    }

    pub fn program(&self) -> &Path {
        &self.program
    }

    pub fn kind(&self) -> Option<ShellKind> {
        self.kind
    }
}

impl std::fmt::Display for Shell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.program.display().to_string())
    }
}

impl<'de> Deserialize<'de> for Shell {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(remote = "Shell")]
        struct ShellObject {
            program: PathBuf,
            kind: Option<ShellKind>,
        }

        #[derive(Deserialize)]
        #[serde(untagged)]
        enum ShellDeserializationMode {
            Path(PathBuf),
            #[serde(with = "ShellObject")]
            Object(Shell),
        }

        Ok(match ShellDeserializationMode::deserialize(deserializer)? {
            ShellDeserializationMode::Object(mut shell) => {
                shell.program = normalize_path(&shell.program).map_err(D::Error::custom)?;
                shell
            }
            ShellDeserializationMode::Path(program) => Shell {
                program: normalize_path(program).map_err(D::Error::custom)?,
                kind: None,
            },
        })
    }
}

#[derive(Debug, Display, Serialize, EnumIter, Clone, Copy)]
pub enum ShellKind {
    Posix,
    Cmd,
    Powershell,
}

unit_enum_from_str!(ShellKind);
unit_enum_deserialize!(ShellKind);
