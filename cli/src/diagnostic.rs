//! The CLI's sole diagnostic formatter. Operations retain typed error facts.

use std::{error::Error, fmt, io, path::PathBuf};

#[derive(Debug)]
pub(crate) enum CliError {
    Read {
        path: PathBuf,
        source: io::Error,
    },
    /// `text` is the exact source the errors' byte spans index.
    Parse {
        path: PathBuf,
        text: String,
        errors: Vec<cartan_syntax::ast::Error>,
    },
    Output {
        source: io::Error,
    },
}

impl fmt::Display for CliError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { path, source } => write!(formatter, "{}: {source}", path.display()),
            Self::Parse { path, text, errors } => {
                let sources = [cartan_diagnostics::Source { name: path, text }];
                let reports: Vec<String> = errors
                    .iter()
                    .map(|error| {
                        cartan_diagnostics::render(
                            &sources,
                            &cartan_syntax_diagnostics::syntax(error, 0),
                        )
                    })
                    .collect();
                formatter.write_str(&reports.join("\n"))
            }
            Self::Output { source } => write!(formatter, "stdout: {source}"),
        }
    }
}

impl Error for CliError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Read { source, .. } | Self::Output { source } => Some(source),
            Self::Parse { .. } => None,
        }
    }
}

/// The JSON result of checking one source: whether its syntax is valid and
/// each diagnostic in the shared JSON envelope, with exact byte spans.
pub(crate) fn parse_json(
    path: &std::path::Path,
    text: &str,
    errors: &[cartan_syntax::ast::Error],
) -> serde_json::Value {
    let sources = [cartan_diagnostics::Source { name: path, text }];
    let diagnostics: Vec<_> = errors
        .iter()
        .map(|error| {
            cartan_diagnostics::to_json(&sources, &cartan_syntax_diagnostics::syntax(error, 0))
        })
        .collect();
    serde_json::json!({
        "source": path.to_string_lossy(),
        "valid": errors.is_empty(),
        "diagnostics": diagnostics,
    })
}

/// Render domain errors only at the application's presentation boundary. A
/// rendered report already names its severity and code, as downstream tools
/// print it; other errors take an `error:` prefix.
pub(crate) fn print(error: &CliError, colored: bool) {
    let text = error.to_string();
    match (text.strip_prefix("error["), colored) {
        (Some(rest), true) => eprintln!("\x1b[1;31merror\x1b[0m[{rest}"),
        (Some(_), false) => eprintln!("{text}"),
        (None, true) => eprintln!("\x1b[1;31merror:\x1b[0m {text}"),
        (None, false) => eprintln!("error: {text}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_and_preserves_io_causes() {
        let error = CliError::Read {
            path: PathBuf::from("source.cart"),
            source: io::Error::from(io::ErrorKind::NotFound),
        };
        let CliError::Read { source, .. } = &error else {
            panic!("expected typed read error");
        };
        assert_eq!(source.kind(), io::ErrorKind::NotFound);
        assert_eq!(error.to_string(), format!("source.cart: {source}"));
        assert!(error.source().unwrap().is::<io::Error>());
        let error = CliError::Output {
            source: io::Error::from(io::ErrorKind::PermissionDenied),
        };
        assert!(error.to_string().starts_with("stdout: "));
        assert!(error.source().unwrap().is::<io::Error>());
    }

    #[test]
    fn renders_each_parser_error_as_a_located_report() {
        let text = "param n =";
        let errors = cartan_syntax::ast::parse_document(text).unwrap_err();
        let count = errors.len();
        let error = CliError::Parse {
            path: PathBuf::from("source.cart"),
            text: text.into(),
            errors,
        };
        let rendered = error.to_string();
        assert!(rendered.starts_with("error[syntax."));
        assert_eq!(rendered.matches("error[").count(), count);
        assert!(rendered.contains("source.cart:1:"));
        assert!(rendered.contains("found end of file"));
    }
}
