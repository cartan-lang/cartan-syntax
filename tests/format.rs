use cartan_syntax::ast::{self, BinaryOperator, BodyItem, Error, ExprKind, FormDiagnosticKind};
use cartan_syntax::format::{format_document, FormatError};

fn formats(source: &str, expected: &str) {
    let formatted = format_document(source).unwrap();
    assert_eq!(formatted, expected);
    assert_eq!(format_document(&formatted).unwrap(), formatted);
    ast::parse_document(&formatted).unwrap();
}

#[test]
fn formats_a_function_expression_inside_an_equation() {
    formats(
        "scale(gain:Real)=fn (x :Real){gain*x}",
        "scale(gain: Real) = fn(x: Real) { gain * x }",
    );
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
    let source = "a=for i in .. 0 .. n {\n    - f( x[ i ],i+1 ) ^2\n  }";
    let expected = "a = for i in ..0..n {\n    -f(x[i], i + 1)^2\n}";
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
        "a = \"é  # \\t\"\nb = \"\"\"  a\n\t  b\n   \"\"\"\nc = f\" {{ { 1+ 2 } }} \"\nx = 1\n    + 2",
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

#[test]
fn braced_branches_and_arms_keep_their_lines() {
    formats(
        "x=if c{a}else if d{b}else{c}+1",
        "x = if c { a } else if d { b } else { c } + 1",
    );
    formats(
        "x=if c{\ny:=1\n  y\n}else{\n0\n}",
        "x = if c {\n    y := 1\n    y\n} else {\n    0\n}",
    );
    formats(
        "y=match s{\n A (x){x}\nB{1},\n    else {0}\n}",
        "y = match s {\n    A(x) { x }\n    B { 1 },\n    else { 0 }\n}",
    );
}

#[test]
fn a_sum_written_one_term_to_a_line_lines_up_its_operators() {
    // ragged input, the first term on its own line
    formats(
        "laplacian(u: Array<Real, 2>, i: Int, j: Int) =\n  u[i - 1, j + 0]\n     + u[i + 1, j + 0]\n + u[i + 0, j - 1]\n        + u[i + 0, j + 1]\n   - u[i + 0, j + 0] * 4\n",
        "laplacian(u: Array<Real, 2>, i: Int, j: Int) =\n      u[i - 1, j + 0]\n    + u[i + 1, j + 0]\n    + u[i + 0, j - 1]\n    + u[i + 0, j + 1]\n    - u[i + 0, j + 0] * 4\n",
    );
    // every term signed, after a line ending in spaces
    formats(
        "f(u: Real, v: Real) = \n   + u\n   - v\n",
        "f(u: Real, v: Real) =\n    + u\n    - v\n",
    );
    // a leading `+` on the first term
    formats(
        "laplacian(u: Array<Real, 2>, i: Int, j: Int) =\n+ u[i - 1, j + 0]\n  + u[i + 1, j + 0]\n- u[i + 0, j + 0] * 4\n",
        "laplacian(u: Array<Real, 2>, i: Int, j: Int) =\n    + u[i - 1, j + 0]\n    + u[i + 1, j + 0]\n    - u[i + 0, j + 0] * 4\n",
    );
    // a bracket opened on a continued line, and comments at line ends
    formats(
        "s(a: Real, b: Real) =\n  f(a) # first\n  + g(\n  a,\n  b\n  )\n  - h(a) # last\n",
        "s(a: Real, b: Real) =\n      f(a) # first\n    + g(\n        a,\n        b\n    )\n    - h(a) # last\n",
    );
    // an operator that ends its own line leaves its operand in the operand column
    formats(
        "s(a: Real, b: Real) =\n  a\n  +\n  # why\n  b\n  - a\n",
        "s(a: Real, b: Real) =\n      a\n    +\n      # why\n      b\n    - a\n",
    );
}

#[test]
fn a_long_condition_is_a_local_one_operand_to_a_line() {
    formats(
        "f(a: Bool, b: Bool, c: Bool) = {\n  alive :=\n a\n or b\n  or c\n  alive\n}\n",
        "f(a: Bool, b: Bool, c: Bool) = {\n    alive :=\n           a\n        or b\n        or c\n    alive\n}\n",
    );
    formats(
        "f(a: Bool, b: Bool) = {\n  alive := a\n     and b\n  alive\n}\n",
        "f(a: Bool, b: Bool) = {\n    alive := a\n        and b\n    alive\n}\n",
    );
}

#[test]
fn a_sign_against_its_operand_begins_an_item() {
    formats("v = [1\n -2\n 3]\n", "v = [1\n    -2\n    3]\n");
    formats(
        "f(y: Real) = {\n  x := 1\n  -y\n}\n",
        "f(y: Real) = {\n    x := 1\n    -y\n}\n",
    );
}

#[test]
fn a_line_continues_after_an_unfinished_one() {
    formats(
        "x =\n1\ny = {\n z :=\n 2\n z\n}\nstep {\n a <-\n  b\n}\nw = 1 +\n2\n",
        "x =\n    1\ny = {\n    z :=\n        2\n    z\n}\nstep {\n    a <-\n        b\n}\nw = 1 +\n    2\n",
    );
}

#[test]
fn else_opening_a_line_stands_under_its_if() {
    formats(
        "x = if a { 1 }\n  else { 2 }\n",
        "x = if a { 1 }\n    else { 2 }\n",
    );
    formats(
        "f(x: Real) =\n  if x > 0 { x }\n      else if x < 0 { -x }\n  else { 0 }\n",
        "f(x: Real) =\n    if x > 0 { x }\n    else if x < 0 { -x }\n    else { 0 }\n",
    );
    formats(
        "y = match s {\n A(x) { x }\n else { 0 }\n}\nz = match s {\n A(x) { x },\n else { 0 }\n}\n",
        "y = match s {\n    A(x) { x }\n    else { 0 }\n}\nz = match s {\n    A(x) { x },\n    else { 0 }\n}\n",
    );
}

#[test]
fn adjacent_declarations_line_up() {
    // an annotation, a blank line or a comment ends a run; a lone declaration
    // takes one space
    formats(
        "aa = 1\nb = 2\nc: Int = 3\nddd = 4\n\neeeee = 5\nf = 6\n# note\ngg = 7\nh    = 8\n",
        "aa = 1\nb  = 2\nc: Int = 3\nddd = 4\n\neeeee = 5\nf     = 6\n# note\ngg = 7\nh  = 8\n",
    );
    // a destructuring local is no part of a run; a value that runs onto later
    // lines joins by its first line; locals and bindings align apart
    formats(
        "f(u: Conserved) = {\n    Conserved(rho, mom, nrg) := u\n    vel := mom / rho\n    p := 1\n    faces := for j in 0..2 {\n        j\n    }\n    vel\n}\n",
        "f(u: Conserved) = {\n    Conserved(rho, mom, nrg) := u\n    vel   := mom / rho\n    p     := 1\n    faces := for j in 0..2 {\n        j\n    }\n    vel\n}\n",
    );
    formats(
        "param n = 1\nparam count = 2\n",
        "param n     = 1\nparam count = 2\n",
    );
}

#[test]
fn types_and_powers_print_tight() {
    formats(
        "state u: Array < Real , 2 > | None = none\nx = a ^ 2\n",
        "state u: Array<Real, 2>|None = none\nx = a^2\n",
    );
}

#[test]
fn a_continued_chain_keeps_crlf() {
    formats(
        "s(a: Real, b: Real) =\r\n  a\r\n   + b\r\n  - a\r\n",
        "s(a: Real, b: Real) =\r\n      a\r\n    + b\r\n    - a\r\n",
    );
}
