use std::{
    collections::{BTreeMap, HashSet},
    fmt::Debug,
    io::{self, Read},
    num::NonZeroUsize,
    path::{Path, PathBuf},
    str::FromStr,
};

use anyhow::{anyhow, Context};
use blaze_common::{
    error::Result,
    shell::{Shell, ShellKind},
    util::path_to_string,
    value::{to_value, Value},
    workspace::Workspace,
};
use handlebars::{no_escape, Handlebars, HelperDef, RenderError, RenderErrorReason};
use serde::Deserialize;

use crate::system::{
    env::Env,
    process::{Process, ProcessOptions},
    random::random_string,
    shell::ShellFormatter,
};

pub const HELPERS_FOLDER: &str = ".blaze/helpers";

const ROOT_KEY: &str = "root";

const WORKSPACE_KEY: &str = "workspace";
const PROJECT_KEY: &str = "project";
const PROJECT_NAME_KEY: &str = "name";
const PROJECT_ROOT_KEY: &str = "root";

const ENVIRONMENT_KEY: &str = "environment";

const VARIABLES_KEY: &str = "vars";

const PLATFORM_KEY: &str = "platform";
const FAMILY_KEY: &str = "family";
const ARCHITECTURE_KEY: &str = "architecture";
const SEPARATOR_KEY: &str = "sep";
const USER_KEY: &str = "user";
const HOSTNAME_KEY: &str = "hostname";

#[cfg(not(windows))]
const SEPARATOR: &str = "/";

#[cfg(windows)]
const SEPARATOR: &str = "\\";

#[derive(Default, Clone)]
pub struct TemplateData<'reg> {
    data: Value,
    generator: Handlebars<'reg>,
}

impl Debug for TemplateData<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.data.fmt(f)
    }
}

impl<'reg> TemplateData<'reg> {
    pub fn try_new(workspace_root: &Path) -> Result<Self> {
        let rhai_scripts_path = workspace_root.join(HELPERS_FOLDER);

        let rhai_scripts_path_exists = match std::fs::metadata(rhai_scripts_path) {
            Ok(metadata) => metadata.is_dir(),
            Err(err) if err.kind() == io::ErrorKind::NotFound => false,
            Err(err) => return Err(err.into()),
        };

        let scripts = rhai_scripts_path_exists
            .then(|| {
                std::fs::read_dir(workspace_root.join(HELPERS_FOLDER))?
                    .filter_map(|entry| {
                        let entry = match entry {
                            Ok(entry) => entry,
                            Err(err) => return Some(Err(anyhow!(err))),
                        };
                        match entry.file_type() {
                            Ok(filetype) if filetype.is_file() => {}
                            Ok(_) => return None,
                            Err(err) => return Some(Err(anyhow!(err))),
                        };
                        let path = entry.path();
                        match path.extension()?.to_str() {
                            Some("rhai") => {}
                            Some(_) => return None,
                            None => return Some(Err(anyhow!("could not get file extension"))),
                        };

                        Some(Ok(path))
                    })
                    .collect::<Result<HashSet<_>>>()
            })
            .transpose()?
            .unwrap_or_default();

        Ok(Self {
            data: Self::initial_data(workspace_root)?,
            generator: Self::generator(workspace_root, &scripts)?,
        })
    }

    pub fn extend_with_env(&mut self) {
        self.data
            .overwrite(Value::object([(ENVIRONMENT_KEY, Env::get_all_as_value())]));
    }

    pub fn extend_with_workspace(&mut self, workspace: &Workspace) -> Result<()> {
        self.data
            .overwrite(Value::object([(WORKSPACE_KEY, to_value(workspace)?)]));
        Ok(())
    }

    pub fn extend_with_variables(&mut self, value: Value) {
        self.data.overwrite(Value::object([(VARIABLES_KEY, value)]));
    }

    pub fn with_project(&self, name: &str, root: &Path) -> Result<Self> {
        let mut copy = self.clone();

        copy.data.overwrite(Value::object([(
            PROJECT_KEY,
            Value::object([
                (PROJECT_NAME_KEY, Value::string(name)),
                (PROJECT_ROOT_KEY, Value::string(path_to_string(root)?)),
            ]),
        )]));

        Ok(copy)
    }

    pub fn inner(&self) -> &Value {
        &self.data
    }

    pub fn render_str(&self, view: &str) -> Result<String> {
        self.generator
            .render_template(view, &self.data)
            .context("could not compile view template")
    }

    pub fn render(&self, value: &mut Value) -> Result<()> {
        if let Some(string) = value.as_str() {
            *value = Value::string(self.render_str(string)?);
        } else if let Some(mut obj) = value.as_mut_object() {
            for field in obj.values_mut() {
                self.render(field)?;
            }
        } else if let Some(arr) = value.as_mut_vec() {
            for el in arr {
                self.render(el)?;
            }
        }
        Ok(())
    }

