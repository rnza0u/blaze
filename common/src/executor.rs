use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Display,
    path::{Path, PathBuf},
    str::FromStr,
};

use anyhow::bail;
use base64::Engine;
use hash_value::{to_value, Value};
use serde::{de::Error as DeError, ser::Error as SerError, Deserialize, Serialize};
use strum_macros::{Display, EnumIter};
use url::Url;

use crate::{cache::FileChangesMatcher, error::Error, unit_enum_deserialize, unit_enum_from_str};

#[derive(Hash, PartialEq, Eq, Serialize, EnumIter, Display, Debug, Clone, Copy)]
pub enum ExecutorKind {
    Rust,
    Node,
}

unit_enum_from_str!(ExecutorKind);
unit_enum_deserialize!(ExecutorKind);

#[derive(Serialize, Debug, Hash, PartialEq, Eq, Clone)]
#[serde(untagged)]
pub enum ExecutorReference {
    Standard {
        url: Url,
    },
    Custom {
        url: Url,
        #[serde(flatten)]
        location: Location,
    },
}

impl Display for ExecutorReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Standard { url } => url.fmt(f),
            Self::Custom { url, location } => {
                let fmt_git_location = |url: &Url, options: &GitOptions| {
                    let mut out = url.to_string();
                    if let Some(path) = options.path() {
                        out.push_str(&format!("[/{}]", path.display()));
                    }
                    if let Some(checkout) = options.checkout() {
                        out.push_str(&format!("@{}", checkout));
                    }
                    out
                };

                match location {
                    Location::Git {
                        options: git_options,
                    }
                    | Location::GitOverHttp { git_options, .. }
                    | Location::GitOverSsh { git_options, .. } => {
                        write!(f, "{}", fmt_git_location(url, git_options))
                    }
                    _ => url.fmt(f),
                }
            }
        }
    }
}

const URL_KEY: &str = "url";
const FORMAT_KEY: &str = "format";
const AUTHENTICATION_KEY: &str = "authentication";

impl<'de> Deserialize<'de> for ExecutorReference {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let root = Value::deserialize(deserializer)?;
        let url = root
            .at(URL_KEY)
            .and_then(Value::as_str)
            .or_else(|| root.as_str())
            .map(|s| Url::parse(s).map_err(D::Error::custom))
            .transpose()?
            .ok_or_else(|| D::Error::missing_field(URL_KEY))?;
        let is_single_url = matches!(root, Value::String(_));

        match url.scheme() {
            "std" => Ok(Self::Standard { url }),
            custom_executor_scheme => Ok(Self::Custom {
                location: match custom_executor_scheme {
                    "file" => Location::LocalFileSystem {
                        options: if is_single_url {
                            FileSystemOptions::default()
                        } else {
                            FileSystemOptions::deserialize(&root).map_err(D::Error::custom)?
                        },
                    },
                    "http" | "https" => {
                        let format = root
                            .at(FORMAT_KEY)
                            .map(|f| HttpFormatIdentifier::deserialize(f).map_err(D::Error::custom))
                            .transpose()?
                            .ok_or_else(|| D::Error::missing_field(FORMAT_KEY))?;

                        let transport =
                            HttpTransport::deserialize(&root).map_err(D::Error::custom)?;

                        match format {
                            HttpFormatIdentifier::Git => Location::GitOverHttp {
                                transport,
                                git_options: GitOptions::deserialize(&root)
                                    .map_err(D::Error::custom)?,
                                authentication: root
                                    .at(AUTHENTICATION_KEY)
                                    .map(|auth| {
                                        GitPlainAuthentication::deserialize(auth)
                                            .map_err(D::Error::custom)
                                    })
                                    .transpose()?,
                            },
                            HttpFormatIdentifier::Tarball => Location::TarballOverHttp {
                                transport,
                                tarball_options: TarballOptions::deserialize(&root)
                                    .map_err(D::Error::custom)?,
                                authentication: root
                                    .at(AUTHENTICATION_KEY)
                                    .map(|auth| {
                                        HttpAuthentication::deserialize(auth)
                                            .map_err(D::Error::custom)
                                    })
                                    .transpose()?,
                            },
                        }
                    }
                    "ssh" => Location::GitOverSsh {
                        transport: SshTransport::deserialize(&root).map_err(D::Error::custom)?,
                        git_options: GitOptions::deserialize(&root).map_err(D::Error::custom)?,
                        authentication: root
                            .at(AUTHENTICATION_KEY)
                            .map(|auth| {
                                SshAuthentication::deserialize(auth).map_err(D::Error::custom)
                            })
                            .transpose()?,
                    },
                    "git" => Location::Git {
                        options: GitOptions::deserialize(&root).map_err(D::Error::custom)?,
                    },
                    "npm" => Location::Npm {
                        options: NpmOptions::deserialize(&root).map_err(D::Error::custom)?,
                    },
                    "cargo" => Location::Cargo {
                        options: CargoOptions::deserialize(&root).map_err(D::Error::custom)?,
                    },
                    invalid_scheme => {
                        return Err(serde::de::Error::custom(format!(
                            "invalid url scheme \"{invalid_scheme}\""
                        )))
                    }
                },
                url,
            }),
        }
    }
}

