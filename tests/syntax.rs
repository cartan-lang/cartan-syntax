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
fn parses_all_five_current_lab_fixtures_and_retains_loop_structure() {
    // Owner-supplied kernel-lab examples, copied 2026-10-23. Conway alone
    // normalizes line continuation: its subtraction operator moves from the
    // next line's start to the preceding line's end. The original lab checkout
    // remains frozen; the grammar's leading-minus rule is preserved.
    for source in [
        include_str!("fixtures/diffusion.cart"),
        include_str!("fixtures/transport.cart"),
        include_str!("fixtures/conway.cart"),
        include_str!("fixtures/hydro.cart"),
        include_str!("fixtures/nbody.cart"),
    ] {
        let tree = valid_document(source);
        assert!(!nodes(&tree.root, "for_expression").is_empty());
        assert_eq!(nodes(&tree.root, "agent").len(), 1);
    }
    for source in [
        include_str!("fixtures/hydro.cart"),
        include_str!("fixtures/nbody.cart"),
    ] {
        let tree = valid_document(source);
        let reductions = nodes(&tree.root, "fold_expression");
        assert!(!reductions.is_empty());
        for reduction in reductions {
            assert_eq!(
                tree.text(reduction.child("keyword").unwrap()),
                Some("reduce")
            );
            assert!(reduction.child("accumulator").is_some());
            assert!(reduction.child("driver").is_some());
            assert!(reduction.child("body").is_some());
        }
    }
    let hydro = valid_document(include_str!("fixtures/hydro.cart"));
    assert!(!nodes(&hydro.root, "site_pattern").is_empty());
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
    assert_eq!(value(&tree).named_children().count(), 1);
    let tree = expression("[0..\n8]");
    assert_eq!(value(&tree).named_children().count(), 2);
    let tree = expression("[0..n periodic, 3.., ..n, ..]");
    assert_eq!(value(&tree).named_children().count(), 4);
    assert_eq!(nodes(&tree.root, "periodic_expression").len(), 1);

    let tree = parse_expression("if (a + b)\n- c < 3 then 1 else 0");
    assert!(tree.has_errors());
    assert!(matches!(
        tree.diagnostics[0].kind,
        DiagnosticKind::Stated {
            rule: Rule::Then,
            ..
        }
    ));
    expression("if (a + b) -\nc < 3 then 1 else 0");
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
    let tree = parse_document("while true { x = 1 }");
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
    assert_eq!(*rule, Rule::BodyEquals);
    assert!(*refused);
    assert!(!diagnostic.incomplete);
    assert_eq!(diagnostic.span.start, 13);
    assert_eq!(diagnostic.found.as_deref(), Some("x"));
    let name = fields.iter().find(|field| field.name == "name").unwrap();
    assert_eq!(tree.source.get(name.span.start..name.span.end), Some("x"));
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
    // These have concrete grammar trees; the legacy AST lowering refuses the
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
}
