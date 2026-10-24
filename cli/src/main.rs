use std::{env, fs, io, io::Write, path::Path, process::ExitCode};

const HELP: &str = "Cartan syntax inspection

Usage:
  cartan-syntax ast <source.cart>
  cartan-syntax --help
  cartan-syntax --version

Print the normalized source AST and byte spans as indented Rust debug output.
This is an inspection format, not typed computational IR or a stable interchange format.";

fn output(value: impl std::fmt::Display) -> ExitCode {
    match writeln!(io::stdout().lock(), "{value}") {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("stdout: {error}");
            ExitCode::FAILURE
        }
    }
}

fn main() -> ExitCode {
    let args: Vec<_> = env::args_os().skip(1).collect();
    let path = match args.as_slice() {
        [arg] if arg == "--help" || arg == "-h" => return output(HELP),
        [arg] if arg == "--version" => {
            return output(format_args!("cartan-syntax {}", env!("CARGO_PKG_VERSION")));
        }
        [command, path] if command == "ast" => path,
        _ => {
            eprintln!("{HELP}");
            return ExitCode::FAILURE;
        }
    };
    let source = match fs::read_to_string(path) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("{}: {error}", Path::new(path).display());
            return ExitCode::FAILURE;
        }
    };
    match cartan_syntax::ast::parse_document(&source) {
        Ok(document) => output(format_args!("{:#?}", document.items)),
        Err(errors) => {
            eprintln!("{errors:#?}");
            ExitCode::FAILURE
        }
    }
}
