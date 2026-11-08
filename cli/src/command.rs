//! The authoritative command tree, also used for discovery and color selection.
//! Help, outline, discovery and color conventions come from `cartan_cli_support`.

use clap::{builder::PathBufValueParser, Arg, ArgAction, Command};

pub(crate) fn command() -> Command {
    // Help and version are explicit so the color probe can edit them unbuilt.
    cartan_cli_support::with_help(tree())
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
        .about("Check and inspect Cartan source syntax")
        .after_help("SOURCE checks a file's syntax, and BINDING or UPDATE (update:N; update means update:0) also checks that it names one top-level item; --ast prints that item's syntax tree, main by default, and --json prints JSON. Under --json every outcome, including usage and I/O failures, is a JSON result on stdout.")
        .long_about("Check a Cartan source file's syntax, or print its source syntax tree. The syntax tree describes written constructs with byte spans; it is not typed computational IR. Success claims syntax only, with no name resolution, typing or execution.")
        .arg(cartan_cli_support::color_arg())
        .arg(
            Arg::new("describe-cli")
                .long("describe-cli")
                .action(ArgAction::SetTrue)
                .help("Print versioned command metadata as JSON"),
        )
        .arg(
            Arg::new("source")
                .value_name("SOURCE")
                .value_parser(PathBufValueParser::new())
                .help("Cartan source file to check"),
        )
        .arg(
            Arg::new("selection")
                .value_name("BINDING|UPDATE")
                .requires("source")
                .help("Top-level binding, or update block as update:N (update means update:0); --ast selects main by default"),
        )
        .arg(
            Arg::new("ast")
                .long("ast")
                .action(ArgAction::SetTrue)
                .requires("source")
                .help("Print the source syntax tree instead of checking"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .action(ArgAction::SetTrue)
                .requires("source")
                .help("Print JSON instead of text"),
        )
}
