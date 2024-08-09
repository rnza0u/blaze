use std::path::Path;

use blaze_common::error::Result;
use blaze_core::GlobalOptions;
use build_time::build_time_utc;
use clap::{crate_name, crate_version, Parser};

use crate::subcommand::BlazeSubCommandExecution;

#[derive(Parser, Debug)]
#[command(
    display_name = "version",
    name = "version",
    about = "Display version.",
    long_about = "Display Blaze version and build information."
)]
pub struct VersionCommand;

impl BlazeSubCommandExecution for VersionCommand {
    fn execute(&self, _root: &Path, _globals: GlobalOptions) -> Result<()> {
        println!(
            "{} v{}, built at: {}",
            crate_name!(),
            crate_version!(),
            build_time_utc!("%Y-%m-%d %H:%M:%S (UTC)")
        );
        Ok(())
    }
}
