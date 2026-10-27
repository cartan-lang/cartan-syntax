//! Version-one discovery from the built Clap tree; never a second validator.

use clap::{Arg, ArgAction, Command};
use serde_json::{json, Value};

pub(crate) fn describe(command: &mut Command) -> Value {
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
        _ => unreachable!("unsupported Clap argument action"),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exports_repetition_unbounded_arity_and_only_visible_discovery() {
        let mut command = Command::new("example")
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
}
