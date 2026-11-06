mod command;
mod diagnostic;
mod metadata;

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

fn parse(path: &Path) -> Result<cartan_syntax::ast::Document, CliError> {
    let text = fs::read_to_string(path).map_err(|source| CliError::Read {
        path: path.to_owned(),
        source,
    })?;
    cartan_syntax::ast::parse_document(&text).map_err(|errors| CliError::Parse {
        path: path.to_owned(),
        text,
        errors,
    })
}

fn ir(path: &Path) -> Result<(), CliError> {
    output(format_args!("{:#?}", parse(path)?.items))
}

fn check(path: &Path) -> Result<(), CliError> {
    parse(path)?;
    output(format_args!("{}: valid syntax", path.display()))
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
    } else if let Some((name @ ("ir" | "check"), matches)) = matches.subcommand() {
        let path = matches.get_one::<PathBuf>("source").unwrap();
        if name == "ir" {
            ir(path)
        } else {
            check(path)
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
