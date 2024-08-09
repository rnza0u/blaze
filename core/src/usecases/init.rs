use std::{borrow::Cow, path::Path};

use anyhow::{anyhow, bail, Context};
use blaze_common::{configuration_file::ConfigurationFileFormat, error::Result};

use crate::{
    logging::get_logger,
    system::{
        env::{Env, USER_ENV_FILE},
        repository::{add_to_gitignore, open_or_init_repository},
    },
    workspace::{
        cache_store::CacheStore, init::init_workspace_files, template::HELPERS_FOLDER,
        variables::VARIABLES_FILE_PATH, workspace_handle::WorkspaceHandle,
    },
    WorkspaceGlobals,
};

use super::GlobalOptions;

#[derive(Default)]
pub struct InitOptions {
    pub create_directory: bool,
    pub format: ConfigurationFileFormat,
    pub name: Option<String>,
    pub no_git: bool,
}

/// Create a new workspace. This will initialize the root configuration file (workspace.json).
pub fn init<R: AsRef<Path>>(root: R, options: InitOptions, globals: GlobalOptions) -> Result<()> {
    let logger = get_logger(globals.get_log_level().unwrap_or_default());

    let root_ref = root.as_ref();

    logger.debug(format!(
        "checking if workspace already exists at {}.",
        root_ref.display()
    ));

    if WorkspaceHandle::exists_at_root(root_ref)? {
        bail!("workspace already exists at this location.");
    }

    logger.debug(format!(
        "reading directory metadata {}.",
        root_ref.display()
    ));

    match std::fs::metadata(root_ref).map(|metadata| metadata.is_dir()) {
        Ok(true) => {}
        Ok(false) => bail!("{} must be a directory.", root_ref.display()),
        Err(err) => {
            if err.kind() != std::io::ErrorKind::NotFound {
                bail!(anyhow!(err).context("an error occured while reading root folder metadata."));
            }
            if !options.create_directory {
                bail!("{} does not exist", root_ref.display());
            }
            std::fs::create_dir_all(root_ref).with_context(|| {
                format!(
                    "could not create workspace directory at {}.",
                    root_ref.display()
                )
            })?;
        }
    };

    let name = options.name
        .or_else(|| {
            logger.debug("workspace name was not provided. infering name from root directory name");
            root_ref.file_name()
                .and_then(|os_str| os_str.to_str().map(|str| str.to_string()))
        })
        .ok_or_else(|| anyhow!(
            "could not infer workspace name from path {}, try providing a workspace name or use an appropriate directory", 
            root_ref.display()
        ))?;

    logger.debug("persisting new files to disk");

    Env::create_dotenv_files(root_ref).context("could not write dotenv files")?;
    init_workspace_files(&name, root_ref, options.format)
        .context("could not write workspace files")?;
    let _ = WorkspaceGlobals::new(root_ref, globals)?;

    if options.no_git {
        logger.debug("skipping git repository initialization");
    } else {
        logger.debug("initializing git repository...");
        let _ = open_or_init_repository(root_ref)?;
        logger.debug("adding git ignore rules");
        add_to_gitignore(
            root_ref,
            &[
                Cow::Borrowed(USER_ENV_FILE),
                ".blaze/*".into(),
                format!("!{}", HELPERS_FOLDER).into(),
                format!("!{}.*", VARIABLES_FILE_PATH).into(),
                "!.blaze/jpath".into(),
                "user-variables.*".into(),
            ],
        )?;
    }

    logger.debug("initializing cache");

    let _ = CacheStore::load(root_ref)?;

    Ok(())
}