    fn initial_data(root: &Path) -> Result<Value> {
        let mut keys = vec![
            (ROOT_KEY, Value::string(path_to_string(root)?)),
            (SEPARATOR_KEY, Value::string(SEPARATOR)),
            (PLATFORM_KEY, std::env::consts::OS.into()),
            (FAMILY_KEY, std::env::consts::FAMILY.into()),
            (ARCHITECTURE_KEY, std::env::consts::ARCH.into()),
            (USER_KEY, whoami::username().into()),
        ];

        if let Ok(hostname) = whoami::fallible::hostname() {
            keys.push((HOSTNAME_KEY, hostname.into()));
        }

        Ok(Value::object(keys))
    }

    fn generator(
        workspace_root: &Path,
        helper_scripts: &HashSet<PathBuf>,
    ) -> Result<Handlebars<'reg>> {
        let mut reg = Handlebars::new();
        reg.set_strict_mode(true);
        reg.register_escape_fn(no_escape);
        reg.register_helper(
            "shell",
            Box::new(ShHelper {
                root: workspace_root.to_owned(),
            }),
        );
        reg.register_helper("random", Box::new(RandomHelper));
        for script in helper_scripts {
            let helper_name = script
                .file_stem()
                .unwrap()
                .to_str()
                .ok_or_else(|| anyhow!("could not get file stem for {}", script.display()))?;
            reg.register_script_helper_file(helper_name, script)?;
        }
        Ok(reg)
    }
}

struct ShHelper {
    root: PathBuf,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ShHelperNamedParams {
    #[serde(default)]
    trim: bool,
    shell: Option<PathBuf>,
    shell_kind: Option<ShellKind>,
    cwd: Option<PathBuf>,
}

impl HelperDef for ShHelper {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &handlebars::Helper<'rc>,
        _: &'reg Handlebars<'reg>,
        _: &'rc handlebars::Context,
        _: &mut handlebars::RenderContext<'reg, 'rc>,
        out: &mut dyn handlebars::Output,
    ) -> handlebars::HelperResult {
        let generic_error = |message: &str| -> RenderError {
            RenderErrorReason::Other(format!("shell template helper error: {message}")).into()
        };

        let hash_value = h
            .hash()
            .iter()
            .map(|(field, value)| (field, value.value()))
            .collect::<BTreeMap<_, _>>();

        let hash_value = to_value(hash_value).unwrap();

        let params = ShHelperNamedParams::deserialize(hash_value)
            .map_err(|err| generic_error(&err.to_string()))?;

        let command = h
            .param(0)
            .and_then(|p| p.value().as_str())
            .ok_or_else(|| RenderErrorReason::ParamNotFoundForIndex("command", 0))?;

        let shell = params
            .shell
            .as_ref()
            .map(|path| Shell::new(path, params.shell_kind));

        let shell_formatter = shell
            .as_ref()
            .map(ShellFormatter::from_shell)
            .unwrap_or_else(ShellFormatter::default);

        let (program, arguments) = shell_formatter.format_command(command).map_err(|err| {
            generic_error(&format!(
                "could not format helper command \"{command}\" ({err})"
            ))
        })?;

        let process = Process::run_with_options(
            program,
            arguments,
            ProcessOptions {
                cwd: Some(params.cwd.unwrap_or_else(|| self.root.to_owned())),
                display_output: false,
                ..Default::default()
            },
        )
        .map_err(|err| {
            generic_error(&format!(
                "could not create process for \"{command}\" ({err})"
            ))
        })?;

        let mut stdout = process
            .stdout()
            .map_err(|err| generic_error(&format!("could not get stdout for {process} ({err})")))?;

        let status = process
            .wait()
            .map_err(|err| generic_error(&format!("could not wait for command process ({err})")))?;

        if !status.success {
            return Err(generic_error(&format!(
                "command failed with status code: {:?}",
                status.code
            )))?;
        }

        let mut buffer = vec![];
        stdout.read_to_end(&mut buffer)?;

        let mut output = (*String::from_utf8_lossy(&buffer)).to_owned();

        if params.trim {
            output.truncate(output.trim_end().len());
        }

        out.write(&output)?;

        Ok(())
    }
}

struct RandomHelper;

impl HelperDef for RandomHelper {
    fn call<'reg: 'rc, 'rc>(
        &self,
        h: &handlebars::Helper<'rc>,
        _: &'reg Handlebars<'reg>,
        _: &'rc handlebars::Context,
        _: &mut handlebars::RenderContext<'reg, 'rc>,
        out: &mut dyn handlebars::Output,
    ) -> handlebars::HelperResult {
        let size = NonZeroUsize::from_str(
            &h.param(0)
                .ok_or_else(|| RenderErrorReason::ParamNotFoundForIndex("size", 0))?
                .render(),
        )
        .map_err(|err| RenderErrorReason::Other(format!("invalid random string size ({err})")))?;
        out.write(&random_string(size.get()))?;
        Ok(())
    }
}
