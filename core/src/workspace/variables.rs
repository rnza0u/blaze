use std::{
    collections::HashSet,
    fs, io,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, bail, Context};
use blaze_common::{
    error::Result,
    logger::Logger,
    value::Value,
    variables::{VariablesConfiguration, VariablesOverride},
};

use super::{
    configurations::{
        deserialize_code, deserialize_configuration, infer_configuration_file_path,
        infer_format_from_path, DeserializationContext,
    },
    template::TemplateData,
};

pub const VARIABLES_FILE_PATH: &str = ".blaze/variables";

pub struct LoadVariablesOptions<'a> {
    pub template_data: &'a TemplateData<'a>,
    pub logger: &'a Logger,
    pub overrides: Vec<VariablesOverride>,
    pub jpath: &'a HashSet<PathBuf>,
}

pub fn load_variables_from_root(root: &Path, options: LoadVariablesOptions) -> Result<Value> {
    let variables_file_path = Path::new(VARIABLES_FILE_PATH);

    let variables_files_folder_path = root.join(variables_file_path.parent().unwrap());
    let pathinfo = infer_configuration_file_path(
        &variables_files_folder_path,
        variables_file_path.file_name().unwrap().to_str().unwrap(),
    )?;

    let deserialization_context = DeserializationContext {
        jpath: options.jpath,
        template_data: options.template_data,
    };

    if pathinfo.is_none() {
        options.logger.debug("no variables file was found");
        let mut vars = Value::default();
        apply_overrides(&mut vars, root, options.overrides, deserialization_context)?;
        return Ok(vars);
    }

    let (file_type, variables_file_path) = pathinfo.unwrap();

    options.logger.debug(format!(
        "loading global variables from {}",
        variables_file_path.display()
    ));

    let configuration: VariablesConfiguration =
        deserialize_configuration(&variables_file_path, file_type, deserialization_context)
            .with_context(|| {
                format!(
                    "{} variable files has errors",
                    variables_file_path.display()
                )
            })?;

    let mut vars = configuration.vars;

    for extra_variables_file in configuration.include {
        let path = if extra_variables_file.path.is_absolute() {
            extra_variables_file.path
        } else {
            variables_files_folder_path.join(extra_variables_file.path)
        };

        if extra_variables_file.optional {
            match fs::metadata(&path) {
                Ok(metadata) if metadata.is_file() => {}
                Ok(_) => bail!("{} must be a file", path.display()),
                Err(err) if err.kind() == io::ErrorKind::NotFound => continue,
                Err(err) => return Err(err.into()),
            };
        }

        options
            .logger
            .debug(format!("loading extra variables from {}", path.display()));

        let deserialized: Value = deserialize_configuration(
            &path,
            infer_format_from_path(&path).ok_or_else(|| {
                anyhow!(
                    "could not get configuration file format from {}",
                    path.display()
                )
            })?,
            deserialization_context,
        )
        .with_context(|| {
            format!(
                "could not deserialize extra variables files at {}",
                path.display()
            )
        })?;

        vars.overwrite(&deserialized);
    }

    apply_overrides(&mut vars, root, options.overrides, deserialization_context)?;

    Ok(vars)
}

fn apply_overrides(
    variables: &mut Value,
    root: &Path,
    overrides: Vec<VariablesOverride>,
    deserialization_context: DeserializationContext<'_>,
) -> Result<()> {
    for variables_override in overrides {
        let overriding_value = match variables_override {
            VariablesOverride::String { path, value } => {
                let value = deserialization_context
                    .template_data
                    .render_str(&value)
                    .with_context(|| format!("error while rendering {value}"))?;

                path.into_iter()
                    .rev()
                    .fold(Value::string(value), |value, key| {
                        Value::object([(key, value)])
                    })
            }
            VariablesOverride::Code { format, code } => {
                deserialize_code(&code, format, deserialization_context)
                    .with_context(|| format!("error while evaluating {format} code \"{code}\""))?
            }
            VariablesOverride::File { path } => {
                let path = if path.is_absolute() {
                    path
                } else {
                    root.join(path)
                };
                let file_type = infer_format_from_path(&path).ok_or_else(|| {
                    anyhow!(
                        "could not infer configuration file format for {}",
                        path.display()
                    )
                })?;
                deserialize_configuration(&path, file_type, deserialization_context).with_context(
                    || {
                        format!(
                            "error while evaluating user-provided variables file {}",
                            path.display()
                        )
                    },
                )?
            }
        };
        variables.overwrite(overriding_value);
    }
    Ok(())
}
