use cartan_syntax::{
    grammar, parse_document, parse_expression, DiagnosticKind, Node, Rule, SyntaxTree,
};

fn valid_document(source: &str) -> SyntaxTree {
    let tree = parse_document(source);
    assert!(!tree.has_errors(), "{source:?}: {:?}", tree.diagnostics);
    assert_spans(&tree);
    tree
}

fn expression(source: &str) -> SyntaxTree {
    let tree = parse_expression(source);
    assert!(!tree.has_errors(), "{source:?}: {:?}", tree.diagnostics);
    assert_spans(&tree);
    tree
}

fn assert_spans(tree: &SyntaxTree) {
    fn visit(node: &Node, source: &str) {
        assert!(source.get(node.start..node.end).is_some(), "{node:?}");
        for child in &node.children {
            visit(child, source);
        }
    }
    visit(&tree.root, &tree.source);
    for diagnostic in &tree.diagnostics {
        assert!(tree
            .source
            .get(diagnostic.span.start..diagnostic.span.end)
            .is_some());
        if let DiagnosticKind::Stated { fields, .. } = &diagnostic.kind {
            for field in fields {
                assert!(tree.source.get(field.span.start..field.span.end).is_some());
            }
        }
    }
}

fn nodes<'a>(node: &'a Node, kind: &str) -> Vec<&'a Node> {
    let mut found = Vec::new();
    if node.kind == kind {
        found.push(node);
    }
    for child in &node.children {
        found.extend(nodes(child, kind));
    }
    found
}

fn value(tree: &SyntaxTree) -> &Node {
    tree.root.named_children().next().unwrap()
}

#[test]
fn map_entries_distinguish_static_and_computed_separators_in_the_cst() {
    let source = "{ static: 1, key => 2 }";
    let tree = expression(source);
    let entries = nodes(&tree.root, "map_entry");
    assert_eq!(entries.len(), 2);
    assert_eq!(tree.text(entries[0].child("key").unwrap()), Some("static"));
    assert_eq!(tree.text(entries[0].child("separator").unwrap()), Some(":"));
    assert_eq!(tree.text(entries[1].child("key").unwrap()), Some("key"));
    assert_eq!(
        tree.text(entries[1].child("separator").unwrap()),
        Some("=>")
    );
    assert_eq!(tree.text(entries[1].child("value").unwrap()), Some("2"));
}

/// A statement continues onto the next line where the grammar says it does: a
/// conditional's `else` opens a line, `and` and `or` open lines, and a
/// trailing operator and an open paren carry the line on.
#[test]
fn statements_continue_across_lines_where_the_grammar_says() {
    for (source, tree) in [
        (
            "x = if a { 1 }\n    else if b { 2 }\n    else { 3 }",
            "(conditional_expression condition: (identifier) consequence: (body (number)) \
             alternative: (conditional_expression condition: (identifier) \
             consequence: (body (number)) alternative: (body (number))))",
        ),
        (
            "x = a == 1\n    and b\n    or c",
            "(binary_expression left: (binary_expression left: (binary_expression \
             left: (identifier) right: (number)) right: (identifier)) right: (identifier))",
        ),
        (
            "x = a -\n    b",
            "(binary_expression left: (identifier) right: (identifier))",
        ),
        (
            "x = 2 * (\n    a +\n    b\n)",
            "(binary_expression left: (number) right: (parenthesized_expression \
             (binary_expression left: (identifier) right: (identifier))))",
        ),
    ] {
        let document = valid_document(source);
        let [binding] = document.root.named_children().collect::<Vec<_>>()[..] else {
            panic!("{source:?}: {}", document.root.sexp())
        };
        let value = binding.child("value").unwrap().sexp();
        assert_eq!(value.trim_start_matches("value: "), tree, "{source:?}");
    }
}

#[test]
fn preserves_power_unary_projection_and_range_precedence() {
    let tree = expression("-2^2");
    let unary = value(&tree);
    assert_eq!(unary.kind, "unary_expression");
    let power = unary.child("operand").unwrap();
    assert_eq!(power.kind, "binary_expression");
    assert_eq!(tree.text(power.child("operator").unwrap()), Some("^"));

    let tree = expression("2^3^4");
    let power = value(&tree);
    assert_eq!(tree.text(power.child("left").unwrap()), Some("2"));
    assert_eq!(power.child("right").unwrap().kind, "binary_expression");

    let tree = expression("a of b ^2");
    let power = value(&tree);
    assert_eq!(power.kind, "binary_expression");
    assert_eq!(power.child("left").unwrap().kind, "component_expression");

    let tree = expression("1 + 2..3 == 4");
    let range = value(&tree);
    assert_eq!(range.kind, "range_expression");
    assert_eq!(tree.text(range.child("left").unwrap()), Some("1 + 2"));
    assert_eq!(tree.text(range.child("right").unwrap()), Some("3 == 4"));
}