#[derive(Hash, Debug, PartialEq, Eq, Clone)]
pub enum Location {
    LocalFileSystem {
        options: FileSystemOptions,
    },

    TarballOverHttp {
        transport: HttpTransport,
        tarball_options: TarballOptions,
        authentication: Option<HttpAuthentication>,
    },

    GitOverHttp {
        transport: HttpTransport,
        git_options: GitOptions,
        authentication: Option<GitPlainAuthentication>,
    },

    GitOverSsh {
        transport: SshTransport,
        git_options: GitOptions,
        authentication: Option<SshAuthentication>,
    },
    Git {
        options: GitOptions,
    },
    Cargo {
        options: CargoOptions,
    },
    Npm {
        options: NpmOptions,
    },
}

impl Serialize for Location {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::LocalFileSystem { options } => options.serialize(serializer),
            Self::GitOverHttp {
                transport,
                git_options,
                authentication,
            } => {
                let mut value = to_value(transport).map_err(S::Error::custom)?;
                value.overwrite(to_value(git_options).map_err(S::Error::custom)?);
                value.overwrite(Value::object([(
                    FORMAT_KEY,
                    Value::string(HttpFormatIdentifier::Git.to_string()),
                )]));
                if let Some(authentication) = authentication {
                    value.overwrite(Value::object([(
                        AUTHENTICATION_KEY,
                        to_value(authentication).map_err(S::Error::custom)?,
                    )]));
                }
                value.serialize(serializer)
            }
            Self::GitOverSsh {
                transport,
                git_options,
                authentication,
            } => {
                let mut value = to_value(transport).map_err(S::Error::custom)?;
                value.overwrite(to_value(git_options).map_err(S::Error::custom)?);
                if let Some(authentication) = authentication {
                    value.overwrite(to_value(authentication).map_err(S::Error::custom)?);
                }
                value.serialize(serializer)
            }
            Self::Npm { options } => options.serialize(serializer),
            Self::Cargo { options } => options.serialize(serializer),
            _ => todo!(),
        }
    }
}

#[derive(Default, EnumIter, Display, Serialize, Hash, PartialEq, Eq, Debug, Clone, Copy)]
pub enum RebuildStrategy {
    Always,
    #[default]
    OnChanges,
}

unit_enum_from_str!(RebuildStrategy);
unit_enum_deserialize!(RebuildStrategy);

#[derive(Deserialize, Serialize, Hash, Debug, PartialEq, Eq, Clone, Default)]
pub struct FileSystemOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    kind: Option<ExecutorKind>,
    #[serde(default)]
    rebuild: RebuildStrategy,
    #[serde(skip_serializing_if = "Option::is_none")]
    watch: Option<BTreeSet<FileChangesMatcher>>,
}

impl FileSystemOptions {
    pub fn kind(&self) -> Option<ExecutorKind> {
        self.kind
    }

    pub fn rebuild(&self) -> RebuildStrategy {
        self.rebuild
    }

    pub fn watch(&self) -> Option<&BTreeSet<FileChangesMatcher>> {
        self.watch.as_ref()
    }
}

