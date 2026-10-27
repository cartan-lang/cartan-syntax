mod command;
mod diagnostic;
mod metadata;

use diagnostic::CliError;
use std::{env, fs, io, io::IsTerminal, io::Write, path::PathBuf, process::ExitCode};

fn output(value: impl std::fmt::Display) -> Result<(), CliError> {
    match writeln!(io::stdout().lock(), "{value}") {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => Ok(()),
        Err(source) => Err(CliError::Output { source }),
    }
}

fn run(path: PathBuf) -> Result<(), CliError> {
    let source = fs::read_to_string(&path).map_err(|source| CliError::Read {
        path: path.clone(),
        source,
    })?;
    let document = cartan_syntax::ast::parse_document(&source)
        .map_err(|errors| CliError::Parse { path, errors })?;
    output(format_args!("{:#?}", document.items))
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
    } else if let Some(("ir", matches)) = matches.subcommand() {
        run(matches.get_one::<PathBuf>("source").unwrap().clone())
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
