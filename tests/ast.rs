use cartan_syntax::ast::{self, *};
use cartan_syntax::Span;

fn expression(source: &str) -> Expr {
    let expression =
        ast::parse_expression(source).unwrap_or_else(|errors| panic!("{source:?}: {errors:?}"));
    assert_expr_spans(source, &expression);
    expression
}

fn assert_name_span(source: &str, name: &Name) {
    assert_eq!(
        source.get(name.span.start..name.span.end),
        Some(name.text.as_str())
    );
}

fn assert_pattern_spans(source: &str, pattern: &Pattern) {
    let text = source.get(pattern.span.start..pattern.span.end).unwrap();
    match &pattern.kind {
        PatternKind::Name(name) => assert_name_span(source, name),
        PatternKind::Wildcard => assert_eq!(text, "_"),
        PatternKind::List(patterns) | PatternKind::Site(patterns) => {
            for pattern in patterns {
                assert_pattern_spans(source, pattern);
            }
        }
    }
}

fn assert_body_spans(source: &str, body: &Body) {
    assert!(source.get(body.span.start..body.span.end).is_some());
    for item in &body.items {
        match item {
            BodyItem::Local(local) => {
                assert!(source.get(local.span.start..local.span.end).is_some());
                assert_pattern_spans(source, &local.pattern);
                assert_expr_spans(source, &local.value);
            }
            BodyItem::Result(expr) => assert_expr_spans(source, expr),
        }
    }
}

fn assert_expr_spans(source: &str, expr: &Expr) {
    assert!(source.get(expr.span.start..expr.span.end).is_some());
    match &expr.kind {
        ExprKind::Number(_) | ExprKind::Bool(_) => {}
        ExprKind::Name(name) => assert_name_span(source, name),
        ExprKind::Unary { operand, .. } | ExprKind::Periodic(operand) => {
            assert_expr_spans(source, operand)
        }
        ExprKind::Binary { left, right, .. } => {
            assert_expr_spans(source, left);
            assert_expr_spans(source, right);
        }
        ExprKind::If {
            condition,
            consequence,
            alternative,
        } => {
            for expr in [condition, consequence, alternative] {
                assert_expr_spans(source, expr);
            }
        }
        ExprKind::Call {
            function,
            arguments,
        } => {
            assert_name_span(source, function);
            for expr in arguments {
                assert_expr_spans(source, expr);
            }
        }
        ExprKind::Index { target, indices } => {
            assert_expr_spans(source, target);
            for expr in indices {
                assert_expr_spans(source, expr);
            }
        }
        ExprKind::Projection { key, value } => {
            assert_name_span(source, key);
            assert_expr_spans(source, value);
        }
        ExprKind::List(expressions) => {
            for expr in expressions {
                assert_expr_spans(source, expr);
            }
        }
        ExprKind::Range { start, end, .. } => {
            for expr in start.iter().chain(end) {
                assert_expr_spans(source, expr);
            }
        }
        ExprKind::Body(body) => assert_body_spans(source, body),
        ExprKind::For(comprehension) => {
            assert_pattern_spans(source, &comprehension.binder);
            assert_expr_spans(source, &comprehension.collection);
            assert_expr_spans(source, &comprehension.body);
        }
        ExprKind::Reduce(reduction) => {
            assert_name_span(source, &reduction.accumulator);
            assert_pattern_spans(source, &reduction.binder);
            assert_expr_spans(source, &reduction.seed);
            assert_expr_spans(source, &reduction.collection);
            assert_body_spans(source, &reduction.body);
        }
    }
}

