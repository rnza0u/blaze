use std::{collections::HashMap, fmt::Display, fs::File, path::PathBuf, str::FromStr};

use semver::Version;
use serde::{Deserialize, Serialize};
use strum_macros::EnumIter;

use crate::error::{Error, Result};

const BIN_ROOT_VAR: &str = "BIN_ROOT";
const METADATA_FILENAME: &str = "metadata.json";
const CONTENT_FILENAME: &str = "blaze.tar.gz";

pub struct VersionEntry {
    pub version: Version,
    pub root: PathBuf,
}

pub fn list_versions() -> Result<Vec<VersionEntry>> {
    let root_dir = std::env::var(BIN_ROOT_VAR)
        .map_err(|_| Error::Configuration("bin root is not provided"))?;
    let mut v = std::fs::read_dir(&root_dir)
        .map_err(Error::any)?
        .map(|entry| {
            let entry = entry.map_err(Error::any)?;
            if !entry.file_type().map_err(Error::any)?.is_dir() {
                return Err(Error::InvalidState("version entry is not a directory"));
            }
            let version = Version::parse(entry.file_name().to_str().unwrap())
                .map_err(|_| Error::InvalidState("version directory has invalid name"))?;
            Ok(version)
        })
        .collect::<Result<Vec<_>>>()?;
    v.sort_by(|a, b| b.cmp(a));
    Ok(v.into_iter()
        .map(|version: Version| VersionEntry {
            root: PathBuf::from(&root_dir).join(version.to_string()),
            version,
        })
        .collect())
}

#[derive(EnumIter, Hash, PartialEq, Eq, Clone, Copy)]
pub enum Platform {
    X8664LinuxGnu,
    X8664LinuxMusl,
    X8664Osx,
    Aarch64Osx,
    X8664Windows,
}

const X86_64_LINUX_GNU: &str = "x86_64-linux-gnu";
const X86_64_LINUX_MUSL: &str = "x86_64-linux-musl";
const X86_64_OSX: &str = "x86_64-osx";
const AARCH64_OSX: &str = "aarch64-osx";
const X86_64_WINDOWS: &str = "x86_64-windows";

impl Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::X8664LinuxGnu => X86_64_LINUX_GNU,
            Self::X8664LinuxMusl => X86_64_LINUX_MUSL,
            Self::X8664Osx => X86_64_OSX,
            Self::Aarch64Osx => AARCH64_OSX,
            Self::X8664Windows => X86_64_WINDOWS,
        })
    }
}

impl FromStr for Platform {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        Ok(match s {
            X86_64_LINUX_GNU => Self::X8664LinuxGnu,
            X86_64_LINUX_MUSL => Self::X8664LinuxMusl,
            X86_64_OSX => Self::X8664Osx,
            AARCH64_OSX => Self::Aarch64Osx,
            X86_64_WINDOWS => Self::X8664Windows,
            _ => return Err(Error::BadParams),
        })
    }
}

impl Serialize for Platform {
    fn serialize<S>(&self, serializer: S) -> std::prelude::v1::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[derive(Serialize, Deserialize)]
pub struct Build {
    #[serde(flatten)]
    metadata: BuildMetadata,
    version: Version,
}

#[derive(Serialize, Deserialize)]
struct BuildMetadata {
    checksum: String,
    size: usize,
}

pub fn list_builds(version_identifier: &VersionIdentifier) -> Result<HashMap<Platform, Build>> {
    let version_entry = version_entry_from_version_identifier(version_identifier)?;
    std::fs::read_dir(&version_entry.root)
        .map_err(Error::any)?
        .map(|entry| entry.map_err(Error::any))
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .filter_map(|entry| {
            Platform::from_str(entry.file_name().to_str().unwrap())
                .ok()
                .map(|platform| (platform, entry.path()))
        })
        .map(|(platform, root)| {
            Ok((
                platform,
                Build {
                    metadata: serde_json::from_reader(
                        std::fs::File::open(root.join(METADATA_FILENAME)).map_err(Error::any)?,
                    )
                    .map_err(Error::any)?,
                    version: version_entry.version.clone(),
                },
            ))
        })
        .collect::<Result<_>>()
}

pub enum VersionIdentifier {
    Latest,
    Provided(Version),
}

impl FromStr for VersionIdentifier {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        Ok(match s {
            "latest" => Self::Latest,
            version => Self::Provided(Version::parse(version).map_err(|_| Error::BadParams)?),
        })
    }
}

pub struct PackageDownload {
    pub file: File,
    pub version: Version,
}

pub fn get_package_download(
    version_identifier: &VersionIdentifier,
    platform: Platform,
) -> Result<PackageDownload> {
    let version_entry = version_entry_from_version_identifier(version_identifier)?;
    match File::open(
        version_entry
            .root
            .join(platform.to_string())
            .join(CONTENT_FILENAME),
    ) {
        Ok(file) => Ok(PackageDownload {
            file,
            version: version_entry.version,
        }),
        Err(not_found) if not_found.kind() == std::io::ErrorKind::NotFound => {
            Err(Error::VersionNotFound)
        }
        Err(err) => Err(Error::any(err)),
    }
}

fn version_entry_from_version_identifier(identifier: &VersionIdentifier) -> Result<VersionEntry> {
    let versions_entries = list_versions()?;
    Ok(match identifier {
        VersionIdentifier::Latest => versions_entries
            .into_iter()
            .next()
            .ok_or_else(|| Error::VersionNotFound)?,
        VersionIdentifier::Provided(provided_version) => versions_entries
            .into_iter()
            .find(|entry| entry.version == *provided_version)
            .ok_or_else(|| Error::VersionNotFound)?,
    })
}
