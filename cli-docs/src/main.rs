use clap::{ArgAction, Args, Command};
use handlebars::no_escape;
use serde::Serialize;
use std::{error::Error, fs::File, path::Path};

const EXECUTABLE_NAME: &str = "blaze";
const MAIN_OUT_DIR_VAR: &str = "OUT_DIR";

type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn write_documentation(command: Command, at: &Path) -> Result<()> {
    let command_name = command.get_name().to_owned();

    if command.has_subcommands() {
        let subdir = if command_name.as_str() != "blaze" {
            let subdir = at.join(&command_name);
            std::fs::create_dir_all(&subdir)?;
            subdir
        } else {
            at.to_owned()
        };

        for subcommand in command.get_subcommands() {
            write_documentation(subcommand.clone(), &subdir)?;
        }
    }

    output_mdx_from_command(at, &command)?;
    output_man(at, command)?;
    // output_man_from_markdown(at, &command_name)?;

    Ok(())
}

fn output_man(root: &Path, command: Command) -> Result<()> {
    let man_file_path = root.join(format!("{}.man", command.get_name()));
    let man = clap_mangen::Man::new(command.clone());
    let mut buffer: Vec<u8> = Default::default();
    man.render(&mut buffer)?;
    std::fs::write(&man_file_path, buffer)?;
    Ok(())
}

#[derive(Debug, Serialize)]
struct ValueTemplateData {
    name: String,
    multiple: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OptionTemplateData {
    #[serde(skip_serializing_if = "Option::is_none")]
    short: Option<char>,
    long: String,
    help: String,
    required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<ValueTemplateData>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ArgumentTemplateData {
    required: bool,
    multiple: bool,
    help: String,
    name: String,
}

#[derive(Debug, Serialize)]
struct SubCommandTemplateData {
    name: String,
    link: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CommandTemplateData {
    name: String,
    about: String,
    long_about: String,
    options: Vec<OptionTemplateData>,
    arguments: Vec<ArgumentTemplateData>,
    subcommands: Vec<SubCommandTemplateData>,
}

impl TryFrom<&Command> for CommandTemplateData {
    type Error = Box<dyn Error>;

    fn try_from(command: &Command) -> Result<Self> {
        Ok(Self {
            name: command.get_name().to_owned(),
            about: command
                .get_about()
                .ok_or("about must be provided")?
                .to_string(),
            long_about: command
                .get_long_about()
                .ok_or("long about must be provided")?
                .to_string(),
            arguments: command
                .get_positionals()
                .map(|argument| {
                    Ok(ArgumentTemplateData {
                        help: argument
                            .get_long_help()
                            .or_else(|| argument.get_help())
                            .ok_or("could not get argument help/long help")?
                            .to_string(),
                        multiple: argument
                            .get_num_args()
                            .map(|num_args| num_args.max_values() > 1)
                            .unwrap_or_default(),
                        required: argument.is_required_set(),
                        name: argument
                            .get_value_names()
                            .unwrap()
                            .first()
                            .unwrap()
                            .to_string(),
                    })
                })
                .collect::<Result<_>>()?,
            options: command
                .get_opts()
                .map(|option| {
                    Ok(OptionTemplateData {
                        short: option.get_short(),
                        long: option
                            .get_long()
                            .map(str::to_owned)
                            .ok_or("long is not set")?,
                        required: option.is_required_set(),
                        value: matches!(option.get_action(), ArgAction::Set | ArgAction::Append)
                            .then(|| {
                                Ok::<_, Self::Error>(ValueTemplateData {
                                    name: option
                                        .get_value_names()
                                        .ok_or("could not get value names")?[0]
                                        .to_string(),
                                    multiple: matches!(option.get_action(), ArgAction::Append)
                                        && option.get_value_delimiter().is_none(),
                                })
                            })
                            .transpose()?,
                        help: option
                            .get_long_help()
                            .or_else(|| option.get_help())
                            .expect("could not get option help/long help")
                            .to_string(),
                    })
                })
                .collect::<Result<_>>()?,
            subcommands: command
                .get_subcommands()
                .map(|subcommand| SubCommandTemplateData {
                    name: subcommand.get_name().to_owned(),
                    link: if command.get_name() == "blaze" {
                        subcommand.get_name().to_owned()
                    } else {
                        format!("{}/{}", command.get_name(), subcommand.get_name())
                    },
                })
                .collect(),
        })
    }
}

fn output_mdx_from_command(root: &Path, command: &Command) -> Result<()> {
    let file = File::create(root.join(format!("{}.mdx", command.get_name())))?;
    let mut generator = handlebars::Handlebars::new();
    let data = CommandTemplateData::try_from(command)?;
    generator.register_template_string(
        "argument-identifier",
        include_str!("./argument-identifier.handlebars"),
    )?;
    generator.register_template_string(
        "option-identifier",
        include_str!("./option-identifier.handlebars"),
    )?;
    generator.register_escape_fn(no_escape);
    generator.render_template_to_write(include_str!("./command.handlebars"), &data, file)?;

    Ok(())
}

fn main() -> Result<()> {
    let out_dir = std::path::PathBuf::from(std::env::var(MAIN_OUT_DIR_VAR)?);

    match std::fs::remove_dir_all(&out_dir) {
        Err(err) if err.kind() != std::io::ErrorKind::NotFound => return Err(err.into()),
        _ => {}
    };
    std::fs::create_dir_all(&out_dir)?;

    write_documentation(
        blaze_cli::command::Command::augment_args(clap::Command::new(EXECUTABLE_NAME)),
        &out_dir,
    )
}
