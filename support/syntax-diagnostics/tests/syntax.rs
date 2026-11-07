use cartan_diagnostics::{render, Source};
use cartan_syntax::ast;
use cartan_syntax_diagnostics::syntax;
use std::path::Path;

#[test]
fn syntax_eof_has_stable_facts_and_plain_output() {
    let text = "main = (";
    let error = ast::parse_document(text).unwrap_err().remove(0);
    let report = syntax(&error, 0);
    let output = render(
        &[Source {
            name: Path::new("broken.cart"),
            text,
        }],
        &report,
    );
    assert!(report.code.starts_with("syntax."));
    assert!(output.contains("broken.cart:1:"));
    assert!(output.contains('^'));
    assert!(!output.contains('\u{1b}'));
    assert!(!output.contains("Grammar"));
    assert_eq!(report.fields["incomplete"], true);
}

#[test]
fn stated_rules_and_related_spans_keep_their_identity() {
    let text = "require(a, 3)";
    let report = syntax(&ast::parse_document(text).unwrap_err()[0], 2);
    assert_eq!(report.code, "syntax.require_message");
    let primary = report.primary.unwrap().location;
    assert_eq!(
        (primary.source, primary.span.start),
        (2, text.find('3').unwrap())
    );
    assert_eq!(report.fields["refused"], false);

    let text = "m = match s { A then 1, else 2, else 3 }";
    let report = syntax(&ast::parse_document(text).unwrap_err()[0], 0);
    assert_eq!(report.code, "syntax.duplicate_else");
    let first = &report.related[0].location.span;
    assert_eq!(&text[first.start..first.end], "else 2");
}

/// The owner-approved wording for a representative error of each family. A
/// token-set summary that stops matching the grammar falls back to
/// `unexpected …` and fails here rather than printing a wrong expectation.
#[test]
fn messages_name_what_was_expected_and_found() {
    for (text, code, message, notes) in [
        (
            "main =\n",
            "syntax.expected",
            "expected an expression, found end of file",
            0,
        ),
        (
            "x = = 2",
            "syntax.expected",
            "expected an expression, found `=`",
            0,
        ),
        (
            "x = [1, 2)",
            "syntax.unclosed_delimiter",
            "expected `]` to close this List, found `)`",
            0,
        ),
        (
            "x = for\n",
            "syntax.expected",
            "expected a name or pattern, found end of file",
            0,
        ),
        (
            "x = \"abc\n",
            "syntax.expected",
            "expected `\"`, found end of line",
            1,
        ),
        ("é = 1", "syntax.expected", "unexpected `é`", 0),
        (
            "x = 1 2",
            "syntax.item_end",
            "expected end of line, found `2`",
            0,
        ),
        (
            "x = if a\n",
            "syntax.then",
            "expected `then` after the condition",
            1,
        ),
        (
            "b = a < 2 < 3",
            "syntax.comparison_chain",
            "comparisons cannot be chained",
            1,
        ),
        (
            "f(x) = { y := x }",
            "syntax.body_result_count",
            "this body has no result expression",
            1,
        ),
        (
            "y = g(1) { 2 }",
            "syntax.unsupported_construct",
            "payload expressions are not supported",
            0,
        ),
    ] {
        let errors = ast::parse_document(text).unwrap_err();
        let report = syntax(&errors[0], 0);
        assert_eq!(
            (report.code, report.message.as_str()),
            (code, message),
            "{text:?}"
        );
        assert_eq!(report.notes.len(), notes, "{text:?}: {:?}", report.notes);
        assert_eq!(
            report.primary.unwrap().location.span.start,
            errors[0].span().start
        );
    }
}

/// An open bracket is named in the headline only where it is certainly the
/// defect; elsewhere it is a related label, and always a structured field.
#[test]
fn unclosed_brackets_point_at_their_opener() {
    for (text, code, message) in [
        (
            "main = [1, 2, 3\n",
            "syntax.unclosed_delimiter",
            "expected `]` to close this List, found end of file",
        ),
        (
            "a = [1, 3\nb = \"Hi\"\nmain = [1, 2, 3]\n",
            "syntax.expected",
            "expected `,` or `]`, found `=`",
        ),
    ] {
        let report = syntax(&ast::parse_document(text).unwrap_err()[0], 0);
        assert_eq!((report.code, report.message.as_str()), (code, message));
        let opener = &report.related[0];
        assert_eq!(opener.message, "List opened here");
        assert_eq!(opener.location.span.start, text.find('[').unwrap());
        assert_eq!(report.fields["unclosed"]["close"], "]");
        assert!(report.notes.is_empty());
    }
}
