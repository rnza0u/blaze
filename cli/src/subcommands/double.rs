use std::str::FromStr;

use anyhow::bail;
use blaze_common::error::{Error, Result};

#[derive(Debug, Clone)]
pub struct Double {
    pub target: String,
    pub project: Option<String>,
}

impl FromStr for Double {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let parts = s.split(':').collect::<Vec<_>>();
        let mut maybe_project: Option<String> = None;
        let target = match parts.as_slice() {
            [target] => target,
            [project, target] => {
                let _ = maybe_project.insert((*project).to_owned());
                target
            }
            _ => bail!("invalid double format was provided ({s})"),
        };
        Ok(Self {
            project: maybe_project,
            target: (*target).to_owned(),
        })
    }
}
