//! The conventions every Cartan component CLI shares, over a Clap command tree:
//! `-h` and `--help` with a generated command outline, `--describe-cli`
//! discovery, the `--color` option, and the `--json` result envelope.
//!
//! A component owns its command tree and its wording. Build the tree, pass it
//! through [`with_help`], and call the rest at the application boundary.

use cartan_diagnostics::Report;
use clap::{error::ContextKind, Arg, ArgAction, ColorChoice, Command};
use serde_json::{json, Map, Value};
use std::ffi::OsString;
use std::path::Path;

/// Give `command` and every subcommand `-h` for concise help and `--help` for
/// help ending in its full outline. A command's after-help stays in `-h` and
/// stands above the outline in `--help`.
pub fn with_help(command: Command) -> Command {
    help_at(command, &[])
}

fn help_at(command: Command, parents: &[&str]) -> Command {
    let mut outline = String::from("Command outline:\n");
    write_outline(&command, parents, &mut outline);
    let mut path = parents.to_vec();
    path.push(command.get_name());
    let path: Vec<String> = path.into_iter().map(str::to_owned).collect();
    // Long help replaces a short after-help note, so keep it ahead of the outline.
    let after = command
        .get_after_long_help()
        .or(command.get_after_help())
        .map(|note| format!("{note}\n\n"))
        .unwrap_or_default();
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
        .after_long_help(format!("{after}{}", outline.trim_end()));
    let path: Vec<&str> = path.iter().map(String::as_str).collect();
    for child in command.get_subcommands_mut() {
        *child = help_at(child.clone(), &path);
    }
    command
}

/// The full command outline: each command path depth first, then its visible
/// arguments in definition order.
pub fn outline(command: &Command) -> String {
    let mut text = String::from("Command outline:\n");
    write_outline(command, &[], &mut text);
    text.trim_end().to_owned()
}

