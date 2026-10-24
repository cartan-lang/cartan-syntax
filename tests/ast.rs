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

fn assert_qualified_spans(source: &str, name: &QualifiedName) {
    assert!(source.get(name.span.start..name.span.end).is_some());
    assert_name_span(source, &name.namespace);
    assert!(!name.members.is_empty());
    for member in &name.members {
        assert_name_span(source, member);
    }
}

fn assert_use_spans(source: &str, import: &Use) {
    match &import.source {
        UseSource::Namespace(name) => assert_name_span(source, name),
        UseSource::File(path) => {
            assert_eq!(&source[path.span.start..path.span.end], path.raw);
        }
    }
    if let Some(selection) = &import.selection {
        assert!(source[selection.span.start..selection.span.end].starts_with("::"));
        match &selection.kind {
            Imports::All { span } => assert_eq!(&source[span.start..span.end], "*"),
            Imports::Names(names) => {
                for name in names {
                    assert_name_span(source, name);
                }
            }
        }
    }
    if let Some(alias) = &import.alias {
        assert_name_span(source, alias);
    }
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
        ExprKind::QualifiedName(name) => assert_qualified_spans(source, name),
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
            match function {
                NameRef::Local(name) => assert_name_span(source, name),
                NameRef::Qualified(name) => assert_qualified_spans(source, name),
            }
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
                ItemKind::Use(import) => assert_use_spans(source, import),
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
                matches!(&document.items[0].kind, ItemKind::Use(Use { source: UseSource::Namespace(Name { text, .. }), selection: Some(ImportSelection { kind: Imports::All { .. }, .. }), alias: None }) if text == "std")
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
        "use std::* with (x: 1)",
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

fn import(source: &str) -> (Use, Span) {
    let mut document =
        ast::parse_document(source).unwrap_or_else(|errors| panic!("{source:?}: {errors:?}"));
    assert_eq!(document.items.len(), 1);
    let item = document.items.pop().unwrap();
    let ItemKind::Use(import) = item.kind else {
        panic!("expected import");
    };
    assert_use_spans(source, &import);
    (import, item.span)
}

#[test]
fn import_heads_aliases_and_selectors_preserve_written_spans() {
    for head in ["std", "\"λ 🐈.cart\""] {
        for tail in ["", " as chosen", "::value", "::{first, second}", "::*"] {
            let source = format!("# π\nuse {head}{tail}\n");
            let (import, item_span) = import(&source);
            assert_eq!(
                &source[item_span.start..item_span.end],
                format!("use {head}{tail}")
            );
            match &import.source {
                UseSource::Namespace(name) => assert_eq!(name.text, "std"),
                UseSource::File(path) => {
                    assert_eq!(path.raw, head);
                    assert_eq!(path.decoded, "λ 🐈.cart");
                    assert_eq!(path.span.start, source.find('"').unwrap());
                    assert_eq!(path.span.end, path.span.start + head.len());
                }
            }
            if tail.starts_with(" as") {
                let alias = import.alias.as_ref().unwrap();
                assert_eq!(alias.text, "chosen");
                assert_eq!(alias.span.start, source.find("chosen").unwrap());
                assert!(import.selection.is_none());
            } else {
                assert!(import.alias.is_none());
            }
            if tail.starts_with("::") {
                let selection = import.selection.as_ref().unwrap();
                assert_eq!(&source[selection.span.start..selection.span.end], tail);
                assert_eq!(selection.span.start, source.find("::").unwrap());
                match &selection.kind {
                    Imports::All { .. } => assert_eq!(tail, "::*"),
                    Imports::Names(names) => {
                        let expected = if tail == "::value" {
                            vec!["value"]
                        } else {
                            vec!["first", "second"]
                        };
                        assert_eq!(
                            names
                                .iter()
                                .map(|name| name.text.as_str())
                                .collect::<Vec<_>>(),
                            expected
                        );
                    }
                }
            } else {
                assert!(import.selection.is_none());
            }
        }
    }
}

#[test]
fn single_and_braced_selectors_normalize_alike_without_erasing_spelling() {
    for selector in ["::value", "::{value}", ":: { value }"] {
        let source = format!("use std{selector}");
        let (import, _) = import(&source);
        let selection = import.selection.unwrap();
        assert_eq!(&source[selection.span.start..selection.span.end], selector);
        let Imports::Names(names) = selection.kind else {
            panic!("expected names");
        };
        assert_eq!(names.len(), 1);
        assert_eq!(names[0].text, "value");
    }
    let (plain, _) = import("use std");
    let (all, _) = import("use std::*");
    assert_eq!(plain.selection, None);
    assert!(matches!(
        all.selection,
        Some(ImportSelection {
            kind: Imports::All { .. },
            ..
        })
    ));
}

#[test]
fn quoted_paths_decode_only_the_adopted_escape_rules() {
    for (raw, decoded) in [
        (r#""line\nbreak.cart""#, "line\nbreak.cart"),
        (r#""tab\tname.cart""#, "tab\tname.cart"),
        (r#""quote\"slash\\.cart""#, "quote\"slash\\.cart"),
        (r#""unknown\r\u0041\q\λ.cart""#, "unknownru0041qλ.cart"),
        (r#""""#, ""),
    ] {
        let source = format!("# λ\nuse {raw}");
        let (import, _) = import(&source);
        let UseSource::File(path) = import.source else {
            panic!("expected file path");
        };
        assert_eq!(path.raw, raw);
        assert_eq!(path.decoded, decoded);
        assert_eq!(
            path.span,
            Span {
                start: source.find('"').unwrap(),
                end: source.len()
            }
        );
    }
}

#[test]
fn qualified_values_and_calls_preserve_segments_and_offsets() {
    let source = "# λ\nx = ns :: inner::value\ny = ns::inner::function(1, local)\n";
    let document = ast::parse_document(source).unwrap();
    for item in &document.items {
        let ItemKind::Binding(binding) = &item.kind else {
            panic!("expected binding");
        };
        assert_expr_spans(source, &binding.value);
    }
    let ItemKind::Binding(value) = &document.items[0].kind else {
        panic!();
    };
    let ExprKind::QualifiedName(name) = &value.value.kind else {
        panic!();
    };
    assert_eq!(name.namespace.text, "ns");
    assert_eq!(
        name.members
            .iter()
            .map(|member| member.text.as_str())
            .collect::<Vec<_>>(),
        ["inner", "value"]
    );
    assert_eq!(
        &source[name.span.start..name.span.end],
        "ns :: inner::value"
    );
    let ItemKind::Binding(call) = &document.items[1].kind else {
        panic!();
    };
    let ExprKind::Call {
        function: NameRef::Qualified(name),
        arguments,
    } = &call.value.kind
    else {
        panic!();
    };
    assert_eq!(name.namespace.text, "ns");
    assert_eq!(name.members[1].text, "function");
    assert_eq!(
        &source[name.span.start..name.span.end],
        "ns::inner::function"
    );
    assert_eq!(arguments.len(), 2);
    assert!(
        matches!(expression("f(1)").kind, ExprKind::Call { function: NameRef::Local(Name { text, .. }), .. } if text == "f")
    );
}

#[test]
fn import_alias_and_selection_are_a_typed_source_form_refusal() {
    for head in ["std", "\"λ.cart\""] {
        for selector in ["::*", "::name", "::{one, two}"] {
            let source = format!("# λ\nuse {head}{selector} as alias");
            assert!(!cartan_syntax::parse_document(&source).has_errors());
            assert_eq!(
                ast::parse_document(&source).unwrap_err(),
                vec![Error::Form(FormDiagnostic {
                    kind: FormDiagnosticKind::UseAliasAndImports,
                    span: Span {
                        start: source.find("use").unwrap(),
                        end: source.len()
                    },
                })]
            );
        }
    }
}

#[test]
fn import_extension_keeps_unsupported_clauses_and_grammar_failures_structured() {
    for source in [
        "use std with (x: 1)",
        "use \"file.cart\" as file with (x: 1)",
        "x: ns::Type = 1",
        "x = \"general string\"",
        "x = ns::f'(1)",
    ] {
        assert!(
            !cartan_syntax::parse_document(source).has_errors(),
            "{source}"
        );
        let errors = ast::parse_document(source).unwrap_err();
        assert!(errors.iter().all(|error| matches!(
            error,
            Error::Form(FormDiagnostic {
                kind: FormDiagnosticKind::UnsupportedConstruct { .. },
                ..
            })
        )));
        for error in errors {
            assert!(source.get(error.span().start..error.span().end).is_some());
        }
    }
    for source in [
        "ns::f(x) = x",
        "use",
        "use \"unfinished",
        "use std::",
        "use std as",
        "use std::{",
    ] {
        let cst = cartan_syntax::parse_document(source);
        assert!(cst.has_errors(), "{source}");
        assert_eq!(
            ast::parse_document(source).unwrap_err(),
            cst.diagnostics
                .into_iter()
                .map(Error::Grammar)
                .collect::<Vec<_>>()
        );
    }
}
