use std::{collections::HashMap, path::Path};

use anyhow::Context;
use blaze_common::{configuration_file::ConfigurationFileFormat, error::Result};
use serde::Serialize;

pub fn init_workspace_files(
    name: &str,
    root: &Path,
    format: ConfigurationFileFormat,
) -> Result<()> {
    struct InitFile {
        content: &'static str,
        dst: &'static Path,
    }

    macro_rules! get_files {
        ($ext:literal) => {
            vec![
                InitFile {
                    content: include_str!(concat!($ext, "/workspace.handlebars")),
                    dst: Path::new(concat!("workspace.", $ext)),
                },
                InitFile {
                    content: include_str!(concat!($ext, "/variables.handlebars")),
                    dst: Path::new(concat!(".blaze/variables.", $ext)),
                },
                InitFile {
                    content: include_str!(concat!($ext, "/user-variables.handlebars")),
                    dst: Path::new(concat!("user-variables.", $ext)),
                },
                InitFile {
                    content: include_str!(concat!($ext, "/project.handlebars")),
                    dst: Path::new(concat!("example-project/project.", $ext)),
                },
            ]
        };
    }

    let files: HashMap<ConfigurationFileFormat, Vec<InitFile>> = [
        (ConfigurationFileFormat::Jsonnet, get_files!("jsonnet")),
        (ConfigurationFileFormat::Json, get_files!("json")),
        (ConfigurationFileFormat::Yaml, get_files!("yml")),
    ]
    .into();

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct ExampleFileData<'a> {
        workspace_name: &'a str,
    }

    for file in &files[&format] {
        let rendered = handlebars::Handlebars::new().render_template(
            file.content,
            &ExampleFileData {
                workspace_name: name,
            },
        )?;
        let path = root.join(file.dst);
        let parent = path.parent().unwrap();
        std::fs::create_dir_all(parent)
            .with_context(|| format!("could not create directory at {}", parent.display()))?;
        std::fs::write(root.join(file.dst), rendered)
            .with_context(|| format!("could not write workspace file at {}", file.dst.display()))?;
    }

    Ok(())
}