fn write_outline(command: &Command, parents: &[&str], out: &mut String) {
    let mut path = parents.to_vec();
    path.push(command.get_name());
    let about = command
        .get_about()
        .map(|about| format!("  {about}"))
        .unwrap_or_default();
    out.push_str(&format!("  {}{about}\n", path.join(" ")));
    // Help/version entries and globals inherited below the root are omitted.
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
        let repeat = if matches!(arg.get_action(), ArgAction::Append | ArgAction::Count)
            || arg
                .get_num_args()
                .is_some_and(|arity| arity.max_values() > 1)
        {
            "..."
        } else {
            ""
        };
        let spelling = match (arg.get_long(), arg.get_short()) {
            _ if arg.is_positional() && arg.is_required_set() => format!("<{name}>"),
            _ if arg.is_positional() => format!("[{name}]"),
            (Some(long), Some(short)) if takes_value => format!("-{short}, --{long} <{name}>"),
            (Some(long), Some(short)) => format!("-{short}, --{long}"),
            (Some(long), None) if takes_value => format!("--{long} <{name}>"),
            (Some(long), None) => format!("--{long}"),
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

/// Version-one `--describe-cli` discovery from the built tree; never a second
/// validator.
pub fn describe(command: &mut Command) -> Value {
    command.build();
    json!({
        "schema_version": 1,
        "program": command.get_name(),
        "version": command.get_version(),
        "command": node(command),
    })
}

fn node(command: &Command) -> Value {
    json!({
        "name": command.get_name(),
        "description": command.get_about().map(ToString::to_string),
        // The command's after-help prose, without the generated outline.
        "notes": command.get_after_help().map(ToString::to_string),
        "aliases": command.get_visible_aliases().collect::<Vec<_>>(),
        "arguments": command.get_arguments().filter(|arg| !arg.is_hide_set()).map(argument).collect::<Vec<_>>(),
        "commands": command.get_subcommands().filter(|child| !child.is_hide_set()).map(node).collect::<Vec<_>>(),
    })
}

fn argument(arg: &Arg) -> Value {
    let arity = arg.get_num_args().expect("command tree is built");
    let action = match arg.get_action() {
        ArgAction::Set => "set",
        ArgAction::Append => "append",
        ArgAction::SetTrue => "set_true",
        ArgAction::SetFalse => "set_false",
        ArgAction::Count => "count",
        ArgAction::Help => "help",
        ArgAction::HelpShort => "help_short",
        ArgAction::HelpLong => "help_long",
        ArgAction::Version => "version",
        _ => unreachable!("new Clap action requires schema review"),
    };
    let choices: Vec<_> = arg
        .get_value_parser()
        .possible_values()
        .into_iter()
        .flatten()
        .filter(|value| !value.is_hide_set())
        .map(|value| value.get_name().to_owned())
        .collect();
    json!({
        "id": arg.get_id().as_str(),
        "description": arg.get_help().map(ToString::to_string),
        "long": arg.get_long(),
        "short": arg.get_short().map(|value| value.to_string()),
        "aliases": arg.get_visible_aliases().unwrap_or_default(),
        "short_aliases": arg.get_visible_short_aliases().unwrap_or_default().iter().map(ToString::to_string).collect::<Vec<_>>(),
        "position": arg.get_index(),
        "value_names": arg.get_value_names().unwrap_or_default().iter().map(|value| value.as_str()).collect::<Vec<_>>(),
        "arity": {"min": arity.min_values(), "max": (arity.max_values() != usize::MAX).then_some(arity.max_values())},
        "action": action,
        "repeatable": matches!(arg.get_action(), ArgAction::Append | ArgAction::Count),
        "required": arg.is_required_set(),
        "global": arg.is_global_set(),
        "default_values": arg.get_default_values().iter().map(|value| value.to_string_lossy()).collect::<Vec<_>>(),
        "choices": choices,
    })
}

/// The global `--color WHEN` option [`color`] reads.
pub fn color_arg() -> Arg {
    Arg::new("color")
        .long("color")
        .value_name("WHEN")
        .global(true)
        .value_parser(["auto", "always", "never"])
        .default_value("auto")
        .help("Control terminal colors")
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

/// The color choice in `args`, even when they are otherwise invalid. An
/// explicit `--color` wins; automatic color yields to a non-empty `NO_COLOR`.
pub fn color(command: &Command, args: &[OsString]) -> ColorChoice {
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

/// Resolve automatic color for one output stream.
pub fn stream_color(choice: ColorChoice, terminal: bool) -> ColorChoice {
    match choice {
        ColorChoice::Auto if terminal => ColorChoice::Always,
        ColorChoice::Auto => ColorChoice::Never,
        value => value,
    }
}

/// Whether `--json` was requested at any level of the matched command path,
/// even when the arguments are otherwise invalid.
pub fn json_requested(command: &Command, args: &[OsString]) -> bool {
    let Ok(matches) = presentation_probe(command.clone()).try_get_matches_from(args) else {
        return false;
    };
    let mut level = &matches;
    loop {
        // An aborted probe may lack the flag's default, so read it fallibly.
        if level.try_get_one::<bool>("json").ok().flatten() == Some(&true) {
            return true;
        }
        match level.subcommand() {
            Some((_, child)) => level = child,
            None => return false,
        }
    }
}

/// The `cli.usage` diagnostic for invalid arguments: Clap's rendered error,
/// its kind and the offending argument, if any.
pub fn usage_report(error: &clap::Error) -> Report {
    let mut report = Report::error("cli.usage", error.render().to_string(), None);
    let argument = error.get(ContextKind::InvalidArg).map(ToString::to_string);
    report.fields = json!({"kind": format!("{:?}", error.kind()), "argument": argument});
    report
}

/// Whether a `--json` result succeeded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Ok,
    Error,
}

/// The `--json` result envelope: `schema_version`, `status`, `source` and
/// `diagnostics`. A component adds its payload keys to the returned map.
pub fn result(
    status: Status,
    source: Option<&Path>,
    diagnostics: Vec<Value>,
) -> Map<String, Value> {
    let status = match status {
        Status::Ok => "ok",
        Status::Error => "error",
    };
    let mut map = Map::new();
    map.insert("schema_version".into(), json!(1));
    map.insert("status".into(), json!(status));
    map.insert(
        "source".into(),
        json!(source.map(|path| path.to_string_lossy())),
    );
    map.insert("diagnostics".into(), Value::Array(diagnostics));
    map
}

#[cfg(test)]
mod tests {
    use super::*;

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
            .arg(
                Arg::new("output")
                    .short('o')
                    .long("output")
                    .value_name("PATH")
                    .help("Output"),
            )
            .arg(
                Arg::new("set")
                    .long("set")
                    .value_name("NAME=VALUE")
                    .num_args(1..)
                    .help("Assignments"),
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
        assert_eq!(
            outline(&command),
            "Command outline:\n\
             \x20 tool  A tool\n\
             \x20     --param <NAME=VALUE>...  Static parameter\n\
             \x20     -o, --output <PATH>  Output\n\
             \x20     --set <NAME=VALUE>...  Assignments\n\
             \x20 tool emit  Emit code\n\
             \x20     <TARGET>  Target [choices: metal, cuda, wasm]\n\
             \x20     [SOURCE]  Source file\n\
             \x20 tool wasm  Wasm tools\n\
             \x20 tool wasm validate\n\
             \x20 tool wasm validate diffusion  Validate diffusion\n\
             \x20     --offset <X>  Offset [default: 0.5]"
        );
    }

    #[test]
    fn exports_repetition_unbounded_arity_and_only_visible_discovery() {
        let mut command = Command::new("example")
            .after_help("A note")
            .arg(
                Arg::new("watch")
                    .long("watch")
                    .visible_alias("observe")
                    .alias("secret")
                    .action(ArgAction::Append)
                    .num_args(3..),
            )
            .arg(Arg::new("verbose").short('v').action(ArgAction::Count))
            .arg(Arg::new("hidden").long("hidden").hide(true))
            .subcommand(Command::new("hidden-command").hide(true));
        let value = describe(&mut command);
        assert_eq!(value["command"]["notes"], "A note");
        let args = value["command"]["arguments"].as_array().unwrap();
        let watch = args.iter().find(|arg| arg["id"] == "watch").unwrap();
        assert_eq!(watch["arity"], json!({"min":3,"max":null}));
        assert_eq!(watch["action"], "append");
        assert_eq!(watch["repeatable"], true);
        assert_eq!(watch["aliases"], json!(["observe"]));
        let verbose = args.iter().find(|arg| arg["id"] == "verbose").unwrap();
        assert_eq!(verbose["arity"], json!({"min":0,"max":0}));
        assert_eq!(verbose["action"], "count");
        assert_eq!(verbose["repeatable"], true);
        assert!(args.iter().all(|arg| arg["id"] != "hidden"));
        assert!(value["command"]["commands"]
            .as_array()
            .unwrap()
            .iter()
            .all(|child| child["name"] != "hidden-command"));
    }

    #[test]
    fn json_is_found_on_the_matched_subcommand_despite_invalid_arguments() {
        let command = with_help(
            Command::new("tool").arg(color_arg()).subcommand(
                Command::new("inspect")
                    .arg(Arg::new("json").long("json").action(ArgAction::SetTrue))
                    .arg(Arg::new("module").required(true)),
            ),
        );
        let requested = |args: &[&str]| {
            let args: Vec<OsString> = args.iter().map(OsString::from).collect();
            json_requested(&command, &args)
        };
        assert!(requested(&["tool", "inspect", "--json"]));
        assert!(requested(&["tool", "inspect", "--json", "--unknown"]));
        assert!(!requested(&["tool", "inspect", "m"]));
    }

    #[test]
    fn usage_errors_and_results_keep_their_typed_fields() {
        let error = Command::new("tool")
            .try_get_matches_from(["tool", "--unknown"])
            .unwrap_err();
        let report = usage_report(&error);
        assert_eq!(report.code, "cli.usage");
        assert_eq!(report.fields["kind"], "UnknownArgument");
        assert_eq!(report.fields["argument"], "--unknown");
        let map = result(Status::Error, Some(Path::new("a.cart")), vec![]);
        assert_eq!(map["schema_version"], 1);
        assert_eq!(map["status"], "error");
        assert_eq!(map["source"], "a.cart");
        assert_eq!(map["diagnostics"], json!([]));
    }
}