#[test]
fn normalizes_all_five_workloads_and_retains_document_items() {
    for source in [
        include_str!("fixtures/diffusion.cart"),
        include_str!("fixtures/transport.cart"),
        include_str!("fixtures/conway.cart"),
        include_str!("fixtures/hydro.cart"),
        include_str!("fixtures/nbody.cart"),
    ] {
        let document = ast::parse_document(source).unwrap_or_else(|errors| panic!("{errors:?}"));
        assert_eq!(document.source, source);
        for item in &document.items {
            assert!(source.get(item.span.start..item.span.end).is_some());
            match &item.kind {
                ItemKind::Binding(binding) => {
                    assert_pattern_spans(source, &binding.pattern);
                    assert_expr_spans(source, &binding.value);
                }
                ItemKind::Function(function) => {
                    assert_name_span(source, &function.name);
                    for parameter in &function.parameters {
                        assert_pattern_spans(source, &parameter.pattern);
                    }
                    assert_expr_spans(source, &function.body);
                }
                ItemKind::While(update) => {
                    assert_expr_spans(source, &update.guard);
                    for write in &update.writes {
                        assert_name_span(source, &write.target);
                        assert_expr_spans(source, &write.value);
                    }
                }
                ItemKind::Use(import) => assert_name_span(source, &import.namespace),
                ItemKind::Fiber(fiber) => {
                    assert_name_span(source, &fiber.name);
                    for component in &fiber.components {
                        assert_name_span(source, &component.name);
                    }
                    let expected = if fiber.name.text == "Particle" { 7 } else { 3 };
                    assert_eq!(fiber.components.len(), expected);
                    assert_eq!(fiber.algebra.as_ref().unwrap().text, "Module");
                }
            }
        }
        assert_eq!(
            document
                .items
                .iter()
                .filter(|item| matches!(item.kind, ItemKind::While(_)))
                .count(),
            1
        );
        assert_eq!(
            document
                .items
                .iter()
                .filter(|item| matches!(
                    item.kind,
                    ItemKind::Binding(Binding {
                        kind: BindingKind::State,
                        ..
                    })
                ))
                .count(),
            1
        );
        if source.contains("use std::*") {
            assert!(
                matches!(&document.items[0].kind, ItemKind::Use(Use { namespace: Name { text, .. }, imports: Imports::All { .. } }) if text == "std")
            );
            assert!(
                matches!(&document.items.last().unwrap().kind, ItemKind::Binding(Binding { pattern: Pattern { kind: PatternKind::Name(Name { text, .. }), .. }, value: Expr { kind: ExprKind::Number(value), .. }, .. }) if text == "main" && value == "0")
            );
        }
    }
}

#[test]
fn preserves_body_order_and_normalizes_list_patterns_with_original_spans() {
    for (source, pattern_span, name_spans) in [
        ("{ i, j := [10, 2]\n i + j }", (2, 6), [(2, 3), (5, 6)]),
        ("{ [i, j] := [10, 2]\n i + j }", (2, 8), [(3, 4), (6, 7)]),
    ] {
        let expr = expression(source);
        let ExprKind::Body(body) = expr.kind else {
            panic!()
        };
        let [BodyItem::Local(local), BodyItem::Result(_)] = body.items.as_slice() else {
            panic!()
        };
        assert_eq!(
            local.pattern.span,
            Span {
                start: pattern_span.0,
                end: pattern_span.1
            }
        );
        let PatternKind::List(patterns) = &local.pattern.kind else {
            panic!()
        };
        assert_eq!(patterns.len(), 2);
        for ((pattern, name), (start, end)) in patterns.iter().zip(["i", "j"]).zip(name_spans) {
            assert_eq!(
                pattern.kind,
                PatternKind::Name(Name {
                    text: name.into(),
                    span: Span { start, end }
                })
            );
        }
        assert_eq!(
            &source[local.value.span.start..local.value.span.end],
            "[10, 2]"
        );
    }
    let expr = expression("{ i + j\n i, j := [10, 2] }");
    let ExprKind::Body(body) = expr.kind else {
        panic!()
    };
    assert!(matches!(
        body.items.as_slice(),
        [BodyItem::Result(_), BodyItem::Local(_)]
    ));
    let scalar = expression("for i in xs { i }");
    let singleton = expression("for [i] in xs { i }");
    let (ExprKind::For(scalar), ExprKind::For(singleton)) = (scalar.kind, singleton.kind) else {
        panic!()
    };
    assert!(matches!(scalar.binder.kind, PatternKind::Name(_)));
    assert!(matches!(singleton.binder.kind, PatternKind::List(_)));
}