#[test]
fn preserves_newline_separation_continuation_and_open_ranges() {
    let tree = expression("[1\n-2\n3]");
    assert_eq!(value(&tree).named_children().count(), 3);
    let tree = expression("f(1\n2)");
    assert_eq!(
        nodes(&tree.root, "argument_list")[0]
            .named_children()
            .count(),
        2
    );
    let tree = expression("[1\n+2]");
    assert_eq!(value(&tree).named_children().count(), 2);
    for sign in ["+", "-"] {
        let tree = expression(&format!("[1\n{sign} 2\n  {sign}\n3]"));
        assert_eq!(value(&tree).named_children().count(), 1, "{sign}");
        let tree = expression(&format!("{{ y := 2\n {sign}y }}"));
        assert_eq!(nodes(&tree.root, "unary_expression").len(), 1, "{sign}");
    }
    let tree = valid_document(
        "f(u, i) =\n      + u[i - 1]\n      + u[i + 1]\n      - u[i + 0] * 2\ng = 1\n",
    );
    assert_eq!(tree.root.named_children().count(), 2);
    assert_eq!(nodes(&tree.root, "binary_expression").len(), 6);
    assert_eq!(nodes(&tree.root, "unary_expression").len(), 1);
    let tree = expression("[0..\n8]");
    assert_eq!(value(&tree).named_children().count(), 2);
    let tree = expression("[..0..n, 3.., ..n, ..]");
    assert_eq!(value(&tree).named_children().count(), 4);
    assert_eq!(nodes(&tree.root, "periodic_range").len(), 1);
    let tree = expression("[0..n periodic, 3.., ..n, ..]");
    assert_eq!(value(&tree).named_children().count(), 4);
    assert_eq!(nodes(&tree.root, "periodic_expression").len(), 1);

    expression("if (a + b)\n- c < 3 { 1 } else { 0 }");
    let tree = parse_expression("if (a + b)\n-c < 3 { 1 } else { 0 }");
    assert!(tree.has_errors());
    assert!(matches!(
        tree.diagnostics[0].kind,
        DiagnosticKind::Stated {
            rule: Rule::IfBody,
            ..
        }
    ));
    expression("if (a + b) -\nc < 3 { 1 } else { 0 }");
}

#[test]
fn preserves_binding_pattern_and_postfix_distinctions() {
    let tree = valid_document("f = 1\ng() = 1\nh(x: P) = x\nk((x): P) = x\n");
    let items: Vec<_> = tree.root.named_children().collect();
    assert_eq!(items[0].kind, "binding");
    assert_eq!(items[1].kind, "function_definition");
    assert_eq!(nodes(items[2], "site_pattern").len(), 0);
    assert_eq!(nodes(items[3], "site_pattern").len(), 1);
    let tree = expression("for i in b { i }");
    assert!(nodes(&tree.root, "list_pattern").is_empty());
    let tree = expression("for [i] in b { i }");
    assert_eq!(nodes(&tree.root, "list_pattern").len(), 1);
    let tree = expression("f[1]");
    assert_eq!(value(&tree).kind, "index_expression");
    let tree = expression("f [1]");
    assert_eq!(value(&tree).kind, "payload_expression");
    assert!(parse_expression("f (1)").has_errors());
}

