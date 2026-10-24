mod diagnostic;

use diagnostic::CliError;
use std::{env, fs, io, io::Write, path::PathBuf, process::ExitCode};

const HELP: &str = "Cartan syntax inspection

Usage:
  cartan-syntax ast <source.cart>
  cartan-syntax --help
  cartan-syntax --version

Print the normalized source AST and byte spans as indented Rust debug output.
This is an inspection format, not typed computational IR or a stable interchange format.";

fn output(value: impl std::fmt::Display) -> Result<(), CliError> {
    match writeln!(io::stdout().lock(), "{value}") {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => Ok(()),
        Err(source) => Err(CliError::Output { source }),
    }
}

fn run() -> Result<(), CliError> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    let path = match args.as_slice() {
        [] => return output(HELP),
        [arg] if arg == "--help" || arg == "-h" => return output(HELP),
        [arg] if arg == "--version" => {
            return output(format_args!("cartan-syntax {}", env!("CARGO_PKG_VERSION")));
        }
        [command, path] if command == "ast" => PathBuf::from(path),
        _ => return Err(CliError::Usage),
    };
    let source = fs::read_to_string(&path).map_err(|source| CliError::Read {
        path: path.clone(),
        source,
    })?;
    let document = cartan_syntax::ast::parse_document(&source)
        .map_err(|errors| CliError::Parse { path, errors })?;
    output(format_args!("{:#?}", document.items))
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
