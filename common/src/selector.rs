use std::{borrow::Cow, collections::BTreeSet, fmt::Display};

use serde::{Deserialize, Serialize};

/// Used for selecting projects across the workspace.
#[derive(Clone, Debug, Hash, Serialize, Deserialize)]
pub enum ProjectSelector {
    All,
    #[serde(untagged)]
    Array(BTreeSet<String>),
    #[serde(untagged)]
    IncludeExclude {
        include: BTreeSet<String>,
        exclude: BTreeSet<String>,
    },
    #[serde(untagged)]
    Tagged(BTreeSet<String>),
}

impl Display for ProjectSelector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&match self {
            Self::All => Cow::Borrowed("all projects"),
            Self::Array(names) => format!("projects with names: {names:?}").into(),
            Self::IncludeExclude { include, exclude } => {
                format!("projects matching expressions {include:?}, excluding {exclude:?}").into()
            }
            Self::Tagged(tags) => format!("projects tagged with: {tags:?}").into(),
        })
    }
}

impl ProjectSelector {
    pub fn all() -> Self {
        Self::All
    }

    pub fn array<N: IntoIterator<Item = S>, S: AsRef<str>>(names: N) -> Self {
        Self::Array(names.into_iter().map(|name| name.as_ref().into()).collect())
    }

    pub fn include_exclude<S: AsRef<str>, I: IntoIterator<Item = S>, E: IntoIterator<Item = S>>(
        include: I,
        exclude: E,
    ) -> Self {
        Self::IncludeExclude {
            include: include
                .into_iter()
                .map(|pattern| pattern.as_ref().into())
                .collect(),
            exclude: exclude
                .into_iter()
                .map(|pattern| pattern.as_ref().into())
                .collect(),
        }
    }

    pub fn tagged<S: AsRef<str>, I: IntoIterator<Item = S>>(tags: I) -> Self {
        Self::Tagged(tags.into_iter().map(|s| s.as_ref().to_owned()).collect())
    }
}
