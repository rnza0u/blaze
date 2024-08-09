use std::{
    collections::HashSet,
    fs::File,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
};

use anyhow::{bail, Context};
use blaze_common::{
    error::Result,
    logger::{LogLevel, Logger},
    variables::VariablesOverride,
};

use crate::{
    logging::get_logger,
    system::{env::Env, locks::clean_locks},
    workspace::{
        cache_store::CacheStore,
        configurations::DeserializationContext,
        template::TemplateData,
        variables::{load_variables_from_root, LoadVariablesOptions},
        workspace_handle::{OpenWorkspaceOptions, WorkspaceHandle},
    },
};

const JPATH_FILE: &str = ".blaze/.jpath";

#[derive(Default)]
pub struct GlobalOptions {
    log_level: Option<LogLevel>,
    no_cache: bool,
    variable_overrides: Vec<VariablesOverride>,
}

impl GlobalOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_log_level(mut self, level: LogLevel) -> Self {
        self.log_level = Some(level);
        self
    }

    pub fn without_cache(mut self) -> Self {
        self.no_cache = true;
        self
    }

    pub fn with_variable_overrides<I: IntoIterator<Item = VariablesOverride>>(
        mut self,
        overrides: I,
    ) -> Self {
        self.variable_overrides = overrides.into_iter().collect();
        self
    }

    pub fn get_log_level(&self) -> Option<LogLevel> {
        self.log_level
    }

    pub fn is_no_cache(&self) -> bool {
        self.no_cache
    }

    pub fn get_variable_overrides(&self) -> &[VariablesOverride] {
        &self.variable_overrides
    }
}

pub struct WorkspaceGlobals<'a> {
    workspace_handle: WorkspaceHandle,
    logger: Logger,
    log_level: LogLevel,
    cache: Option<CacheStore>,
    template_data: TemplateData<'a>,
    jpath: HashSet<PathBuf>,
}

impl<'a> WorkspaceGlobals<'a> {
    pub fn workspace_handle(&self) -> &WorkspaceHandle {
        &self.workspace_handle
    }

    pub fn logger(&self) -> Logger {
        self.logger.clone()
    }

    pub fn log_level(&self) -> LogLevel {
        self.log_level
    }

    pub fn cache(&self) -> Option<&CacheStore> {
        self.cache.as_ref()
    }

    pub fn deserialization_context(&'a self) -> DeserializationContext<'a> {
        DeserializationContext {
            jpath: &self.jpath,
            template_data: &self.template_data,
        }
    }
}

impl WorkspaceGlobals<'_> {
    pub fn new(root: &Path, options: GlobalOptions) -> Result<Self> {
        let mut root = dunce::canonicalize(root)
            .with_context(|| format!("could not canonicalize root directory {}", root.display()))?;
        let base_root = root.clone();
        loop {
            if WorkspaceHandle::exists_at_root(&root)? {
                break;
            }
            if !root.pop() {
                bail!("{} is not part of a Blaze workspace.", base_root.display())
            }
        }

        let jpath = get_jpath(&root).context("error while reading jpath file")?;

        let mut log_level = options.log_level.unwrap_or_default();
        let mut logger = get_logger(log_level);

        logger.debug(format!("loading .env files from {}", root.display()));

        Env::load_dotenv_files(&root).context("error while loading dotenv files")?;

        let mut template_data =
            TemplateData::try_new(&root).context("error while initializing template data")?;
        template_data.extend_with_env();

        logger.debug("loading workspace variables");

        let variables = load_variables_from_root(
            &root,
            LoadVariablesOptions {
                template_data: &template_data,
                logger: &logger,
                overrides: options.variable_overrides,
                jpath: &jpath,
            },
        )
        .context("error while loading workspace variables")?;

        template_data.extend_with_variables(variables);

        logger.debug(format!("loading workspace from {}", root.display()));

        let workspace_handle = WorkspaceHandle::from_root(
            &root,
            OpenWorkspaceOptions {
                template_data: &template_data,
                jpath: &jpath,
            },
        )
        .with_context(|| format!("error while loading workspace from {}", root.display()))?;

        if options.log_level.is_none() {
            if let Some(level) = workspace_handle.inner().settings().log_level() {
                log_level = level;
                logger = get_logger(level);
            }
        }

        template_data.extend_with_workspace(workspace_handle.inner())?;

        let cache = (!options.no_cache)
            .then(|| CacheStore::load(&root))
            .transpose()
            .context("error while loading workspace cache")?;

        Ok(Self {
            cache,
            workspace_handle,
            log_level,
            template_data,
            logger,
            jpath,
        })
    }
}

fn get_jpath(root: &Path) -> Result<HashSet<PathBuf>> {
    Ok(match File::open(root.join(JPATH_FILE)) {
        Ok(path) => {
            let reader = BufReader::new(path);
            reader
                .lines()
                .map(|line| Ok(line?))
                .collect::<Result<Vec<_>>>()
                .context("could not read lines")?
                .into_iter()
                .map(|line| {
                    let path = PathBuf::from(line);
                    let path = if path.is_relative() {
                        root.join(path)
                    } else {
                        path
                    };
                    match std::fs::metadata(&path) {
                        Ok(metadata) if metadata.is_dir() => {}
                        Ok(_) => return Ok(None),
                        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
                        Err(err) => return Err(err.into()),
                    };
                    Ok(Some(dunce::canonicalize(path)?))
                })
                .collect::<Result<HashSet<_>>>()
                .context("error while creating jpath")?
                .into_iter()
                .flatten()
                .collect()
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => HashSet::new(),
        Err(err) => return Err(err.into()),
    })
}

pub fn global_init(globals: &WorkspaceGlobals<'_>) -> Result<()> {
    clean_locks(globals.workspace_handle.inner().root()).context("error while cleaning locks")?;
    Ok(())
}
