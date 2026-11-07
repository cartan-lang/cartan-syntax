mod command;
mod diagnostic;
mod metadata;
mod tree;

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
/// JSON mode its diagnostics are the JSON result rather than an error.
fn run(path: &Path, ast: bool, json: bool, colored: bool) -> Result<ExitCode, CliError> {
    let text = read(path)?;
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
    match (ast, json) {
        (false, false) => output(format_args!("{}: valid syntax", path.display()))?,
        (false, true) => output(json_text(&diagnostic::parse_json(path, &text, &[])))?,
        (true, false) => output(tree::render(path, &text, &syntax_tree(&document), colored))?,
        (true, true) => {
            let mut value = diagnostic::parse_json(path, &text, &[]);
            value["items"] = syntax_tree(&document);
            output(json_text(&value))?
        }
    }
    Ok(ExitCode::SUCCESS)
}

/// The source syntax tree's items as serialized data, the one source for both
/// the JSON and the readable view.
fn syntax_tree(document: &cartan_syntax::ast::Document) -> serde_json::Value {
    serde_json::to_value(&document.items).expect("the source AST serializes as JSON")
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
        match run(
            path,
            matches.get_flag("ast"),
            matches.get_flag("json"),
            colored,
        ) {
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
            ExitCode::FAILURE
        }
    }
}
