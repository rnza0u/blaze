use std::{
    borrow::Cow,
    collections::HashMap,
    fmt::Display,
    fs::File,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context};
use base64::Engine;
use blaze_common::{
    error::Result,
    executor::{
        NpmAuthentication, NpmOptions, NpmTokenAuthentication, NpmUsernamePasswordAuthentication,
    },
    logger::Logger,
    util::path_to_string,
    value::{to_value, Value},
    workspace::Workspace,
};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::system::{npm::npm, process::ProcessOptions, random::random_string};

use super::{
    loader::{ExecutorLoadStrategy, ExecutorWithMetadata, LoaderContext},
    resolver::{ExecutorResolution, ExecutorResolver, ExecutorUpdate},
};

const PACKAGE_LOCATION: &str = ".blaze/npm";

#[derive(Serialize, Deserialize)]
struct State {
    package_root: PathBuf,
    load_metadata: Value,
    package_version: String,
}

#[derive(Deserialize)]
struct PackageJson {
    version: String,
}

impl PackageJson {
    fn from_package_root(root: &Path) -> Result<Self> {
        Ok(serde_json::from_reader(File::open(
            root.join("package.json"),
        )?)?)
    }
}

struct CustomNpmrcConfig<'a> {
    entries: HashMap<Cow<'a, str>, Cow<'a, str>>,
    environment: HashMap<String, String>,
}

impl<'a> CustomNpmrcConfig<'a> {
    pub fn from_options_and_registry(options: &'a NpmOptions, registry: Option<&'a str>) -> Self {
        let mut config = CustomNpmrcConfig::new();

        if let Some(registry) = &registry {
            config.registry(registry);
        }

        if options.insecure() {
            config.insecure();
        }

        if let Some(authentication) = options.authentication() {
            match authentication {
                NpmAuthentication::Token(token_authentication) => {
                    config.auth_token(token_authentication, registry);
                }
                NpmAuthentication::UsernamePassword(credentials) => {
                    config.auth_basic(credentials, registry);
                }
            }
        }

        config
    }

    fn new() -> Self {
        Self {
            entries: HashMap::new(),
            environment: HashMap::new(),
        }
    }

    fn registry(&mut self, registry: &'a str) {
        self.entries.insert("registry".into(), registry.into());
    }

    fn insecure(&mut self) {
        self.entries.insert("strict-ssl".into(), "false".into());
    }

    fn auth_token(
        &mut self,
        token_authentication: &'a NpmTokenAuthentication,
        registry: Option<&str>,
    ) {
        let key = if let Some(registry) = registry {
            Cow::Owned(format!("//{registry}/:_authToken"))
        } else {
            Cow::Borrowed("_authToken")
        };
        let var_name = self.create_auth_var(token_authentication.token());
        self.entries.insert(key, format!("${{{var_name}}}").into());
    }

    fn auth_basic(
        &mut self,
        credentials: &'a NpmUsernamePasswordAuthentication,
        registry: Option<&str>,
    ) {
        let key = if let Some(registry) = registry {
            Cow::Owned(format!("//{registry}/:_auth"))
        } else {
            Cow::Borrowed("_auth")
        };
        let base64_auth = base64::prelude::BASE64_STANDARD.encode(format!(
            "{}:{}",
            credentials.username(),
            credentials.password()
        ));
        let var_name = self.create_auth_var(&base64_auth);
        self.entries.insert(key, format!("{{{var_name}}}").into());
    }

    pub fn use_config<F, T>(mut self, f: F) -> Result<T>
    where
        F: FnOnce(HashMap<String, String>) -> T,
    {
        let path = std::env::temp_dir().join(format!(".npmrc_{}", random_string(12)));
        std::fs::write(
            &path,
            self.entries
                .iter()
                .map(|(key, value)| format!("{key} = {value}"))
                .collect::<Vec<_>>()
                .join("\n"),
        )
        .with_context(|| format!("could not write .npmrc file at {}", path.display()))?;

        self.environment
            .insert("NPM_CONFIG_USERCONFIG".into(), path_to_string(&path)?);

        let result = f(self.environment);

        std::fs::remove_file(&path)
            .with_context(|| format!("could not remove .npmrc file at {}", path.display()))?;

        Ok(result)
    }

    fn create_auth_var(&mut self, secret: &str) -> String {
        let var_name = random_string(16);
        self.environment.insert(var_name.clone(), secret.to_owned());
        var_name
    }
}

pub struct NpmResolver<'a> {
    options: NpmOptions,
    logger: &'a Logger,
    packages_root: PathBuf,
    workspace: &'a Workspace,
}

#[derive(Clone, Copy)]
pub struct NpmResolverContext<'a> {
    pub workspace: &'a Workspace,
    pub logger: &'a Logger,
    pub save_in_workspace: bool,
}

