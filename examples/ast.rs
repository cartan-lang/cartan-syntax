use std::{env, fs, process::ExitCode};

fn main() -> ExitCode {
    let mut args = env::args_os().skip(1);
    let Some(path) = args.next() else {
        eprintln!("usage: cargo run --example ast -- <source.cart>");
        return ExitCode::FAILURE;
    };
    if args.next().is_some() {
        eprintln!("expected one source file");
        return ExitCode::FAILURE;
    }
    let source = match fs::read_to_string(&path) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("{}: {error}", std::path::Path::new(&path).display());
            return ExitCode::FAILURE;
        }
    };
    match cartan_syntax::ast::parse_document(&source) {
        Ok(document) => {
            println!("{:#?}", document.items);
            ExitCode::SUCCESS
        }
        Err(errors) => {
            eprintln!("{errors:#?}");
            ExitCode::FAILURE
        }
    }
}
