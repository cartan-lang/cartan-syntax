mod command;
mod diagnostic;
mod metadata;
mod tree;

use cartan_syntax::selection::{self, Selector};
use diagnostic::CliError;
use std::{
    env, fs, io,
    io::IsTerminal,
    io::Write,
    path::{Path, PathBuf},
    process::ExitCode,
};

fn output(value: impl std::fmt::Display) -> Result<(), CliError> {
    match writeln!(io::stdout().lock(), "{value}") {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => Ok(()),
        Err(source) => Err(CliError::Output { source }),
    }
}

fn read(path: &Path) -> Result<String, CliError> {
    fs::read_to_string(path).map_err(|source| CliError::Read {
        path: path.to_owned(),
        source,
    })
}

/// Check or print one source file. Invalid syntax fails with status 1; in
/// JSON mode its diagnostics are the JSON result rather than an error. The
/// whole document is always checked; a selector, given or the inspection
/// default `main`, must also name exactly one top-level item.
fn run(
    path: &Path,
    selector: Option<Selector>,
    ast: bool,
    json: bool,
    colored: bool,
) -> Result<ExitCode, CliError> {
    let text = match read(path) {
        Ok(text) => text,
        // JSON mode reports an unreadable source as its result, not on stderr.
        Err(CliError::Read { path, source }) if json => {
            output(json_text(&diagnostic::read_json(&path, &source)))?;
            return Ok(ExitCode::FAILURE);
        }
        Err(error) => return Err(error),
    };
    let document = match cartan_syntax::ast::parse_document(&text) {
        Ok(document) => document,
        Err(errors) if json => {
            output(json_text(&diagnostic::parse_json(path, &text, &errors)))?;
            return Ok(ExitCode::FAILURE);
        }
        Err(errors) => {
            return Err(CliError::Parse {
                path: path.to_owned(),
                text,
                errors,
            })
        }
    };
    let selector = selector.or_else(|| ast.then(Selector::default));
    let selected = match &selector {
        None => None,
        Some(selector) => match selection::select(&document, selector) {
            Ok(item) => Some((selector, item)),
            Err(error) if json => {
                output(json_text(&diagnostic::selection_json(path, &text, &error)))?;
                return Ok(ExitCode::FAILURE);
            }
            Err(error) => {
                return Err(CliError::Selection {
                    path: path.to_owned(),
                    text,
                    error,
                })
            }
        },
    };
    if json {
        let mut value = diagnostic::parse_json(path, &text, &[]);
        if let Some((selector, item)) = selected {
            value["selection"] = selection_json(selector, item);
            if ast {
                value["item"] = serialized(item);
            }
        }
        output(json_text(&value))?;
    } else {
        match selected {
            Some((selector, item)) if ast => {
                let heading = format!("{} {selector}", path.display());
                output(tree::render(&heading, &text, &serialized(item), colored))?
            }
            Some((selector, item)) => output(format_args!(
                "{}: valid syntax; {selector} at {}",
                path.display(),
                tree::position(&text, item.span.start, item.span.end)
            ))?,
            None => output(format_args!("{}: valid syntax", path.display()))?,
        }
    }
    Ok(ExitCode::SUCCESS)
}

/// One item of the source syntax tree as serialized data, the one source for
/// both the JSON and the readable view.
fn serialized(item: &cartan_syntax::ast::Item) -> serde_json::Value {
    serde_json::to_value(item).expect("the source AST serializes as JSON")
}

/// The selected item's identity: its selector, kind, update index and span.
fn selection_json(selector: &Selector, item: &cartan_syntax::ast::Item) -> serde_json::Value {
    let mut value = serde_json::json!({
        "selector": selector.to_string(),
        "kind": match selector {
            Selector::Binding(_) => "binding",
            Selector::Update(_) => "update",
        },
        "span": {"start": item.span.start, "end": item.span.end},
    });
    if let Selector::Update(index) = selector {
        value["index"] = (*index).into();
    }
    value
}

fn json_text(value: &serde_json::Value) -> String {
    serde_json::to_string_pretty(value).expect("a JSON value prints")
}

fn main() -> ExitCode {
    let args: Vec<_> = env::args_os().collect();
    let mut command = command::command();
    let color = command::color(&command, &args);
    command = command.color(command::stream_color(color, io::stdout().is_terminal()));
    let matches = match command.clone().try_get_matches_from(args) {
        Ok(matches) => matches,
        Err(error) => {
            let code = error.exit_code() as u8;
            let terminal = if error.use_stderr() {
                io::stderr().is_terminal()
            } else {
                io::stdout().is_terminal()
            };
            let command = command.color(command::stream_color(color, terminal));
            let _ = error.with_cmd(&command).print();
            return ExitCode::from(code);
        }
    };
    let result = if matches.get_flag("describe-cli") {
        output(metadata::describe(&mut command))
    } else if let Some(path) = matches.get_one::<PathBuf>("source") {
        let colored =
            command::stream_color(color, io::stdout().is_terminal()) == clap::ColorChoice::Always;
        let selector = matches
            .get_one::<String>("selection")
            .map(|text| Selector::parse(text).map_err(|error| CliError::Selector { error }))
            .transpose();
        match selector.and_then(|selector| {
            run(
                path,
                selector,
                matches.get_flag("ast"),
                matches.get_flag("json"),
                colored,
            )
        }) {
            Ok(code) => return code,
            Err(error) => Err(error),
        }
    } else {
        match command.print_help() {
            Err(source) if source.kind() == io::ErrorKind::BrokenPipe => Ok(()),
            result => result.map_err(|source| CliError::Output { source }),
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            let colored = match color {
                clap::ColorChoice::Always => true,
                clap::ColorChoice::Never => false,
                _ => io::stderr().is_terminal(),
            };
            diagnostic::print(&error, colored);
            ExitCode::from(error.exit_code())
        }
    }
}
