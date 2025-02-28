use anyhow::anyhow;
use blaze_common::{error::Result, value::Value};
use jsonschema::Validator;

macro_rules! create_schema {
    ($name:literal) => {{
        let schema_str = include_str!(concat!(env!("BLAZE_JSON_SCHEMAS_LOCATION"), '/', $name));
        jsonschema::Validator::options()
            .with_draft(jsonschema::Draft::Draft202012)
            .build(&serde_json::from_str(schema_str).expect("could not parse JSON schema"))
            .expect("could not compile JSON schema")
    }};
}

pub fn validate_json(schema: &Validator, value: &Value) -> Result<()> {
    schema
        .validate(&serde_json::to_value(value)?)
        .map_err(|errors| {
            let mut lines = Vec::with_capacity(2);
            for error in errors {
                lines.push(format!(
                    "validation error: {} (at {})",
                    error, error.instance_path
                ));
            }
            anyhow!(lines.join("\n"))
        })
}

pub(crate) use create_schema;
