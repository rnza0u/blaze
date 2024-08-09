use std::path::PathBuf;

use blaze_devkit::{
    configuration_file::ConfigurationFileFormat, logger::LogLevel, value::Value, ExecutorContext, ExecutorResult, IntoEnumIterator
};

#[export_name = "execute"]
pub fn execute(ctx: &ExecutorContext, options: &Value) -> ExecutorResult {

    // options check
    assert_eq!(
        &Value::object([
            ("number", Value::unsigned(1)),
            ("string", Value::string("hello")),
            ("bool", Value::bool(true)),
            ("array", Value::array([Value::unsigned(1), Value::unsigned(2), Value::unsigned(3)])),
            ("null", Value::Null),
            ("float", Value::float(1.0))
        ]), 
        options
    );

    // workspace check
    assert!(ctx.workspace.root().is_absolute());
    assert_eq!("workspace", ctx.workspace.name());
    assert_eq!(ConfigurationFileFormat::Json, ctx.workspace.configuration_file_format());
    assert!(ctx.workspace.configuration_file_path().ends_with("workspace.json"));

    assert!(ctx.workspace.projects().contains_key("project"));
    let project_ref = ctx.workspace.projects().get("project").unwrap();
    assert_eq!(PathBuf::from("project"), project_ref.path());
    assert!(project_ref.description().is_none());
    assert!(project_ref.tags().is_empty());
    
    // project check
    assert_eq!("project", ctx.project.name());
    assert_eq!(ConfigurationFileFormat::Json, ctx.project.configuration_file_format());
    assert!(ctx.project.root().is_absolute());
    assert_eq!(ctx.workspace.root().join("project"), ctx.project.root());
    assert_eq!(ctx.workspace.root().join("project/project.json"), ctx.project.configuration_file_path());

    // context check
    assert_eq!("target", ctx.target);

    // test logger
    for log_level in LogLevel::iter(){
        ctx.logger.log("hello world!", log_level);
    }

    Ok(())
}