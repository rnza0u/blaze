use std::collections::HashSet;
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};

use anyhow::Context;
use anyhow::{anyhow, bail};
use blaze_common::error::Error;
use blaze_common::IntoEnumIterator;
use blaze_common::{configuration_file::ConfigurationFileFormat, error::Result, value::Value};
use jrsonnet_evaluator::{FileImportResolver, Val};
use serde::de::DeserializeOwned;

use super::template::TemplateData;

const JSONNET_EXTVAR_NAME: &str = "blaze";

/// Get corresponding extensions for a given file type. The returned tuple's first element is the main extension.
/// The second tuple element is a list of other extensions that are supported as well.
pub fn get_format_extensions(format: ConfigurationFileFormat) -> (&'static str, Vec<&'static str>) {
    match format {
        ConfigurationFileFormat::Json => ("json", vec![]),
        ConfigurationFileFormat::Yaml => ("yml", vec!["yaml"]),
        ConfigurationFileFormat::Jsonnet => ("jsonnet", vec![]),
    }
}

pub fn infer_format_from_path(path: &Path) -> Option<ConfigurationFileFormat> {
    path.extension()
        .and_then(|ext| ext.to_str())
        .and_then(|ext| {
            ConfigurationFileFormat::iter().find(|format| {
                let extensions = get_format_extensions(*format);
                extensions.0 == ext || extensions.1.contains(&ext)
            })
        })
}

/// Get the full path of a Blaze configuration file in a specific directory
pub fn infer_configuration_file_path<P: AsRef<Path>, S: AsRef<str>>(
    dir: P,
    filename: S,
) -> Result<Option<(ConfigurationFileFormat, PathBuf)>> {
    let dir_ref = dir.as_ref();
    let filename_ref = filename.as_ref();
    for format in ConfigurationFileFormat::iter() {
        let (main_ext, other_ext) = get_format_extensions(format);

        for ext in vec![main_ext].into_iter().chain(other_ext) {
            let configuration_file_path = dir_ref.join(format!("{}.{}", filename_ref, ext));
            match std::fs::metadata(&configuration_file_path) {
                Ok(_) => return Ok(Some((format, configuration_file_path))),
                Err(err) => match err.kind() {
                    std::io::ErrorKind::NotFound => continue,
                    _ => bail!(err),
                },
            };
        }
    }

    Ok(None)
}

#[derive(Clone, Copy)]
pub struct DeserializationContext<'a> {
    pub template_data: &'a TemplateData<'a>,
    pub jpath: &'a HashSet<PathBuf>,
}

pub fn deserialize_configuration<T>(
    path: &Path,
    file_type: ConfigurationFileFormat,
    context: DeserializationContext,
) -> Result<T>
where
    T: DeserializeOwned,
{
    let get_stream = || {
        OpenOptions::new()
            .read(true)
            .open(path)
            .with_context(|| format!("could not open configuration file at {}", path.display()))
    };

    let mut value: Value = match file_type {
        ConfigurationFileFormat::Json => serde_json::from_reader(get_stream()?)
            .with_context(|| format!("could not deserialize JSON at {}", path.display()))?,
        ConfigurationFileFormat::Yaml => serde_yaml::from_reader(get_stream()?)
            .with_context(|| format!("could not deserialize YAML at {}", path.display()))?,
        ConfigurationFileFormat::Jsonnet => {
            let state = create_jsonnet_evaluation_state(context.template_data, context.jpath)?;

            let val = state
                .import(path)
                .map_err(convert_jrsonnet_error)
                .with_context(|| format!("could not evaluate jsonnet file {}", path.display()))?;

            jrsonnet_val_to_value(val)?
        }
    };

    context.template_data.render(&mut value)?;

    Ok(T::deserialize(value)?)
}

