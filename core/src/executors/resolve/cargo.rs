use std::{
    fmt::{Display, Pointer},
    str::FromStr,
};

use anyhow::{anyhow, bail, Context};
use blaze_common::{
    error::{Error, Result},
    executor::CargoOptions,
    logger::Logger,
    workspace::Workspace,
};
use flate2::read::GzDecoder;
use reqwest::{
    blocking::{Client, ClientBuilder},
    header::{HeaderMap, HeaderName, HeaderValue},
};
use semver::{Version, VersionReq};
use serde::Deserialize;
use url::Url;

use crate::system::random::random_string;

use super::resolver::{ExecutorResolution, ExecutorResolver, ExecutorUpdate};

const CRATES_ROOT: &str = ".blaze/cargo";
const DEFAULT_REGISTRY: &str = "crates.io";

pub struct CargoResolver<'a> {
    context: CargoResolverContext<'a>,
    options: CargoOptions,
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
    pub fn new(options: CargoOptions, context: CargoResolverContext<'a>) -> Self {
        Self { options, context }
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
        let registry = url.host_str().unwrap_or(DEFAULT_REGISTRY);
        let port = url.port();
        let crate_name = url.path();
        let required_version = self
            .options
            .version()
            .map(RequiredVersion::from_str)
            .transpose()?;
        let logger = self.context.logger;

        let mut http_client_builder = ClientBuilder::new();
        if self.options.insecure() {
            logger.warn(format!("{url} will be resolved with the `insecure` flag"));
            http_client_builder = http_client_builder
                .danger_accept_invalid_certs(true)
                .danger_accept_invalid_hostnames(true);
        }

        let protocol = if self.options.no_ssl(){
            if !self.options.insecure(){
                bail!("`insecure` must be set to true when using `noSsl`");
            }
            "http"
        } else {
            "https"
        };

        let http_client = http_client_builder.build()?;
        let mut registry_url = Url::parse(&format!("{protocol}://{registry}"))?;
        registry_url.set_port(port).map_err(|_| anyhow!("invalid port was provided"))?;
        

        let get_request = |url: &Url| {
            let mut request = http_client.get(url.clone());

            if let Some(token) = self.options.token() {
                request = request.bearer_auth(token);
            }

            request
        };

        let mut versions_url = registry_url.clone();
        versions_url.set_path(&format!("/api/v1/crates/{crate_name}/versions"));

        logger.debug(format!(
            "reading versions for {crate_name} at {versions_url}"
        ));

        let mut versions = get_request(&versions_url)
            .send()
            .with_context(|| format!("could not send request to {versions_url}"))?
            .json::<VersionsResponse>()
            .with_context(|| format!("could not deserialize response from {versions_url}"))?
            .versions
            .into_iter()
            .map(|v| v.num)
            .collect::<Vec<_>>();

        if versions.is_empty() {
            bail!("no versions were found for crate {crate_name}")
        }

        logger.debug(format!(
            "crate {crate_name} has {} version(s)",
            versions.len()
        ));

        versions.sort();

        let version = match required_version {
            Some(required_version) => versions
                .into_iter()
                .rev()
                .find(|version| required_version.matches(version))
                .ok_or_else(|| {
                    anyhow!("no versions were found that satisfies {required_version}")
                })?,
            None => versions.pop().unwrap(),
        };

        let crate_root = if self.context.save_in_workspace {
            self.context
                .workspace
                .root()
                .join(CRATES_ROOT)
                .join(random_string(12))
        } else {
            std::env::temp_dir().join(random_string(12))
        };

        logger.debug(format!(
            "{crate_name} in v{version} will be downloaded to {}",
            crate_root.display()
        ));

        std::fs::create_dir_all(&crate_root).with_context(|| {
            format!("failed to create crate directory {}", crate_root.display())
        })?;

        let mut download_url = registry_url.clone();
        download_url.set_path(&format!("/api/v1/crates/{crate_name}/{version}/download"));

        let download_request = get_request(&download_url)
            .send()
            .with_context(|| format!("could not send request to {download_url}"))?;

        let mut archive = tar::Archive::new(GzDecoder::new(download_request));
        archive.set_overwrite(true);
        archive
            .unpack(&crate_root)
            .with_context(|| format!("failed to unpack crate {crate_name}"))?;

        todo!()
    }

    fn update(&self, url: &Url, state: &blaze_common::value::Value) -> Result<ExecutorUpdate> {
        todo!()
    }
}
