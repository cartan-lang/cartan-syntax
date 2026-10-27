//! The CLI's sole diagnostic formatter. Operations retain typed error facts.

use std::{error::Error, fmt, io, path::PathBuf};

#[derive(Debug)]
pub(crate) enum CliError {
    Read {
        path: PathBuf,
        source: io::Error,
    },
    Parse {
        path: PathBuf,
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
            Self::Parse { path, errors } => write!(formatter, "{}: {errors:#?}", path.display()),
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

/// Render domain errors only at the application's presentation boundary.
pub(crate) fn print(error: &CliError, colored: bool) {
    if colored {
        eprintln!("\x1b[1;31merror:\x1b[0m {error}");
    } else {
        eprintln!("error: {error}");
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
    fn formats_structured_parser_facts_at_the_boundary() {
        let errors = cartan_syntax::ast::parse_document("param n =").unwrap_err();
        let error = CliError::Parse {
            path: PathBuf::from("source.cart"),
            errors,
        };
        let text = error.to_string();
        assert!(text.starts_with("source.cart: "));
        assert!(text.contains("Grammar("));
        assert!(text.contains("incomplete: true"));
        assert!(text.contains("Span {"));
    }
}