#[test]
fn preserves_bare_and_bracketed_local_list_patterns() {
    for (pattern, kind, pattern_end, names, value_start) in [
        ("i, j", "bare_list_pattern", 6, [(2, 3), (5, 6)], 10),
        ("[i, j]", "list_pattern", 8, [(3, 4), (6, 7)], 12),
    ] {
        let tree = valid_document(&format!("{{ {pattern} := [10, 2]\n i + j }}"));
        let locals = nodes(&tree.root, "local");
        assert_eq!(locals.len(), 1);
        let binder = locals[0].child("pattern").unwrap();
        assert_eq!(binder.kind, kind);
        assert_eq!((binder.start, binder.end), (2, pattern_end));
        assert_eq!(tree.text(binder), Some(pattern));
        let identifiers: Vec<_> = binder.named_children().collect();
        assert_eq!(identifiers.len(), 2);
        for ((identifier, name), span) in identifiers.iter().zip(["i", "j"]).zip(names) {
            assert_eq!(identifier.kind, "identifier");
            assert_eq!(tree.text(identifier), Some(name));
            assert_eq!((identifier.start, identifier.end), span);
        }
        let rhs = locals[0].child("value").unwrap();
        assert_eq!(rhs.kind, "list");
        assert_eq!(tree.text(rhs), Some("[10, 2]"));
        assert_eq!((rhs.start, rhs.end), (value_start, value_start + 7));
        assert_eq!(
            rhs.named_children()
                .map(|node| tree.text(node).unwrap())
                .collect::<Vec<_>>(),
            ["10", "2"]
        );
    }
}

#[test]
fn local_list_patterns_require_a_body_for_colon_equals() {
    for (source, colon, pattern_end) in [
        ("i, j := [10, 2]", 5, 4),
        ("[i, j] := [10, 2]", 7, 6),
        ("[i, j] := [10, 1]", 7, 6),
    ] {
        let tree = parse_document(source);
        assert!(tree.has_errors());
        assert_spans(&tree);
        assert_eq!(tree.diagnostics.len(), 1);
        let diagnostic = &tree.diagnostics[0];
        assert_eq!(
            (diagnostic.span.start, diagnostic.span.end),
            (colon, colon + 1)
        );
        assert_eq!(diagnostic.found.as_deref(), Some(":"));
        assert!(!diagnostic.incomplete);
        assert_eq!(
            diagnostic.kind,
            DiagnosticKind::Stated {
                rule: Rule::PatternValue,
                refused: false,
                fields: vec![cartan_syntax::Field {
                    name: "pattern".into(),
                    span: cartan_syntax::Span {
                        start: 0,
                        end: pattern_end,
                    },
                }],
            }
        );
        valid_document(&source.replace(":=", "="));
    }
}

#[test]
fn reports_typed_rule_identity_sites_and_captured_fields() {
    let tree = parse_document("step { state x = 1 }");
    assert!(tree.has_errors());
    let diagnostic = &tree.diagnostics[0];
    let DiagnosticKind::Stated {
        rule,
        refused,
        fields,
    } = &diagnostic.kind
    else {
        panic!("expected a grammar-stated diagnostic: {diagnostic:?}");
    };
    assert_eq!(*rule, Rule::StateLocal);
    assert!(!*refused);
    assert!(!diagnostic.incomplete);
    assert_eq!(diagnostic.span.start, 15);
    assert_eq!(diagnostic.found.as_deref(), Some("="));
    let name = fields.iter().find(|field| field.name == "name").unwrap();
    assert_eq!(tree.source.get(name.span.start..name.span.end), Some("x"));
    assert_spans(&tree);

    let tree = parse_document("fiber F = (A: Real | B: Real, C: Real)");
    let [diagnostic] = tree.diagnostics.as_slice() else {
        panic!("{:?}", tree.diagnostics);
    };
    assert!(matches!(
        diagnostic.kind,
        DiagnosticKind::Stated {
            rule: Rule::MixedSeparators,
            refused: true,
            ..
        }
    ));
    assert_eq!(diagnostic.found.as_deref(), Some(","));
    assert_spans(&tree);

    let tree = parse_document("step true { x <- 1 }");
    let [diagnostic] = tree.diagnostics.as_slice() else {
        panic!("{:?}", tree.diagnostics);
    };
    assert!(matches!(
        diagnostic.kind,
        DiagnosticKind::Stated {
            rule: Rule::UpdateBody,
            refused: false,
            ..
        }
    ));
    assert_eq!(diagnostic.found.as_deref(), Some("true"));
    assert_spans(&tree);

    let tree = parse_document("param n =");
    assert!(tree.has_errors());
    assert!(tree.diagnostics[0].incomplete);
    assert_eq!(tree.diagnostics[0].span.start, 9);
    assert_spans(&tree);
    for rule in Rule::ALL {
        assert_eq!(Rule::of(rule.label()), Some(*rule));
    }
}

