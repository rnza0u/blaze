use std::collections::HashMap;

use blaze_common::{error::Result, util::path_to_string};

use super::ExecutorContext;

const WORKSPACE_NAME: &str = "BLAZE_WORKSPACE_NAME";
const WORKSPACE_ROOT: &str = "BLAZE_WORKSPACE_ROOT";
const WORKSPACE_CONFIGURATION_FILE_PATH: &str = "BLAZE_WORKSPACE_CONFIGURATION_FILE_PATH";
const WORKSPACE_CONFIGURATION_FILE_FORMAT: &str = "BLAZE_WORKSPACE_CONFIGURATION_FILE_FORMAT";
const PROJECT_NAME: &str = "BLAZE_PROJECT_NAME";
const PROJECT_ROOT: &str = "BLAZE_PROJECT_ROOT";
const TARGET: &str = "BLAZE_TARGET";

pub fn get_executor_env(ctx: &ExecutorContext) -> Result<HashMap<String, String>> {
    Ok([
        (WORKSPACE_NAME.into(), ctx.workspace.name().to_owned()),
        (WORKSPACE_ROOT.into(), path_to_string(ctx.workspace.root())?),
        (
            WORKSPACE_CONFIGURATION_FILE_PATH.into(),
            path_to_string(ctx.workspace.configuration_file_path())?,
        ),
        (
            WORKSPACE_CONFIGURATION_FILE_FORMAT.into(),
            ctx.workspace.configuration_file_format().to_string(),
        ),
        (PROJECT_NAME.into(), ctx.project.name().to_owned()),
        (PROJECT_ROOT.into(), path_to_string(ctx.project.root())?),
        (TARGET.into(), ctx.target.to_owned()),
    ]
    .into())
}