#[derive(Serialize, Deserialize, Hash, Debug, PartialEq, Eq, Clone)]
pub struct HttpTransport {
    #[serde(default)]
    insecure: bool,
    #[serde(default)]
    headers: BTreeMap<String, String>,
}

impl HttpTransport {
    pub fn insecure(&self) -> bool {
        self.insecure
    }

    pub fn headers(&self) -> &BTreeMap<String, String> {
        &self.headers
    }
}

const HTTP_AUTH_MODE_KEY: &str = "mode";

#[derive(Serialize, EnumIter, Display, Hash, PartialEq, Eq, Clone)]
pub enum HttpAuthenticationMode {
    Basic,
    Bearer,
}

unit_enum_from_str!(HttpAuthenticationMode);
unit_enum_deserialize!(HttpAuthenticationMode);

#[derive(Serialize, Deserialize, Hash, Debug, PartialEq, Eq, Clone)]
pub struct HttpBasicAuthentication {
    username: String,
    password: String,
}

impl HttpBasicAuthentication {
    pub fn username(&self) -> &str {
        &self.username
    }

    pub fn password(&self) -> &str {
        &self.password
    }
}

#[derive(Serialize, Deserialize, Hash, Debug, PartialEq, Eq, Clone)]
pub struct HttpDigestAuthentication {
    username: String,
    password: String,
}

impl HttpDigestAuthentication {
    pub fn username(&self) -> &str {
        &self.username
    }

    pub fn password(&self) -> &str {
        &self.password
    }
}

#[derive(Serialize, Deserialize, Hash, Debug, PartialEq, Eq, Clone)]
pub struct HttpBearerAuthentication {
    token: String,
}

impl HttpBearerAuthentication {
    pub fn token(&self) -> &str {
        &self.token
    }
}

#[derive(Hash, Debug, PartialEq, Eq, Clone)]
pub enum HttpAuthentication {
    Basic(HttpBasicAuthentication),
    Bearer(HttpBearerAuthentication),
}

impl<'de> Deserialize<'de> for HttpAuthentication {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let root = Value::deserialize(deserializer)?;
        let mode = root
            .at(HTTP_AUTH_MODE_KEY)
            .map(|m| HttpAuthenticationMode::deserialize(m).map_err(D::Error::custom))
            .transpose()?
            .ok_or_else(|| D::Error::missing_field(HTTP_AUTH_MODE_KEY))?;
        Ok(match mode {
            HttpAuthenticationMode::Basic => HttpAuthentication::Basic(
                HttpBasicAuthentication::deserialize(root).map_err(D::Error::custom)?,
            ),
            HttpAuthenticationMode::Bearer => HttpAuthentication::Bearer(
                HttpBearerAuthentication::deserialize(root).map_err(D::Error::custom)?,
            ),
        })
    }
}

impl Serialize for HttpAuthentication {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut value = Value::object([(
            HTTP_AUTH_MODE_KEY,
            to_value(match self {
                HttpAuthentication::Basic(_) => HttpAuthenticationMode::Basic,
                HttpAuthentication::Bearer(_) => HttpAuthenticationMode::Bearer,
            })
            .map_err(S::Error::custom)?,
        )]);

        value.overwrite(
            match self {
                HttpAuthentication::Basic(basic) => to_value(basic),
                HttpAuthentication::Bearer(bearer) => to_value(bearer),
            }
            .map_err(S::Error::custom)?,
        );

        value.serialize(serializer)
    }
}

#[derive(Serialize, EnumIter, Display)]
pub enum HttpFormatIdentifier {
    Git,
    Tarball,
}

#[derive(Serialize, Hash, Debug, PartialEq, Eq, Clone)]
#[serde(untagged)]
pub enum HttpResource {
    Git {
        #[serde(flatten)]
        options: GitOptions,
    },
    Tarball {
        #[serde(flatten)]
        options: TarballOptions,
    },
}

#[derive(Serialize, Deserialize, Hash, Debug, PartialEq, Eq, Clone)]
#[serde(untagged)]
pub enum GitCheckout {
    Branch { branch: String },
    Tag { tag: String },
    Revision { rev: String },
}

