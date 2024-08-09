use std::path::Path;

use blaze_common::{configuration_file::ConfigurationFileFormat, error::Result};
use blaze_core::{init, GlobalOptions, InitOptions};
use clap::Parser;

use crate::subcommand::BlazeSubCommandExecution;

#[derive(Parser, Debug)]
#[command(
    display_name = "init",
    name = "init", 
    about("Initialize a new Blaze workspace."),
    long_about("This command will create Blaze configuration files in a new workspace and initialize git if not already initialized.
Here is the details of what is generated :
- A workspace configuration file
- A global variables file
- A user variables files
- `.gitignore` entries
- An example project
")
)]
pub struct InitCommand {
    #[arg(
        short = 'c',
        long = "create-directory",
        default_value_t,
        help = "Create the workspace root directory if it does not exist.",
        long_help = "Create the workspace root directory if it does not exist. It will recursively create parent directories when needed."
    )]
    create_directory: bool,

    #[arg(
        short = 'f',
        long = "format",
        help = "Generate configuration files using the specified format.",
        default_value_t
    )]
    format: ConfigurationFileFormat,

    #[arg(
        short = 'g',
        long = "no-git-init",
        default_value_t,
        help = "Do not initialize git if not already initialized.",
        long_help = "Do not initialize git if not already initialized. By default, the init command will initialize a new git repository if it does not already exist, this flag will disable this behavior."
    )]
    no_git_init: bool,

    #[arg(
        short = 'n',
        long = "name",
        help = "Specify name of the workspace.",
        long_help = "Specify name of the workspace, defaults to the workspace root directory name."
    )]
    name: Option<String>,
}

impl BlazeSubCommandExecution for InitCommand {
    fn execute(&self, root: &Path, globals: GlobalOptions) -> Result<()> {
        init(
            root,
            InitOptions {
                create_directory: self.create_directory,
                format: self.format,
                name: self.name.clone(),
                no_git: self.no_git_init,
            },
            globals,
        )
    }
}
