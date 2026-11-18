use cartan_syntax::ast::{self, BinaryOperator, BodyItem, Error, ExprKind, FormDiagnosticKind};
use cartan_syntax::format::{format_document, FormatError};

fn formats(source: &str, expected: &str) {
    let formatted = format_document(source).unwrap();
    assert_eq!(formatted, expected);
    assert_eq!(format_document(&formatted).unwrap(), formatted);
    ast::parse_document(&formatted).unwrap();
}

#[test]
fn declarations_annotations_and_tuples() {
    formats(
        "\tparam n : Int= 3  \nstate count :Int=0\npair :[ Int,Real ]=[n,1.0]\n",
        "param n: Int = 3\nstate count: Int = 0\npair: [Int, Real] = [n, 1.0]\n",
    );
}

#[test]
fn formats_step_writes_and_preserves_their_comments() {
    formats(
        "step{# swap\n  a<-b,b<-a # together\n }",
        "step { # swap\n    a <- b, b <- a # together\n}",
    );
}

#[test]
fn a_function_body_preserves_leading_and_trailing_comments() {
    formats(
        "# heading\nf ( x :Int )={# open\n\t# before the local\n x2:=g( x,2 )   # local\n   x2\n }",
        "# heading\nf(x: Int) = { # open\n    # before the local\n    x2 := g(x, 2) # local\n    x2\n}",
    );
}

#[test]
fn a_comprehension_preserves_calls_indices_and_operator_binding() {
    let source = "a=for i in 0 .. n {\n    - f( x[ i ],i+1 ) ^2\n  }";
    let expected = "a = for i in 0..n {\n    -f(x[i], i + 1) ^ 2\n}";
    formats(source, expected);
    for text in [source, expected] {
        let document = ast::parse_document(text).unwrap();
        let ast::ItemKind::Binding(binding) = &document.items[0].kind else {
            panic!("expected a binding");
        };
        let ExprKind::For(comprehension) = &binding.value.kind else {
            panic!("expected a comprehension");
        };
        let ExprKind::Body(body) = &comprehension.body.kind else {
            panic!("expected a braced body");
        };
        let [BodyItem::Result(result)] = body.items.as_slice() else {
            panic!("expected one result");
        };
        let ExprKind::Unary {
            operator: ast::UnaryOperator::Negate,
            operand,
        } = &result.kind
        else {
            panic!("expected negation of the power");
        };
        let ExprKind::Binary {
            operator: BinaryOperator::Power,
            left,
            right,
        } = &operand.kind
        else {
            panic!("expected a power");
        };
        assert!(matches!(&right.kind, ExprKind::Number(n) if n == "2"));
        assert!(matches!(&left.kind, ExprKind::Call { arguments, .. }
            if matches!(&arguments[0].kind, ExprKind::Index { .. })
            && matches!(&arguments[1].kind, ExprKind::Binary { operator: BinaryOperator::Add, .. })));
    }
}

#[test]
fn leaves_strings_and_line_breaks_intact() {
    formats(
        "a=\"é  # \\t\"\nb=\"\"\"  a\n\t  b\n   \"\"\"\nc=f\" {{ { 1+ 2 } }} \"\nx=1\n\t+   2",
        "a = \"é  # \\t\"\nb = \"\"\"  a\n\t  b\n   \"\"\"\nc = f\" {{ { 1+ 2 } }} \"\nx = 1\n\t+ 2",
    );
    formats("  x=1\r\n\ty=2\r\n", "x = 1\r\ny = 2\r\n");
}

#[test]
fn reports_grammar_and_source_form_failures() {
    let source = "x=f(1";
    let Err(FormatError::Syntax(errors)) = format_document(source) else {
        panic!("expected an incomplete-call diagnostic");
    };
    assert!(matches!(errors.as_slice(), [Error::Grammar(error)] if error.incomplete));
    assert_eq!(source, "x=f(1");

    let Err(FormatError::Syntax(errors)) = format_document("x=a<b<c") else {
        panic!("expected a comparison-chain diagnostic");
    };
    assert!(
        matches!(errors.as_slice(), [Error::Form(error)] if error.kind == FormDiagnosticKind::ComparisonChain)
    );
}

#[test]
fn function_separation_preserves_comment_attachment() {
    formats(
        "f(x)=x\n\"\"\"The next function.\"\"\"\ng(x)=x",
        "f(x) = x\n\n\"\"\"The next function.\"\"\"\ng(x) = x",
    );
    formats(
        "f(x)=x\ng(x)=x\n\n\n\nh(x)=x\n",
        "f(x) = x\n\ng(x) = x\n\nh(x) = x\n",
    );
    formats(
        "f(x)=x # tail\n# next function\ng(x)=x\n",
        "f(x) = x # tail\n\n# next function\ng(x) = x\n",
    );
    formats("f(x)=x\r\n \r\n\r\ng(x)=x", "f(x) = x\r\n\r\ng(x) = x");
    formats("f(x)=x\nx=2\ng(x)=x", "f(x) = x\nx = 2\ng(x) = x");
}