impl Display for GitCheckout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Branch { branch } => branch.as_str(),
            Self::Revision { rev } => rev.as_str(),
            Self::Tag { tag } => tag.as_str(),
        })
    }
}

unit_enum_from_str!(HttpFormatIdentifier);
unit_enum_deserialize!(HttpFormatIdentifier);

#[derive(Deserialize, Serialize, Hash, Debug, PartialEq, Eq, Clone)]
pub struct GitOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    kind: Option<ExecutorKind>,
    #[serde(default)]
    pull: bool,
    #[serde(skip_serializing_if = "Option::is_none", flatten)]
    checkout: Option<GitCheckout>,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<PathBuf>,
}

impl GitOptions {
    pub fn kind(&self) -> Option<ExecutorKind> {
        self.kind
    }

    pub fn pull(&self) -> bool {
        self.pull
    }

    pub fn checkout(&self) -> Option<&GitCheckout> {
        self.checkout.as_ref()
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }
}

#[derive(Deserialize, Serialize, Hash, Debug, PartialEq, Eq, Clone)]
pub struct GitPlainAuthentication {
    username: String,
    password: String,
}

impl GitPlainAuthentication {
    pub fn username(&self) -> &str {
        &self.username
    }

    pub fn password(&self) -> &str {
        &self.password
    }
}

#[derive(Serialize, EnumIter, Display, Hash, Debug, PartialEq, Eq, Copy, Clone)]
pub enum Compression {
    Deflate,
    Zlib,
    Gzip,
}

unit_enum_from_str!(Compression);
unit_enum_deserialize!(Compression);

#[derive(Serialize, Deserialize, Hash, Debug, PartialEq, Eq, Clone)]
pub struct TarballOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    kind: Option<ExecutorKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    compression: Option<Compression>,
}

impl TarballOptions {
    pub fn kind(&self) -> Option<ExecutorKind> {
        self.kind
    }

    pub fn compression(&self) -> Option<Compression> {
        self.compression
    }
}

#[derive(Serialize, EnumIter, Display, Hash, Debug, PartialEq, Eq, Copy, Clone)]
pub enum SshFingerprintAlgorithm {
    Md5,
    Sha1,
    Sha256,
}

unit_enum_from_str!(SshFingerprintAlgorithm);
unit_enum_deserialize!(SshFingerprintAlgorithm);

#[derive(Hash, Debug, PartialEq, Eq, Clone)]
pub enum SshFingerprint {
    Md5([u8; 16]),
    Sha1([u8; 20]),
    Sha256([u8; 32]),
}

macro_rules! parse_fingerprint {
    ($value:expr, $len:literal) => {{
        let decoded = base64::prelude::BASE64_STANDARD_NO_PAD
            .decode($value)
            .map_err(|err| {
                anyhow::anyhow!(
                    "failed to decode ssh fingerprint content {} ({})",
                    $value,
                    err
                )
            })?;
        let length = decoded.len();
        if length != $len {
            bail!(
                "fingerprint has invalid length (expected={}, actual={})",
                $len,
                length
            )
        }
        let mut bytes = [0_u8; $len];
        bytes.copy_from_slice(&decoded);
        Ok::<_, $crate::error::Error>(bytes)
    }};
}

const MD5: &str = "MD5";
const SHA1: &str = "SHA1";
const SHA256: &str = "SHA256";

impl FromStr for SshFingerprint {
    type Err = Error;

    fn from_str(s: &str) -> crate::error::Result<Self> {
        Ok(match s.split_once(':'){
            Some((algorithm, value)) => {
                match algorithm {
                    MD5 => Self::Md5(parse_fingerprint!(value, 16)?),
                    SHA1 => Self::Sha1(parse_fingerprint!(value, 20)?),
                    SHA256 => Self::Sha256(parse_fingerprint!(value, 32)?),
                    _ => bail!("unknown ssh fingerprint algorithm \"{algorithm}\". valid algorithms are MD5, SHA1 and SHA256")
                }
            },
            _ => bail!("bad ssh fingerprint. format must be <ALGORITHM>:<base64 hash>.")
        })
    }
}