fn convert_jrsonnet_error(error: jrsonnet_evaluator::Error) -> Error {
    let mut lines = Vec::with_capacity(1 + error.trace().0.len());
    lines.push(error.error().to_string());
    for el in &error.trace().0 {
        let mut parts = vec![];

        if let Some(location) = &el.location {
            if let Some(path) = location.0.source_path().path() {
                parts.push(path.to_string_lossy().to_string());
            }
            parts.push(format!("{}:{}", location.1, location.2));
        }

        parts.push(el.desc.to_owned());

        lines.push(parts.join(" "));
    }

    anyhow!(
        "jsonnet evaluation error: {}\n\n{}",
        error.error(),
        lines.join("\n")
    )
}

fn create_jsonnet_evaluation_state(
    template_data: &TemplateData,
    jpath: &HashSet<PathBuf>,
) -> Result<jrsonnet_evaluator::State> {
    let state = jrsonnet_evaluator::State::default();
    state.set_import_resolver(FileImportResolver::new(
        jpath.iter().map(PathBuf::to_owned).collect(),
    ));
    let ctx = jrsonnet_stdlib::ContextInitializer::new(
        state.clone(),
        jrsonnet_evaluator::trace::PathResolver::new_cwd_fallback(),
    );
    ctx.add_ext_var(
        JSONNET_EXTVAR_NAME.into(),
        value_to_jrsonnet_val(template_data.inner())?,
    );
    state.set_context_initializer(ctx);
    Ok(state)
}

fn jrsonnet_val_to_value(val: jrsonnet_evaluator::Val) -> Result<Value> {
    Ok(match val {
        Val::Arr(arr) => Value::array(
            arr.iter()
                .map(|val| jrsonnet_val_to_value(val.map_err(convert_jrsonnet_error)?))
                .collect::<Result<Vec<_>>>()?,
        ),
        Val::Bool(b) => Value::bool(b),
        Val::Str(str) => Value::string(str.to_string()),
        Val::Num(n) => Value::float(n),
        Val::Null => Value::Null,
        Val::Obj(obj) => Value::object(
            obj.fields()
                .iter()
                .map(|field| {
                    Ok((
                        field.to_string(),
                        jrsonnet_val_to_value(
                            obj.get(field.clone())
                                .map_err(convert_jrsonnet_error)?
                                .unwrap(),
                        )?,
                    ))
                })
                .collect::<Result<Vec<_>>>()?,
        ),
        _ => bail!("could not convert {val:?} to value"),
    })
}

fn value_to_jrsonnet_val(value: &Value) -> Result<jrsonnet_evaluator::Val> {
    Ok(match value {
        Value::Array(array) => Val::Arr(
            array
                .iter()
                .map(value_to_jrsonnet_val)
                .collect::<Result<Vec<_>>>()?
                .into(),
        ),
        Value::String(string) => Val::Str(string.as_str().into()),
        Value::Null => Val::Null,
        Value::Bool(b) => Val::Bool(*b),
        Value::Unsigned(u) => Val::Num(u32::try_from(*u)?.into()),
        Value::Signed(i) => Val::Num(i32::try_from(*i)?.into()),
        Value::Float(f) => Val::Num(*f),
        Value::Object(object) => {
            let mut builder = jrsonnet_evaluator::ObjValueBuilder::new();
            for (key, value) in object {
                builder
                    .field(key.as_str())
                    .value(value_to_jrsonnet_val(value)?);
            }
            Val::Obj(builder.build())
        }
    })
}

pub fn deserialize_code<T>(
    code: &str,
    format: ConfigurationFileFormat,
    context: DeserializationContext<'_>,
) -> Result<T>
where
    T: DeserializeOwned,
{
    let mut value: Value = match format {
        ConfigurationFileFormat::Json => serde_json::from_str(code)?,
        ConfigurationFileFormat::Yaml => serde_yaml::from_str(code)?,
        ConfigurationFileFormat::Jsonnet => {
            let state = create_jsonnet_evaluation_state(context.template_data, context.jpath)?;

            let val = state
                .evaluate_snippet("inline code", code)
                .map_err(convert_jrsonnet_error)
                .with_context(|| format!("could not evaluate jsonnet snippet {code}"))?;

            jrsonnet_val_to_value(val).context("could not parse Jsonnet output.")?
        }
    };
    context.template_data.render(&mut value)?;
    Ok(T::deserialize(value)?)
}
