//! The authoritative command tree, also used for discovery and color selection.

use clap::{builder::PathBufValueParser, Arg, ArgAction, ColorChoice, Command};
use std::ffi::OsString;

pub(crate) fn command() -> Command {
    // Help and version are explicit so the color probe can edit them unbuilt.
    with_help(tree(), &[])
        .disable_help_subcommand(true)
        .disable_version_flag(true)
        .arg(
            Arg::new("version")
                .short('V')
                .long("version")
                .action(ArgAction::Version)
                .help("Print version"),
        )
}

fn tree() -> Command {
    Command::new("cartan-syntax")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Inspect Cartan source syntax")
        .arg(
            Arg::new("color")
                .long("color")
                .value_name("WHEN")
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

/// Give every command `-h` for concise help and `--help` for its full outline.
fn with_help(command: Command, parents: &[&str]) -> Command {
    let mut outline = String::from("Command outline:\n");
    write_outline(&command, parents, &mut outline);
    let mut path = parents.to_vec();
    path.push(command.get_name());
    let path: Vec<String> = path.into_iter().map(str::to_owned).collect();
    let mut command = command
        .disable_help_flag(true)
        .arg(
            Arg::new("help-short")
                .short('h')
                .action(ArgAction::HelpShort)
                .help("Print concise help"),
        )
        .arg(
            Arg::new("help-full")
                .long("help")
                .action(ArgAction::HelpLong)
                .help("Print help with the full command outline"),
        )
        .after_long_help(outline.trim_end().to_owned());
    let path: Vec<&str> = path.iter().map(String::as_str).collect();
    for child in command.get_subcommands_mut() {
        *child = with_help(child.clone(), &path);
    }
    command
}

/// One line per command path, then its visible arguments, depth first.
fn write_outline(command: &Command, parents: &[&str], out: &mut String) {
    let mut path = parents.to_vec();
    path.push(command.get_name());
    let about = command
        .get_about()
        .map(|about| format!("  {about}"))
        .unwrap_or_default();
    out.push_str(&format!("  {}{about}\n", path.join(" ")));
    // Help/version entries and globals inherited below the root are omitted;
    // the Python outline derived from metadata applies the same rules.
    let shown = |arg: &&Arg| {
        !arg.is_hide_set()
            && !matches!(
                arg.get_action(),
                ArgAction::Help | ArgAction::HelpShort | ArgAction::HelpLong | ArgAction::Version
            )
            && (parents.is_empty() || !arg.is_global_set())
    };
    for arg in command.get_arguments().filter(shown) {
        let name = arg
            .get_value_names()
            .and_then(|names| names.first())
            .map(|name| name.to_string())
            .unwrap_or_else(|| arg.get_id().as_str().to_uppercase());
        let takes_value = matches!(arg.get_action(), ArgAction::Set | ArgAction::Append);
        let repeat = if matches!(arg.get_action(), ArgAction::Append | ArgAction::Count) {
            "..."
        } else {
            ""
        };
        let spelling = match (arg.get_long(), arg.get_short()) {
            _ if arg.is_positional() && arg.is_required_set() => format!("<{name}>"),
            _ if arg.is_positional() => format!("[{name}]"),
            (Some(long), _) if takes_value => format!("--{long} <{name}>"),
            (Some(long), _) => format!("--{long}"),
            (None, Some(short)) if takes_value => format!("-{short} <{name}>"),
            (None, Some(short)) => format!("-{short}"),
            (None, None) => name,
        };
        let mut notes = Vec::new();
        let defaults: Vec<_> = arg
            .get_default_values()
            .iter()
            .map(|value| value.to_string_lossy())
            .collect();
        if takes_value && !defaults.is_empty() {
            notes.push(format!("default: {}", defaults.join(", ")));
        }
        let choices: Vec<_> = arg
            .get_possible_values()
            .into_iter()
            .filter(|value| !value.is_hide_set())
            .map(|value| value.get_name().to_owned())
            .collect();
        if takes_value && !choices.is_empty() {
            notes.push(format!("choices: {}", choices.join(", ")));
        }
        let help = arg.get_help().map(ToString::to_string).unwrap_or_default();
        let notes = if notes.is_empty() {
            String::new()
        } else {
            format!(" [{}]", notes.join("; "))
        };
        out.push_str(&format!("      {spelling}{repeat}  {help}{notes}\n"));
    }
    for child in command
        .get_subcommands()
        .filter(|child| !child.is_hide_set())
    {
        write_outline(child, &path, out);
    }
}

// Disable only eager presentation actions in a clone of the authoritative tree.
// Clap still determines which tokens are options, values and terminated operands.
// Every eager argument is explicit, so the probe edits the unbuilt tree: editing
// a built Clap command does not refresh its option lookup.
fn presentation_probe(mut command: Command) -> Command {
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
    // A help flag must not lose earlier matches to a missing required operand.
    let required: Vec<_> = command
        .get_arguments()
        .filter(|arg| arg.is_required_set())
        .map(|arg| arg.get_id().clone())
        .collect();
    for id in required {
        command = command.mut_arg(id, |arg| arg.required(false));
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The Python outline in tests/python/test_metadata.py expects these lines.
    /// Argument relationships such as `requires_if` stay in Rust validation.
    #[test]
    fn outline_orders_nested_paths_positionals_repeats_defaults_and_choices() {
        let command = Command::new("tool")
            .about("A tool")
            .arg(
                Arg::new("param")
                    .long("param")
                    .value_name("NAME=VALUE")
                    .action(ArgAction::Append)
                    .help("Static parameter"),
            )
            .subcommand(
                Command::new("emit")
                    .about("Emit code")
                    .arg(
                        Arg::new("target")
                            .value_name("TARGET")
                            .required(true)
                            .value_parser(["metal", "cuda", "wasm"])
                            .help("Target"),
                    )
                    .arg(Arg::new("source").value_name("SOURCE").help("Source file")),
            )
            .subcommand(
                Command::new("wasm").about("Wasm tools").subcommand(
                    Command::new("validate").subcommand(
                        Command::new("diffusion").about("Validate diffusion").arg(
                            Arg::new("offset")
                                .long("offset")
                                .value_name("X")
                                .default_value("0.5")
                                .help("Offset"),
                        ),
                    ),
                ),
            );
        let mut out = String::new();
        write_outline(&command, &[], &mut out);
        assert_eq!(
            out,
            "  tool  A tool\n\
             \x20     --param <NAME=VALUE>...  Static parameter\n\
             \x20 tool emit  Emit code\n\
             \x20     <TARGET>  Target [choices: metal, cuda, wasm]\n\
             \x20     [SOURCE]  Source file\n\
             \x20 tool wasm  Wasm tools\n\
             \x20 tool wasm validate\n\
             \x20 tool wasm validate diffusion  Validate diffusion\n\
             \x20     --offset <X>  Offset [default: 0.5]\n"
        );
    }
}