#[test]
fn validates_written_chains_before_removing_grouping() {
    for (source, expected) in [
        ("1 < 2 == 3", FormDiagnosticKind::ComparisonChain),
        ("1 < 2\n== 3", FormDiagnosticKind::ComparisonChain),
        ("1..2..3", FormDiagnosticKind::RangeChain),
        (
            "{ x := 1 }",
            FormDiagnosticKind::BodyResultCount { found: 0 },
        ),
        ("{1\n2}", FormDiagnosticKind::BodyResultCount { found: 2 }),
    ] {
        assert_eq!(
            ast::parse_expression(source).unwrap_err(),
            [Error::Form(FormDiagnostic {
                kind: expected,
                span: Span {
                    start: 0,
                    end: source.len()
                }
            })]
        );
        assert!(!cartan_syntax::parse_expression(source).has_errors());
    }
    for source in ["(1 < 2) == 3", "1 < (2 == 3)", "(1..2)..3", "1..(2..3)"] {
        expression(source);
    }
    let expr = expression("-2^3^4");
    let ExprKind::Unary {
        operator: UnaryOperator::Negate,
        operand,
    } = expr.kind
    else {
        panic!()
    };
    let ExprKind::Binary {
        operator: BinaryOperator::Power,
        right,
        ..
    } = operand.kind
    else {
        panic!()
    };
    assert!(matches!(
        right.kind,
        ExprKind::Binary {
            operator: BinaryOperator::Power,
            ..
        }
    ));
}

#[test]
fn retains_full_grammar_diagnostics_and_incomplete_input() {
    for source in ["param n =", "f(x: ) = 1", "λ = 1", "i, j := [10, 2]"] {
        let cst = cartan_syntax::parse_document(source);
        let expected: Vec<_> = cst
            .diagnostics
            .iter()
            .cloned()
            .map(Error::Grammar)
            .collect();
        let actual = ast::parse_document(source).unwrap_err();
        assert_eq!(actual, expected);
        for error in actual {
            let span = error.span();
            assert!(source.get(span.start..span.end).is_some());
        }
    }
}

#[test]
fn preserves_annotations_site_patterns_and_source_spelling() {
    let source = "# λ\nf((rho, _, pre): Primitive, p: Array<Unknown, 17>) = pre of p[2]\nparam unused: Unknown = 1e99\n";
    let document = ast::parse_document(source).unwrap();
    let ItemKind::Function(function) = &document.items[0].kind else {
        panic!()
    };
    assert_eq!(
        function.name,
        Name {
            text: "f".into(),
            span: Span { start: 5, end: 6 }
        }
    );
    let PatternKind::Site(parts) = &function.parameters[0].pattern.kind else {
        panic!()
    };
    assert_eq!(parts.len(), 3);
    assert!(matches!(parts[1].kind, PatternKind::Wildcard));
    assert_eq!(&source[parts[1].span.start..parts[1].span.end], "_");
    let annotation = function.parameters[1].annotation.as_ref().unwrap();
    let AnnotationKind::Apply { name, arguments } = &annotation.kind else {
        panic!()
    };
    assert_eq!(name.text, "Array");
    assert_eq!(
        &source[annotation.span.start..annotation.span.end],
        "Array<Unknown, 17>"
    );
    assert!(
        matches!(&arguments[0], TypeArgument::Type(Annotation { kind: AnnotationKind::Named(Name { text, .. }), .. }) if text == "Unknown")
    );
    assert!(
        matches!(&arguments[1], TypeArgument::Number { text, span } if text == "17" && &source[span.start..span.end] == "17")
    );
    let ExprKind::Projection { key, value } = &function.body.kind else {
        panic!()
    };
    assert_eq!(key.text, "pre");
    assert!(
        matches!(&value.kind, ExprKind::Index { target, indices } if matches!(&target.kind, ExprKind::Name(name) if name.text == "p") && indices.len() == 1)
    );
    let ItemKind::Binding(binding) = &document.items[1].kind else {
        panic!()
    };
    // Unknown types and machine-nonfinite numeric values are analysis concerns.
    assert_eq!(binding.value.kind, ExprKind::Number("1e99".into()));
}

