//! The authoritative command tree, also used for discovery and color selection.

use clap::{builder::PathBufValueParser, Arg, ArgAction, ColorChoice, Command};
use std::ffi::OsString;

pub(crate) fn command() -> Command {
    Command::new("cartan-syntax")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Inspect Cartan source syntax")
        .arg(
            Arg::new("color")
                .long("color")
                .global(true)
                .value_parser(["auto", "always", "never"])
                .default_value("auto")
                .help("Control terminal colors"),
        )
        .arg(
            Arg::new("describe-cli")
                .long("describe-cli")
                .action(ArgAction::SetTrue)
                .help("Print versioned command metadata as JSON"),
        )
        .subcommand(
            Command::new("ir")
                .about("Print the normalized source AST and byte spans")
                .long_about("Print the normalized source AST and byte spans as indented Rust debug output. This inspection format is not typed computational IR or a stable interchange format.")
                .arg(
                    Arg::new("source")
                        .value_name("SOURCE")
                        .required(true)
                        .value_parser(PathBufValueParser::new())
                        .help("Cartan source file"),
                ),
        )
}

// Disable only eager presentation actions in a clone of the authoritative tree.
// Clap still determines which tokens are options, values and terminated operands.
fn presentation_probe(mut command: Command) -> Command {
    command.build();
    let eager: Vec<_> = command
        .get_arguments()
        .filter(|arg| {
            matches!(
                arg.get_action(),
                ArgAction::Help | ArgAction::HelpShort | ArgAction::HelpLong | ArgAction::Version
            )
        })
        .map(|arg| arg.get_id().clone())
        .collect();
    for id in eager {
        command = command.mut_arg(id, |arg| arg.action(ArgAction::SetTrue));
    }
    for child in command.get_subcommands_mut() {
        *child = presentation_probe(child.clone());
    }
    command.disable_help_subcommand(true).ignore_errors(true)
}

pub(crate) fn color(command: &Command, args: &[OsString]) -> ColorChoice {
    let matches = presentation_probe(command.clone())
        .try_get_matches_from(args)
        .ok();
    let selected = matches
        .as_ref()
        .and_then(|matches| matches.get_one::<String>("color"));
    let explicit = matches.as_ref().is_some_and(|matches| {
        matches.value_source("color") == Some(clap::parser::ValueSource::CommandLine)
    });
    match selected.map(String::as_str) {
        Some("always") => ColorChoice::Always,
        Some("never") => ColorChoice::Never,
        Some("auto") | None => {
            if !explicit && std::env::var_os("NO_COLOR").is_some_and(|value| !value.is_empty()) {
                ColorChoice::Never
            } else {
                ColorChoice::Auto
            }
        }
        _ => unreachable!("Clap validates the advertised color choices"),
    }
}

pub(crate) fn stream_color(choice: ColorChoice, terminal: bool) -> ColorChoice {
    match choice {
        ColorChoice::Auto if terminal => ColorChoice::Always,
        ColorChoice::Auto => ColorChoice::Never,
        value => value,
    }
}
