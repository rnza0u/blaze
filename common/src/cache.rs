use std::{
    collections::{BTreeMap, BTreeSet},
    convert::Infallible,
    num::NonZeroUsize,
    path::{Path, PathBuf},
    str::FromStr,
};

use serde::{de::Error, Deserialize, Deserializer, Serialize};
use strum_macros::{Display, EnumIter};

use crate::{
    enums::{unit_enum_deserialize, unit_enum_from_str},
    error::Result,
    util::normalize_path,
};

#[derive(Debug, Clone, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetCache {
    #[serde(default)]
    invalidate_when: InvalidationStrategy,
}

impl TargetCache {
    pub fn invalidate_when(&self) -> &InvalidationStrategy {
        &self.invalidate_when
    }
}

#[derive(Debug, Default, Clone, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InvalidationStrategy {
    #[serde(skip_serializing_if = "Option::is_none")]
    input_changes: Option<BTreeSet<FileChangesMatcher>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    output_changes: Option<BTreeSet<FileChangesMatcher>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_files_missing"
    )]
    files_missing: Option<BTreeSet<PathBuf>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    expired: Option<TtlOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    command_fails: Option<CommandFailsOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    env_changes: Option<EnvChangesOptions>,
}

impl InvalidationStrategy {
    pub fn output_changes(&self) -> Option<&BTreeSet<FileChangesMatcher>> {
        self.output_changes.as_ref()
    }

    pub fn input_changes(&self) -> Option<&BTreeSet<FileChangesMatcher>> {
        self.input_changes.as_ref()
    }

    pub fn files_missing(&self) -> Option<&BTreeSet<PathBuf>> {
        self.files_missing.as_ref()
    }

    pub fn expired(&self) -> Option<&TtlOptions> {
        self.expired.as_ref()
    }

    pub fn command_fails(&self) -> Option<&CommandFailsOptions> {
        self.command_fails.as_ref()
    }

    pub fn env_changes(&self) -> Option<&EnvChangesOptions> {
        self.env_changes.as_ref()
    }
}

fn deserialize_files_missing<'de, D>(
    deserializer: D,
) -> std::result::Result<Option<BTreeSet<PathBuf>>, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(Some(
        BTreeSet::<PathBuf>::deserialize(deserializer)?
            .into_iter()
            .map(normalize_path)
            .collect::<Result<_>>()
            .map_err(D::Error::custom)?,
    ))
}

#[derive(
    Default, Clone, Copy, Debug, Hash, EnumIter, PartialEq, Eq, PartialOrd, Ord, Display, Serialize,
)]
pub enum MatchingBehavior {
    Timestamps,
    #[default]
    Mixed,
    Hash,
}

unit_enum_from_str!(MatchingBehavior);
unit_enum_deserialize!(MatchingBehavior);

#[derive(Debug, Hash, Clone, PartialEq, Eq, Serialize, PartialOrd, Ord)]
pub struct FileChangesMatcher {
    pattern: String,
    exclude: BTreeSet<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    root: Option<PathBuf>,
    behavior: MatchingBehavior,
}

impl FileChangesMatcher {
    pub fn new(pattern: &str) -> Self {
        Self {
            pattern: pattern.to_owned(),
            exclude: BTreeSet::new(),
            root: None,
            behavior: MatchingBehavior::default(),
        }
    }

    pub fn with_exclude<I: IntoIterator<Item = S>, S: AsRef<str>>(mut self, patterns: I) -> Self {
        self.exclude = patterns
            .into_iter()
            .map(|s| s.as_ref().to_owned())
            .collect();
        self
    }

    pub fn with_root(mut self, root: &Path) -> Self {
        self.root = Some(root.to_owned());
        self
    }

    pub fn with_behavior(mut self, behavior: MatchingBehavior) -> Self {
        self.behavior = behavior;
        self
    }

    pub fn pattern(&self) -> &str {
        &self.pattern
    }

    pub fn exclude(&self) -> &BTreeSet<String> {
        &self.exclude
    }

    pub fn root(&self) -> Option<&Path> {
        self.root.as_deref()
    }

    pub fn behavior(&self) -> MatchingBehavior {
        self.behavior
    }
}

impl FromStr for FileChangesMatcher {
    type Err = Infallible;

    fn from_str(s: &str) -> std::result::Result<Self, Infallible> {
        Ok(Self {
            pattern: s.to_string(),
            behavior: MatchingBehavior::default(),
            exclude: BTreeSet::new(),
            root: None,
        })
    }
}

impl<'de> Deserialize<'de> for FileChangesMatcher {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(remote = "FileChangesMatcher")]
        struct FileChangesMatcherObject {
            pattern: String,
            #[serde(default)]
            exclude: BTreeSet<String>,
            root: Option<PathBuf>,
            #[serde(default)]
            behavior: MatchingBehavior,
        }

        #[derive(Deserialize)]
        #[serde(untagged)]
        enum FileChangesMatcherDeserializationMode {
            SinglePattern(String),
            #[serde(with = "FileChangesMatcherObject")]
            Full(FileChangesMatcher),
        }

        Ok(
            match FileChangesMatcherDeserializationMode::deserialize(deserializer)? {
                FileChangesMatcherDeserializationMode::SinglePattern(pattern) => {
                    FileChangesMatcher::from_str(pattern.as_str()).unwrap()
                }
                FileChangesMatcherDeserializationMode::Full(mut matcher) => {
                    if let Some(root) = &mut matcher.root {
                        *root = normalize_path(&*root).map_err(D::Error::custom)?;
                    }
                    matcher
                }
            },
        )
    }
}

#[derive(Debug, Clone, Hash, Serialize, Deserialize)]
pub struct CommandFailsOptions {
    program: String,
    #[serde(default)]
    arguments: Vec<String>,
    #[serde(default)]
    environment: BTreeMap<String, String>,
    #[serde(default)]
    verbose: bool,
    #[serde(deserialize_with = "deserialize_cwd", default)]
    cwd: Option<PathBuf>,
}

impl CommandFailsOptions {
    pub fn program(&self) -> &str {
        &self.program
    }

    pub fn arguments(&self) -> &[String] {
        &self.arguments
    }

    pub fn environment(&self) -> &BTreeMap<String, String> {
        &self.environment
    }

    pub fn verbose(&self) -> bool {
        self.verbose
    }

    pub fn cwd(&self) -> Option<&Path> {
        self.cwd.as_deref()
    }
}

fn deserialize_cwd<'de, D>(deserializer: D) -> std::result::Result<Option<PathBuf>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<PathBuf>::deserialize(deserializer)?
        .map(normalize_path)
        .transpose()
        .map_err(D::Error::custom)
}

#[derive(Debug, Clone, Copy, Hash, Display, Serialize, EnumIter)]
pub enum TimeUnit {
    Milliseconds,
    Seconds,
    Minutes,
    Hours,
    Days,
}

unit_enum_from_str!(TimeUnit);
unit_enum_deserialize!(TimeUnit);

#[derive(Debug, Clone, Hash, Serialize, Deserialize)]
pub struct TtlOptions {
    unit: TimeUnit,
    amount: NonZeroUsize,
}

impl TtlOptions {
    pub fn unit(&self) -> TimeUnit {
        self.unit
    }

    pub fn amount(&self) -> usize {
        self.amount.get()
    }
}

#[derive(Debug, Clone, Hash, Serialize, Deserialize)]
pub struct EnvChangesOptions {
    variables: BTreeSet<String>,
}

impl EnvChangesOptions {
    pub fn variables(&self) -> &BTreeSet<String> {
        &self.variables
    }
}