#[test]
fn unsupported_constructs_and_optional_clauses_are_not_discarded() {
    for source in [
        "use std::* as s",
        "use std::* with (x: 1)",
        "use \"other.cart\" as other",
        "param n = 1 (locked when true)",
        "fiber Cell<k> = (v: Real)",
        "fiber Mode = (A | B)",
        "f(x: Real = 1) = x",
        "f(x) : { pad: 1 } = x",
        "f(x: Real|None) = x",
        "f(x: [Int; 2]) = x",
        "every 1 while true { x <- 1 }",
        "while true : { precision: 1 } { x <- 1 }",
        "while true { x := 1\n p <- x }",
        "x = reduce a := 0 for i in 0..2 while true { a + i }",
        "x = reduce a := 0 for i in 0..2 : { max_iter: 2 } { a + i }",
        "x = fold a := 0 for i in 0..2 { a + i }",
        "x = reduce a := 0 { a }",
        "x = f'(1)",
        "x = f(:1)",
        "x = Foo(A: 1)",
        "x = (A: Real)(1)",
        "x = if true then 1",
        "x = 0 of y",
        "x = ns::f(1)",
        "x = 1 ?? 2",
        "x = { state p := 1\n p }",
        "x = { y <- 1\n y }",
        "namespace a() { x = 1 }",
    ] {
        let cst = cartan_syntax::parse_document(source);
        assert!(
            !cst.has_errors(),
            "test must reach normalization: {source:?}: {:?}",
            cst.diagnostics
        );
        let errors = ast::parse_document(source).expect_err(source);
        assert!(!errors.is_empty());
        for error in errors {
            assert!(
                matches!(
                    &error,
                    Error::Form(FormDiagnostic {
                        kind: FormDiagnosticKind::UnsupportedConstruct { .. },
                        ..
                    })
                ),
                "{source:?}: {error:?}"
            );
            assert!(source.get(error.span().start..error.span().end).is_some());
        }
    }
}

#[test]
fn preserves_ranges_collections_and_reduction_without_interpreting_them() {
    let expr = expression("[0..n periodic, ..=4, 2.., ..]");
    let ExprKind::List(items) = expr.kind else {
        panic!()
    };
    assert_eq!(items.len(), 4);
    assert!(
        matches!(&items[0].kind, ExprKind::Periodic(axis) if matches!(&axis.kind, ExprKind::Range { start: Some(_), end: Some(_), inclusive: false }))
    );
    assert!(matches!(
        items[1].kind,
        ExprKind::Range {
            start: None,
            end: Some(_),
            inclusive: true
        }
    ));
    assert!(matches!(
        items[2].kind,
        ExprKind::Range {
            start: Some(_),
            end: None,
            inclusive: false
        }
    ));
    assert!(matches!(
        items[3].kind,
        ExprKind::Range {
            start: None,
            end: None,
            inclusive: false
        }
    ));
    let expr = expression("reduce a := 0 for j in 0..=n { a + j }");
    let ExprKind::Reduce(reduction) = expr.kind else {
        panic!()
    };
    assert_eq!(reduction.accumulator.text, "a");
    assert!(matches!(&reduction.binder.kind, PatternKind::Name(name) if name.text == "j"));
    assert!(matches!(
        reduction.collection.kind,
        ExprKind::Range {
            inclusive: true,
            ..
        }
    ));
    let expr = expression("for i, [j, _] in [xs, ys] { i + j }");
    let ExprKind::For(comprehension) = expr.kind else {
        panic!()
    };
    assert!(matches!(comprehension.collection.kind, ExprKind::List(_)));
    let PatternKind::List(patterns) = comprehension.binder.kind else {
        panic!()
    };
    assert!(
        matches!(&patterns[1].kind, PatternKind::List(nested) if matches!(nested[1].kind, PatternKind::Wildcard))
    );
}
