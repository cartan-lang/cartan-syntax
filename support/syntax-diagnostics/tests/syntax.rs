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