impl<'a> NpmResolver<'a> {
    pub fn new(options: NpmOptions, context: NpmResolverContext<'a>) -> Self {
        Self {
            options,
            logger: context.logger,
            workspace: context.workspace,
            packages_root: if context.save_in_workspace {
                context.workspace.root().join(PACKAGE_LOCATION)
            } else {
                std::env::temp_dir()
            },
        }
    }
}

struct Tag<'a> {
    package: &'a str,
    version: Option<&'a str>,
}

impl<'a> Tag<'a> {
    fn new(package: &'a str, version: Option<&'a str>) -> Self {
        Self { package, version }
    }

    fn is_fixed_version(&self) -> bool {
        self.version
            .map(|v| semver::Version::parse(v).is_ok())
            .unwrap_or(false)
    }
}

impl Display for Tag<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let tag = if let Some(version) = &self.version {
            Cow::Owned(format!("{}@{}", self.package, version))
        } else {
            Cow::Borrowed(self.package)
        };
        f.write_str(&tag)
    }
}

impl ExecutorResolver for NpmResolver<'_> {
    fn resolve(&self, url: &Url) -> Result<ExecutorResolution> {
        let package = url.path();
        let registry = url.host_str();

        let package_root = self.packages_root.join(random_string(12));
        let package_root_str = path_to_string(&package_root)?;

        std::fs::create_dir_all(&package_root_str)
            .with_context(|| format!("could not create directory at {package_root_str}"))?;

        let npmrc = CustomNpmrcConfig::from_options_and_registry(&self.options, registry);

        let install = npmrc.use_config(|environment| {
            let tag = Tag::new(package, self.options.version());
            npm(
                [
                    "install",
                    "--global",
                    "--prefix",
                    package_root_str.as_str(),
                    &tag.to_string(),
                ],
                ProcessOptions {
                    display_output: true,
                    environment,
                    ..Default::default()
                },
            )
            .context("could not spawn `npm install` command")?
            .wait()
            .context("could not wait for `npm install` command")
        })??;

        if !install.success {
            bail!("`npm install` failed (code: {:?})", install.code);
        }

        let loader = ExecutorLoadStrategy::NodePackage.get_loader(LoaderContext {
            workspace: self.workspace,
        });

        let executor_with_metadata = loader.load_from_src(&package_root)?;

        let package_json = PackageJson::from_package_root(&package_root)?;

        Ok(ExecutorResolution {
            executor: executor_with_metadata.executor,
            state: to_value(State {
                package_root,
                package_version: package_json.version,
                load_metadata: executor_with_metadata.metadata,
            })?,
        })
    }

    fn update(&self, url: &Url, state: &Value) -> Result<ExecutorUpdate> {
        let mut state = State::deserialize(state)?;

        let current_package_json =
            PackageJson::from_package_root(&state.package_root).with_context(|| {
                format!(
                    "could not read installed executor package.json file at {}",
                    state.package_root.display()
                )
            })?;

        if current_package_json.version != state.package_version {
            bail!(
                "executor has wrong version (expected={}, actual={})",
                state.package_version,
                current_package_json.version
            )
        }

        let package = url.path();
        let registry = url.host_str();
        let tag = Tag::new(package, registry);

        let loader = ExecutorLoadStrategy::NodePackage.get_loader(LoaderContext {
            workspace: self.workspace,
        });

        if tag.is_fixed_version() || !self.options.pull() {
            return Ok(ExecutorUpdate {
                executor: loader.load_from_metadata(&state.load_metadata)?,
                new_state: None,
                updated: false,
            });
        }

        let npmrc = CustomNpmrcConfig::from_options_and_registry(&self.options, registry);
        let update = npmrc.use_config(|environment| {
            let package_root_str = path_to_string(&state.package_root)?;
            npm(
                [
                    "update",
                    "--global",
                    "--prefix",
                    package_root_str.as_str(),
                    &tag.to_string(),
                ],
                ProcessOptions {
                    display_output: true,
                    environment,
                    ..Default::default()
                },
            )
            .context("could not spawn `npm update` command")?
            .wait()
            .context("could not wait for `npm update` command")
        })??;

        if !update.success {
            bail!("`npm update` failed (code={:?})", update.code)
        }

        let new_package_json =
            PackageJson::from_package_root(&state.package_root).with_context(|| {
                format!(
                    "could not read updated executor package.json file at {}",
                    state.package_root.display()
                )
            })?;

        if new_package_json.version == current_package_json.version {
            return Ok(ExecutorUpdate {
                executor: loader.load_from_metadata(&state.load_metadata)?,
                new_state: None,
                updated: false,
            })
        }

        let ExecutorWithMetadata { executor, metadata } = loader.load_from_src(&state.package_root)?;
        state.package_version = new_package_json.version;
        state.load_metadata = metadata;

        Ok(ExecutorUpdate { 
            executor, 
            new_state: Some(to_value(state)?), 
            updated: true
        })
    }
}
