use std::{borrow::Cow, ops::Deref, path::PathBuf, str::FromStr};

use anyhow::bail;
use blaze_common::{
    configuration_file::ConfigurationFileFormat,
    error::{Error, Result},
    logger::LogLevel,
    variables::VariablesOverride,
};
use blaze_core::GlobalOptions;
use clap::Parser;

use crate::{context::CliContext, subcommand::BlazeSubCommand};

#[derive(Debug, Clone)]
struct StrVariableOverride {
    path: String,
    value: String,
}

impl FromStr for StrVariableOverride {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(match s.split_once('=') {
            Some((key, value)) => Self {
                path: key.to_owned(),
                value: value.to_owned(),
            },
            None => bail!("invalid variable override {s}"),
        })
    }
}

#[derive(Parser, Debug)]
#[command(
    display_name = "blaze",
    name = "blaze",
    about = "Blaze CLI root options.",
    long_about = "Blaze CLI root options. These will be available for all Blaze commands and they are all optional."
)]
pub struct Command {
    #[arg(
        short = 'r',
        long = "root",
        help = "Path to the workspace root directory.",
        long_help = "Path to the workspace root directory. Defaults to the current working directory. \
You can provide either an absolute path or a path relative to your current working directory."
    )]
    root: Option<PathBuf>,

    #[arg(
        short = 'l',
        long = "log-level",
        help = "Global level of logging.",
        long_help = "Global level of logging. \
For the maximum level of logging, use the value `Debug`. \
If you only care about errors, use the value `Error`. \
If you want to ignore warnings and debugging messages, use the value `Info`. \
The default mode (`Warn`) will display everything except debugging messages."
    )]
    log_level: Option<LogLevel>,

    #[arg(
        short = 'n',
        long = "no-cache",
        help = "Disable cache globally.",
        long_help = "Disable cache globally. \
This will prevent Blaze from accessing cache data at any time (for reading or writing). \
Target executions or resolved executors will not be saved to cache."
    )]
    no_cache: bool,

    #[arg(
        long,
        help = "Override variables through a JSON string.",
        long_help = "Override variables through a JSON string. \
The option value must be a valid JSON document. \
Variables will be overriden using the JSON Merge patch algorithm (RFC7396).",
        conflicts_with_all = ["yaml_var", "jsonnet_var"]
    )]
    json_var: Option<String>,

    #[arg(
        long,
        help = "Override variables through a YAML string.",
        long_help = "Override variables through a YAML string. \
The option value must be a valid YAML document. \
Variables will be overriden using the JSON Merge patch algorithm (RFC7396).",
        conflicts_with_all = ["json_var", "jsonnet_var"]
    )]
    yaml_var: Option<String>,

    #[arg(
        long,
        help = "Override variables through a Jsonnet string.",
        long_help = "Override variables through a Jsonnet string. \
The option value must be a valid Jsonnet snippet. \
Variables will be overriden using the JSON Merge patch algorithm (RFC7396).",
        conflicts_with_all = ["json_var", "yaml_var"]
    )]
    jsonnet_var: Option<String>,

    #[arg(
        long = "str-var",
        help = "Override a single variable with a string value.",
        long_help = "Override a single variable with a string value. \
The expected format is the variable name, followed by an `=` sign and the corresponding value. \
For example you could override the `foo` variable with the string `bar` using the syntax: `foo=bar`. \
You can override multiple variables by passin this options multiple times. \
You can override nested values with multiple keys separated by dots, for example `my.nested.key=value`."
    )]
    str_vars: Option<Vec<StrVariableOverride>>,

    #[arg(
        long = "var-file",
        help = "Override variables with a custom variables file.",
        long_help = "Override variables with a custom variables file. \
The option value must be the path to the custom variable file."
    )]
    var_files: Option<Vec<PathBuf>>,

    #[clap(subcommand)]
    subcommand: BlazeSubCommand,
}

impl Command {
    pub fn execute(&self, context: CliContext) -> Result<()> {
        let mut variable_overrides = Vec::<VariablesOverride>::with_capacity(
            self.str_vars.as_ref().map(|v| v.len()).unwrap_or(0)
                + if self.json_var.is_some() { 1 } else { 0 }
                + if self.jsonnet_var.is_some() { 1 } else { 0 }
                + if self.yaml_var.is_some() { 1 } else { 0 }
                + self.var_files.as_ref().map(|v| v.len()).unwrap_or(0),
        );

        if let Some(var_files) = &self.var_files {
            variable_overrides.extend(var_files.iter().map(|path| VariablesOverride::File {
                path: if path.is_absolute() {
                    path.to_owned()
                } else {
                    context.cwd.join(path)
                },
            }));
        }

        if let Some(code) = &self.json_var {
            variable_overrides.push(VariablesOverride::Code {
                format: ConfigurationFileFormat::Json,
                code: code.to_owned(),
            })
        } else if let Some(code) = &self.yaml_var {
            variable_overrides.push(VariablesOverride::Code {
                format: ConfigurationFileFormat::Yaml,
                code: code.to_owned(),
            })
        } else if let Some(code) = &self.jsonnet_var {
            variable_overrides.push(VariablesOverride::Code {
                format: ConfigurationFileFormat::Jsonnet,
                code: code.to_owned(),
            })
        }

        if let Some(str_vars) = &self.str_vars {
            variable_overrides.extend(str_vars.iter().map(|str_var| VariablesOverride::String {
                path: str_var.path.split('.').map(str::to_owned).collect(),
                value: str_var.value.to_owned(),
            }));
        }

        let mut global_options = GlobalOptions::new().with_variable_overrides(variable_overrides);

        if let Some(level) = self.log_level {
            global_options = global_options.with_log_level(level);
        }

        if self.no_cache {
            global_options = global_options.without_cache();
        }

        self.subcommand.execute(
            match &self.root {
                Some(dir) => {
                    if dir.is_relative() {
                        Cow::Owned(context.cwd.join(dir))
                    } else {
                        Cow::Borrowed(dir.as_path())
                    }
                }
                None => Cow::Borrowed(context.cwd.as_path()),
            }
            .deref(),
            global_options,
        )
    }
}