/// A branch and an arm open their brace where the condition, the `else` or
/// the arm's pattern ends, and `then` is an ordinary name.
#[test]
fn branches_and_arms_are_refused_without_their_brace() {
    for (source, rule, found, variant) in [
        ("x = if c then 1", Rule::IfBody, "then", None),
        ("x = if c { 1 } else 2", Rule::IfBody, "2", None),
        ("x = match s { A(y) y }", Rule::ArmBody, "y", Some("A")),
        ("x = match s { A { 1 }, else 0 }", Rule::ArmBody, "0", None),
    ] {
        let tree = parse_document(source);
        assert_spans(&tree);
        let [diagnostic] = tree.diagnostics.as_slice() else {
            panic!("{source}: {:?}", tree.diagnostics);
        };
        let DiagnosticKind::Stated {
            rule: stated,
            refused: false,
            fields,
        } = &diagnostic.kind
        else {
            panic!("{source}: {diagnostic:?}");
        };
        assert_eq!(*stated, rule, "{source}");
        assert_eq!(diagnostic.found.as_deref(), Some(found), "{source}");
        assert_eq!(
            &source[diagnostic.span.start..diagnostic.span.end],
            found,
            "{source}"
        );
        let written = fields
            .iter()
            .find(|field| field.name == "variant")
            .map(|field| &source[field.span.start..field.span.end]);
        assert_eq!(written, variant, "{source}");
    }

    let tree = valid_document(
        "then = 1
x = if then { then }",
    );
    let items: Vec<_> = tree.root.named_children().collect();
    assert_eq!(items[0].kind, "binding");
    assert_eq!(tree.text(items[0].child("name").unwrap()), Some("then"));
    let conditional = items[1].child("value").unwrap();
    assert_eq!(conditional.kind, "conditional_expression");
    assert_eq!(conditional.child("condition").unwrap().kind, "identifier");
}

#[test]
fn malformed_annotations_and_unicode_produce_valid_source_spans() {
    for source in [
        "state x: = 1",
        "f(x: ) = 1",
        "f(x:)=1",
        "x = 1界",
        "λ = 1",
        "param while = 1",
        "x = else",
        "f((x,y)) = 1",
        "x = @",
        "x = \"unterminated",
    ] {
        let tree = parse_document(source);
        assert!(tree.has_errors(), "{source:?}");
        assert_spans(&tree);
    }
    let tree = valid_document("# λ\nlabel = \"λ 🐈\"\nescaped = \"\\λ\"\n");
    assert_eq!(nodes(&tree.root, "comment").len(), 1);
    assert_eq!(tree.text(nodes(&tree.root, "string")[0]), Some("\"λ 🐈\""));
}

#[test]
fn grammar_acceptance_is_explicitly_distinct_from_later_form_checks() {
    // These have concrete grammar trees; AST normalization refuses the
    // comparison/range chains and the body with two result expressions.
    for source in ["1 < 2 < 3", "1..2..3", "{1\n2}"] {
        expression(source);
    }
}

#[test]
fn all_editor_tooling_reads_the_shared_grammar() {
    let javascript = grammar::print();
    assert!(javascript.contains("name: \"cartan\""));
    assert!(javascript.contains("for_expression"));
    assert!(javascript.contains("_newline"));
    assert!(grammar::print_textmate().contains("source.cartan"));
    assert!(grammar::print_vim().contains("cartan"));
    assert!(grammar::print_vim_ftdetect().contains("*.cart"));
    assert!(grammar::print_sublime().contains("source.cartan"));
    assert!(grammar::print_emacs(&["custom_builtin"]).contains("custom_builtin"));
    assert!(grammar::grammar()
        .check_query("(for_expression binders: (binder_list)) @loop")
        .is_empty());
    assert!(grammar::grammar()
        .check_query("(step_block body: (body)) @update")
        .is_empty());
    assert!(grammar::grammar()
        .check_query("(periodic_range axis: (range_expression)) @axis")
        .is_empty());
}

#[test]
fn annotated_local_prefix_selects_body_before_list_value_refusal() {
    let tree = valid_document("x = { y: [[Int; 2]; n] := value\n y }");
    let locals = nodes(&tree.root, "local");
    assert_eq!(locals.len(), 1);
    assert_eq!(
        tree.text(locals[0].child("type").unwrap()),
        Some("[[Int; 2]; n]")
    );
    assert!(nodes(&tree.root, "map").is_empty());
    let tree = valid_document("x = { y: [1, 2] }");
    assert_eq!(nodes(&tree.root, "map").len(), 1);
    assert!(nodes(&tree.root, "local").is_empty());
}