impl Display for SshFingerprint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (algorithm_id, slice) = match self {
            Self::Md5(md5) => (MD5, md5.as_slice()),
            Self::Sha1(sha1) => (SHA1, sha1.as_slice()),
            Self::Sha256(sha256) => (SHA256, sha256.as_slice()),
        };
        let mut base64 = String::new();
        base64::prelude::BASE64_STANDARD_NO_PAD.encode_string(slice, &mut base64);
        write!(f, "{algorithm_id}:{base64}")
    }
}

impl Serialize for SshFingerprint {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for SshFingerprint {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        SshFingerprint::from_str(&s).map_err(D::Error::custom)
    }
}

#[derive(Deserialize, Serialize, Hash, Debug, PartialEq, Eq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SshTransport {
    #[serde(skip_serializing_if = "Option::is_none")]
    fingerprints: Option<Vec<SshFingerprint>>,
    #[serde(default)]
    insecure: bool,
}

impl SshTransport {
    pub fn fingerprints(&self) -> Option<&[SshFingerprint]> {
        self.fingerprints.as_deref()
    }

    pub fn insecure(&self) -> bool {
        self.insecure
    }
}

#[derive(Deserialize, Serialize, Hash, Debug, PartialEq, Eq, Clone)]
#[serde(untagged)]
pub enum SshResource {
    Git {
        #[serde(flatten)]
        options: GitOptions,
    },
}

#[derive(Deserialize, Serialize, Hash, Debug, PartialEq, Eq, Clone)]
#[serde(untagged)]
pub enum SshAuthentication {
    Password {
        #[serde(skip_serializing_if = "Option::is_none")]
        username: Option<String>,
        password: String,
    },
    PrivateKeyFile {
        #[serde(skip_serializing_if = "Option::is_none")]
        username: Option<String>,
        key: PathBuf,
        #[serde(skip_serializing_if = "Option::is_none")]
        passphrase: Option<String>,
    },
}

#[derive(Deserialize, Serialize, Hash, Debug, PartialEq, Eq, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CargoOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    version: Option<String>,
    #[serde(default)]
    no_ssl: bool,
    #[serde(default)]
    insecure: bool,
    #[serde(default)]
    pull: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    token: Option<String>,
}

impl CargoOptions {
    pub fn pull(&self) -> bool {
        self.pull
    }

    pub fn version(&self) -> Option<&str> {
        self.version.as_deref()
    }

    pub fn insecure(&self) -> bool {
        self.insecure
    }

    pub fn no_ssl(&self) -> bool {
        self.no_ssl
    }

    pub fn token(&self) -> Option<&str> {
        self.token.as_deref()
    }
}

#[derive(Deserialize, Serialize, Hash, Debug, PartialEq, Eq, Clone)]
pub struct NpmUsernamePasswordAuthentication {
    username: String,
    password: String,
}

impl NpmUsernamePasswordAuthentication {
    pub fn username(&self) -> &str {
        &self.username
    }

    pub fn password(&self) -> &str {
        &self.password
    }
}

#[derive(Deserialize, Serialize, Hash, Debug, PartialEq, Eq, Clone)]
pub struct NpmTokenAuthentication {
    token: String,
}

impl NpmTokenAuthentication {
    pub fn token(&self) -> &str {
        &self.token
    }
}

#[derive(Deserialize, Serialize, Hash, Debug, PartialEq, Eq, Clone)]
#[serde(untagged)]
pub enum NpmAuthentication {
    UsernamePassword(NpmUsernamePasswordAuthentication),
    Token(NpmTokenAuthentication),
}

#[derive(Deserialize, Serialize, Hash, Debug, PartialEq, Eq, Clone)]
pub struct NpmOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    version: Option<String>,
    #[serde(default)]
    pull: bool,
    #[serde(default)]
    insecure: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    authentication: Option<NpmAuthentication>,
}

impl NpmOptions {
    pub fn pull(&self) -> bool {
        self.pull
    }

    pub fn version(&self) -> Option<&str> {
        self.version.as_deref()
    }

    pub fn insecure(&self) -> bool {
        self.insecure
    }

    pub fn authentication(&self) -> Option<&NpmAuthentication> {
        self.authentication.as_ref()
    }
}
