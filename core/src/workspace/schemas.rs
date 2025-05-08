use anyhow::bail;
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

pub fn validate_json(validator: &Validator, value: &Value) -> Result<()> {
    let mut formatted_errors = Vec::with_capacity(2);

    for error in validator.iter_errors(&serde_json::to_value(value)?) {
        formatted_errors.push(format!(
            "validation error: {} (at {})",
            error, error.instance_path
        ));
    }

    if !formatted_errors.is_empty() {
        bail!(formatted_errors.join("\n"))
    }

    Ok(())
}

pub(crate) use create_schema;