#[test]
fn a_failure_inside_brackets_reports_the_innermost_open_one() {
    let text = "x = [1, (2\n";
    let tree = cartan_syntax::parse_document(text);
    let [diagnostic] = tree.diagnostics.as_slice() else {
        panic!("{:?}", tree.diagnostics)
    };
    let open = diagnostic.unclosed.as_ref().unwrap();
    assert_eq!(open.span.start, text.find('(').unwrap());
    assert_eq!((open.open.as_str(), open.close.as_str()), ("(", ")"));
    assert_eq!(open.construct.as_deref(), Some("parenthesized_expression"));
    // Brackets closed on later lines leave nothing open.
    valid_document("x = [\n  1,\n  (2 + 3),\n]\nm = {a: [1, 2]}\n");
}

#[test]
fn an_unfinished_call_waits_for_its_paren_rather_than_refusing() {
    for text in ["x = f(1", "x = f(1, 2\ny = 3\n"] {
        let tree = parse_document(text);
        let diagnostic = &tree.diagnostics[0];
        assert!(
            !matches!(diagnostic.kind, DiagnosticKind::Stated { .. }),
            "{text:?}: {diagnostic:?}"
        );
        let open = diagnostic.unclosed.as_ref().unwrap();
        assert_eq!(open.span.start, text.find('(').unwrap());
    }
    assert!(parse_document("x = f(1").diagnostics[0].incomplete);
}

/// A failed item ends at a line's end after its diagnostic, and every later
/// item is read: one bad line hides no other. Each case gives the text of
/// each `ERROR` region, one diagnostic standing in each, and the number of
/// items read around them.
#[test]
fn a_failed_item_ends_at_its_line_and_later_items_are_read() {
    for (source, errors, items) in [
        // between good lines, at the end with and without a final newline
        ("a = 1\nx = 1 2\nb = 2", &["x = 1 2"][..], 2),
        ("a = 1\nx = 1 2", &["x = 1 2"], 1),
        ("a = 1\nx = 1 2\n", &["x = 1 2"], 1),
        // two bad lines apart, each its own region
        ("x = 1 2\ny = 3\nz = 4 4\nw = 5", &["x = 1 2", "z = 4 4"], 2),
        // a bad line inside a body: the item ends with the body
        (
            "step {\n  u <- 1 1\n  v <- 2\n}\ny = 3",
            &["step {\n  u <- 1 1\n  v <- 2\n}"],
            1,
        ),
        // a statement continued onto the next line is one item when good
        // and one error when bad
        ("x = 1 +\n  2\ny = 3", &[], 2),
        ("x = 1 +\n  2 2\ny = 3", &["x = 1 +\n  2 2"], 1),
        // a quoted literal left open ends at its line; a triple-quoted one
        // runs on, and a literal's line ends are no item's
        ("x = \"abc\ny = 2", &["x = \"abc"], 1),
        ("x = \"\"\"abc\ny = 2", &["x = \"\"\"abc\ny = 2"], 0),
        (
            "x = 1 2 \"\"\"a\nb\"\"\" 3\ny = 2",
            &["x = 1 2 \"\"\"a\nb\"\"\" 3"],
            1,
        ),
        // a bracket left open: the item reaches the line the parse stopped
        // on, and a later line as far out as the item begins one
        ("x = f(1,\ny = 2\nz = 3", &["x = f(1,\ny = 2"], 1),
        (
            "x = f(1,\n  2 2,\n  3)\ny = 4",
            &["x = f(1,\n  2 2,\n  3)"],
            1,
        ),
    ] {
        let tree = parse_document(source);
        assert_spans(&tree);
        let regions: Vec<_> = tree
            .root
            .named_children()
            .filter(|node| node.kind == "ERROR")
            .collect();
        let written: Vec<_> = regions
            .iter()
            .map(|node| tree.text(node).unwrap())
            .collect();
        assert_eq!(written, errors, "{source:?}");
        assert_eq!(tree.diagnostics.len(), regions.len(), "{source:?}");
        for (diagnostic, region) in tree.diagnostics.iter().zip(&regions) {
            let inside =
                region.start <= diagnostic.span.start && diagnostic.span.start <= region.end;
            assert!(inside, "{source:?}: {diagnostic:?}");
        }
        let read = tree
            .root
            .named_children()
            .filter(|node| node.kind != "ERROR" && !node.extra)
            .count();
        assert_eq!(read, items, "{source:?}");
    }
}
