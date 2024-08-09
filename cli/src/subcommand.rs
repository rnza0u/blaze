use std::{
    fmt::{Debug, Display},
    path::Path,
    str::FromStr,
};

use anyhow::bail;
use blaze_common::error::{Error, Result};
use blaze_core::GlobalOptions;
use clap::{error::ErrorKind, ArgMatches, Args, FromArgMatches};
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

use crate::subcommands::{
    describe::DescribeCommand, init::InitCommand, render::RenderCommand, rm_cache::RmCacheCommand,
    run::RunCommand, spawn::SpawnCommand, version::VersionCommand,
};

pub trait BlazeSubCommandExecution: Debug {
    fn execute(&self, root: &Path, globals: GlobalOptions) -> Result<()>;
}

type DynSubCommandExecution = Box<dyn BlazeSubCommandExecution>;

#[derive(Debug)]
pub struct BlazeSubCommand(DynSubCommandExecution);

impl BlazeSubCommand {
    pub fn execute(&self, root: &Path, globals: GlobalOptions) -> Result<()> {
        self.0.execute(root, globals)
    }
}

impl BlazeSubCommand {
    fn try_new(kind: SubCommandKind, args: &ArgMatches) -> std::result::Result<Self, clap::Error> {
        Ok(match kind {
            SubCommandKind::Init => Self(Box::new(InitCommand::from_arg_matches(args)?)),
            SubCommandKind::Run => Self(Box::new(RunCommand::from_arg_matches(args)?)),
            SubCommandKind::Spawn => Self(Box::new(SpawnCommand::from_arg_matches(args)?)),
            SubCommandKind::Describe => Self(Box::new(DescribeCommand::from_arg_matches(args)?)),
            SubCommandKind::Version => Self(Box::new(VersionCommand::from_arg_matches(args)?)),
            SubCommandKind::Render => Self(Box::new(RenderCommand::from_arg_matches(args)?)),
            SubCommandKind::RmCache => Self(Box::new(RmCacheCommand::from_arg_matches(args)?)),
        })
    }
}

const INIT: &str = "init";
const RUN: &str = "run";
const SPAWN: &str = "spawn";
const DESCRIBE: &str = "describe";
const VERSION: &str = "version";
const RENDER: &str = "render";
const RM_CACHE: &str = "rm-cache";

#[derive(Debug, EnumIter)]
pub enum SubCommandKind {
    Init,
    Run,
    Spawn,
    Describe,
    Render,
    RmCache,
    Version,
}

impl SubCommandKind {
    pub fn command(&self) -> clap::Command {
        let augment_args = match self {
            Self::Init => InitCommand::augment_args,
            Self::Run => RunCommand::augment_args,
            Self::Spawn => SpawnCommand::augment_args,
            Self::Describe => DescribeCommand::augment_args,
            Self::Version => VersionCommand::augment_args,
            Self::Render => RenderCommand::augment_args,
            Self::RmCache => RmCacheCommand::augment_args,
        };
        augment_args(clap::Command::new(self.as_str()))
    }

    fn as_str(&self) -> &'static str {
        match self {
            Self::Init => INIT,
            Self::Run => RUN,
            Self::Spawn => SPAWN,
            Self::Describe => DESCRIBE,
            Self::Version => VERSION,
            Self::Render => RENDER,
            Self::RmCache => RM_CACHE,
        }
    }
}

impl Display for SubCommandKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for SubCommandKind {
    type Err = Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(match s {
            INIT => Self::Init,
            RUN => Self::Run,
            SPAWN => Self::Spawn,
            DESCRIBE => Self::Describe,
            VERSION => Self::Version,
            RENDER => Self::Render,
            RM_CACHE => Self::RmCache,
            _ => bail!("invalid sub command \"{s}\""),
        })
    }
}

impl FromArgMatches for BlazeSubCommand {
    fn from_arg_matches(matches: &clap::ArgMatches) -> std::result::Result<Self, clap::Error> {
        match matches.subcommand() {
            Some((name, args)) => Ok(BlazeSubCommand::try_new(
                SubCommandKind::from_str(name).map_err(|err| {
                    clap::Error::raw(ErrorKind::InvalidSubcommand, err.to_string())
                })?,
                args,
            )?),
            None => Err(clap::Error::raw(
                ErrorKind::MissingSubcommand,
                "subcommand is required",
            )),
        }
    }

    fn update_from_arg_matches(
        &mut self,
        _matches: &clap::ArgMatches,
    ) -> std::result::Result<(), clap::Error> {
        unimplemented!()
    }
}

fn augment(mut cmd: clap::Command) -> clap::Command {
    for kind in SubCommandKind::iter() {
        cmd = cmd.subcommand(kind.command());
    }
    cmd.subcommand_required(true)
}

impl clap::Subcommand for BlazeSubCommand {
    fn augment_subcommands(cmd: clap::Command) -> clap::Command {
        augment(cmd)
    }

    fn augment_subcommands_for_update(_: clap::Command) -> clap::Command {
        unimplemented!()
    }

    fn has_subcommand(name: &str) -> bool {
        SubCommandKind::from_str(name).is_ok()
    }
}
