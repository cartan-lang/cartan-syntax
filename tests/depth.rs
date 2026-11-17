//! Nesting beyond `MAX_DEPTH` is a diagnostic on every entry point, never a
//! stack overflow. Each test runs on a thread with the 2 MB stack a test
//! thread has, in whatever build the tests are run in.

use cartan_syntax::ast::{self, Error};
use cartan_syntax::prompt::{completeness, highlight, Completeness};
use cartan_syntax::{DiagnosticKind, Span, MAX_DEPTH};

fn on_a_2_mb_stack(test: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(2 << 20)
        .spawn(test)
        .unwrap()
        .join()
        .unwrap();
}

/// `x = ` with `levels` of `open … close` around `1`. Its tree holds four
/// levels besides those: the document, the binding, the number and its token.
fn nested(open: &str, close: &str, levels: usize) -> String {
    format!("x = {}1{}", open.repeat(levels), close.repeat(levels))
}

/// The span of the one diagnostic, which says the text nests too deep.
fn too_deep(errors: &[Error]) -> Span {
    match errors {
        [Error::Grammar(diagnostic)]
            if diagnostic.kind == DiagnosticKind::TooDeep { limit: MAX_DEPTH } =>
        {
            diagnostic.span
        }
        _ => panic!("{errors:?}"),
    }
}

/// A tree exactly as deep as the limit parses, and one level more is refused
/// at the first token below the limit, for each construct that nests.
#[test]
fn nesting_parses_to_the_limit_and_is_refused_one_level_beyond() {
    on_a_2_mb_stack(|| {
        let levels = MAX_DEPTH - 4;
        for (open, close, below) in [
            ("(", ")", "1"),
            ("[", "]", "1"),
            ("{", "}", "1"),
            // the innermost condition is the first token below the limit
            ("if c then ", " else 2", "c"),
        ] {
            let at = nested(open, close, levels);
            if let Err(errors) = ast::parse_document(&at) {
                panic!("{open:?} at the limit: {errors:?}");
            }
            let beyond = nested(open, close, levels + 1);
            let span = too_deep(&ast::parse_document(&beyond).unwrap_err());
            assert_eq!(&beyond[span.start..span.end], below, "{open:?}");
        }
    });
}

/// Text far deeper than the limit is refused before the stack is exhausted:
/// a run of brackets the parser recurses into, and a chain of a
/// left-grouping operator, which it reads in a loop but which nests the tree
/// that every later walk recurses through.
#[test]
fn a_hundred_thousand_levels_are_refused_without_overflow() {
    on_a_2_mb_stack(|| {
        let parens = format!("{}1{}", "(".repeat(100_000), ")".repeat(100_000));
        too_deep(&ast::parse_document(&format!("x = {parens}")).unwrap_err());
        too_deep(&ast::parse_expression(&parens).unwrap_err());
        assert_eq!(completeness(&parens), Completeness::Invalid);
        // With no annotation context, highlighting needs only its lexical scan.
        assert_eq!(highlight(&parens).len(), 200_001);
        let sum = format!("x = 1{}", " + 1".repeat(100_000));
        let span = too_deep(&ast::parse_document(&sum).unwrap_err());
        assert_eq!(&sum[span.start..span.end], "1");
    });
}
