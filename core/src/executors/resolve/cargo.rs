use std::{
    fmt::{Display, Pointer},
    path::{Path, PathBuf},
    str::FromStr,
};

use anyhow::{anyhow, bail, Context};
use blaze_common::{
    error::{Error, Result},
    executor::CargoOptions,
    logger::Logger,
    value::{to_value, Value},
    workspace::Workspace,
};
use flate2::read::GzDecoder;
use reqwest::blocking::{Client, ClientBuilder, Response};
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::system::random::random_string;

use super::{
    loader::ExecutorLoadStrategy,
    resolver::{ExecutorResolution, ExecutorResolver, ExecutorUpdate},
};

const CRATES_ROOT: &str = ".blaze/cargo";
const DEFAULT_REGISTRY: &str = "crates.io";

pub struct CargoResolver<'a> {
    context: CargoResolverContext<'a>,
    options: CargoOptions,
    http_client: Client,
}

#[derive(Serialize, Deserialize)]
struct State {
    crate_root: PathBuf,
    version: Version,
}

pub struct CargoResolverContext<'a> {
    workspace: &'a Workspace,
    save_in_workspace: bool,
    logger: &'a Logger,
}

#[derive(Deserialize)]
struct VersionsResponse {
    versions: Vec<VersionResponse>,
}

#[derive(Deserialize)]
struct VersionResponse {
    num: Version,
}

impl<'a> CargoResolver<'a> {
    pub fn try_new(options: CargoOptions, context: CargoResolverContext<'a>) -> Result<Self> {
        let mut http_client_builder = ClientBuilder::new();
        if options.insecure() {
            context.logger.warn(
                "HTTP client will be used with the `insecure` flag when fetching cargo crates.",
            );
            http_client_builder = http_client_builder
                .danger_accept_invalid_certs(true)
                .danger_accept_invalid_hostnames(true);
        }

        Ok(Self {
            options,
            context,
            http_client: http_client_builder.build()?,
        })
    }

    fn get(&self, url: &Url) -> Result<Response> {
        let mut request = self.http_client.get(url.clone());
        if let Some(token) = self.options.token() {
            request = request.bearer_auth(token);
        }
        request
            .send()
            .with_context(|| format!("could not send request to {url}"))
    }

    fn registry_url(&self, registry_host: &str, port: Option<u16>) -> Result<Url> {
        let protocol = if self.options.no_ssl() {
            if !self.options.insecure() {
                bail!("`insecure` must be set to true when using `noSsl`");
            }
            "http"
        } else {
            "https"
        };

        let mut registry_url = Url::parse(&format!("{protocol}://{registry_host}"))?;
        registry_url
            .set_port(port)
            .map_err(|_| anyhow!("invalid port was provided"))?;

        Ok(registry_url)
    }

    fn get_versions(&self, registry_url: &Url, crate_name: &str) -> Result<Vec<Version>> {
        let mut versions_url = registry_url.clone();
        versions_url.set_path(&format!("/api/v1/crates/{crate_name}/versions"));

        let mut versions = self
            .get(&versions_url)?
            .json::<VersionsResponse>()
            .with_context(|| format!("could not deserialize response from {versions_url}"))?
            .versions
            .into_iter()
            .map(|v| v.num)
            .collect::<Vec<_>>();

        versions.sort();

        Ok(versions)
    }

    fn create_crate_root_path(&self) -> PathBuf {
        let crate_root_path = || random_string(12);

        if self.context.save_in_workspace {
            self.context
                .workspace
                .root()
                .join(CRATES_ROOT)
                .join(crate_root_path())
        } else {
            std::env::temp_dir().join(crate_root_path())
        }
    }

    fn download_crate(
        &self,
        registry: &Url,
        crate_name: &str,
        version: &Version,
        destination: &Path,
    ) -> Result<PathBuf> {
        if destination.try_exists().with_context(|| {
            format!(
                "could not check if crate directory {} exists",
                destination.display()
            )
        })? {
            std::fs::remove_dir_all(destination).with_context(|| {
                format!("could not remove crate directory {}", destination.display())
            })?;
        }

        std::fs::create_dir_all(destination).with_context(|| {
            format!("failed to create crate directory {}", destination.display())
        })?;

        let mut download_url = registry.clone();
        download_url.set_path(&format!("/api/v1/crates/{crate_name}/{version}/download"));

        let download = self.get(&download_url)?;

        let mut archive = tar::Archive::new(GzDecoder::new(download));
        archive.set_overwrite(true);
        archive
            .unpack(destination)
            .with_context(|| format!("failed to unpack crate {crate_name}"))?;

        Ok(destination.join(format!("{crate_name}-{version}")))
    }

    fn find_matching_version(&self, mut available_versions: Vec<Version>) -> Result<Version> {
        if available_versions.is_empty() {
            bail!("no versions were found")
        }

        let required_version = self
            .options
            .version()
            .map(RequiredVersion::from_str)
            .transpose()?;

        let version = match required_version {
            Some(required_version) => available_versions
                .into_iter()
                .rev()
                .find(|version| required_version.matches(version))
                .ok_or_else(|| {
                    anyhow!("no versions were found that satisfies {required_version}")
                })?,
            None => available_versions.pop().unwrap(),
        };

        Ok(version)
    }
}

pub enum RequiredVersion {
    Fixed(Version),
    Req(VersionReq),
}

impl Display for RequiredVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Fixed(version) => version.fmt(f),
            Self::Req(req) => req.fmt(f),
        }
    }
}

impl RequiredVersion {
    pub fn is_fixed(&self) -> bool {
        matches!(self, Self::Fixed(_))
    }

    pub fn matches(&self, version: &Version) -> bool {
        match self {
            Self::Fixed(fixed_version) => version == fixed_version,
            Self::Req(requirement) => requirement.matches(version),
        }
    }
}

impl FromStr for RequiredVersion {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(Version::parse(s)
            .map(RequiredVersion::Fixed)
            .or_else(|_| VersionReq::parse(s).map(RequiredVersion::Req))?)
    }
}

impl ExecutorResolver for CargoResolver<'_> {
    fn resolve(&self, url: &Url) -> Result<ExecutorResolution> {
        let logger = self.context.logger;

        let registry_url =
            self.registry_url(url.host_str().unwrap_or(DEFAULT_REGISTRY), url.port())?;

        let crate_name = url.path();

        logger.debug(format!(
            "reading versions for {crate_name} at {registry_url}"
        ));

        let versions = self.get_versions(&registry_url, crate_name)?;

        logger.debug(format!(
            "found {} version(s) for {}",
            versions.len(),
            crate_name
        ));

        let version = self.find_matching_version(versions)?;

        let crate_root = self.create_crate_root_path();

        logger.debug(format!(
            "{crate_name} in version {version} will be downloaded to {}",
            crate_root.display()
        ));

        let src = self.download_crate(&registry_url, crate_name, &version, &crate_root)?;

        Ok(ExecutorResolution {
            load_strategy: ExecutorLoadStrategy::RustLocal,
            state: to_value(State {
                crate_root: src.to_owned(),
                version,
            })?,
            src,
        })
    }

    fn update(&self, url: &Url, state: &Value) -> Result<ExecutorUpdate> {
        let no_update = || ExecutorUpdate {
            load_strategy: ExecutorLoadStrategy::RustLocal,
            new_state: None,
            update: None,
        };
        let logger = self.context.logger;

        if !self.options.pull() {
            return Ok(no_update());
        }

        let required_version = self
            .options
            .version()
            .map(RequiredVersion::from_str)
            .transpose()?;

        if required_version.is_some_and(|r| r.is_fixed()) {
            return Ok(no_update());
        }

        let mut state = State::deserialize(state)?;

        let registry_url =
            self.registry_url(url.host_str().unwrap_or(DEFAULT_REGISTRY), url.port())?;
        let crate_name = url.path();

        let versions = self.get_versions(&registry_url, crate_name)?;

        let new_version = self.find_matching_version(versions)?;

        if new_version == state.version {
            return Ok(no_update());
        }

        logger.debug(format!(
            "{crate_name} in version {new_version} will be downloaded to {}",
            state.crate_root.display()
        ));

        let src =
            self.download_crate(&registry_url, crate_name, &new_version, &state.crate_root)?;
        state.version = new_version;

        Ok(ExecutorUpdate {
            load_strategy: ExecutorLoadStrategy::RustLocal,
            new_state: Some(to_value(state)?),
            update: Some(src),
        })
    }
}
