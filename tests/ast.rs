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
        ExprKind::String(string) => {
            assert_eq!(&source[expr.span.start..expr.span.end], string.raw);
        }
        ExprKind::Template(template) => {
            let mut previous_end = expr.span.start;
            for piece in &template.pieces {
                let piece_span = match piece {
                    Piece::Text(text) => {
                        assert_eq!(&source[text.span.start..text.span.end], text.raw);
                        assert!(!text.value.is_empty());
                        text.span
                    }
                    Piece::Hole(hole) => {
                        let written = &source[hole.span.start..hole.span.end];
                        assert!(written.starts_with('{') && written.ends_with('}'));
                        assert_expr_spans(source, &hole.expr);
                        if let Some(format) = &hole.format {
                            assert_eq!(
                                &source[format.span.start..format.span.end],
                                format!(":{}", format.raw)
                            );
                        }
                        hole.span
                    }
                };
                assert!(previous_end <= piece_span.start && piece_span.end <= expr.span.end);
                previous_end = piece_span.end;
            }
        }
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
        ExprKind::Map(entries) => {
            for entry in entries {
                assert!(source.get(entry.span.start..entry.span.end).is_some());
                assert_name_span(source, &entry.key);
                assert_expr_spans(source, &entry.value);
            }
        }
        ExprKind::MapJoin {
            left,
            right,
            operator_span,
        } => {
            assert_eq!(&source[operator_span.start..operator_span.end], "<>");
            assert_expr_spans(source, left);
            assert_expr_spans(source, right);
        }
        ExprKind::Configurable { settings, body } => {
            for atom in settings {
                assert_eq!(
                    &source[atom.operator_span.start..atom.operator_span.end],
                    ":"
                );
                assert_expr_spans(source, &atom.value);
            }
            assert_body_spans(source, body);
        }
        ExprKind::Configure {
            kind,
            target,
            settings,
            operator_span,
        } => {
            assert_eq!(
                &source[operator_span.start..operator_span.end],
                match kind {
                    ConfigureKind::Colon => ":",
                    ConfigureKind::Dot => ".",
                }
            );
            assert_expr_spans(source, target);
            assert_expr_spans(source, settings);
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
                ItemKind::Struct(_) => panic!("workloads declare no structs"),
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
    let Some(ParameterAnnotation::Value(annotation)) = &function.parameters[1].annotation else {
        panic!()
    };
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

#[test]
fn named_maps_preserve_empty_nested_duplicate_entries_and_utf8_spans() {
    assert!(matches!(expression("{}").kind, ExprKind::Map(entries) if entries.is_empty()));
    let source = "{# λ\n a: 1, a: {b: value}, other: xs[0]}";
    let expr = expression(source);
    assert_eq!(&source[expr.span.start..expr.span.end], source);
    let ExprKind::Map(entries) = expr.kind else {
        panic!()
    };
    assert_eq!(entries.len(), 3);
    assert_eq!(entries[0].key.text, "a");
    assert_eq!(entries[1].key.text, "a");
    assert_ne!(entries[0].key.span, entries[1].key.span);
    assert_eq!(
        &source[entries[1].span.start..entries[1].span.end],
        "a: {b: value}"
    );
    assert!(matches!(entries[1].value.kind, ExprKind::Map(_)));
    assert!(matches!(entries[2].value.kind, ExprKind::Index { .. }));
    assert!(matches!(expression("{value}").kind, ExprKind::Body(_)));
}

#[test]
fn map_join_keeps_operand_order_additive_precedence_and_token_spans() {
    let expr = expression("a <> b <> c + d * e");
    let ExprKind::Binary {
        operator: BinaryOperator::Add,
        left,
        right,
    } = expr.kind
    else {
        panic!()
    };
    let ExprKind::MapJoin {
        left: first,
        right: third,
        ..
    } = left.kind
    else {
        panic!()
    };
    assert!(matches!(first.kind, ExprKind::MapJoin { .. }));
    assert!(matches!(third.kind, ExprKind::Name(Name { text, .. }) if text == "c"));
    assert!(matches!(
        right.kind,
        ExprKind::Binary {
            operator: BinaryOperator::Multiply,
            ..
        }
    ));
    let source = "{a:1}\n  <> {b:2}";
    let ExprKind::MapJoin { operator_span, .. } = expression(source).kind else {
        panic!()
    };
    assert_eq!(operator_span.start, source.find("<>").unwrap());
}

#[test]
fn configurable_constructor_preserves_atoms_body_order_and_braces() {
    let source = ":base\n :{a: 1} :over {\n value := a + 1\n value\n tail := 2\n}";
    let expr = expression(source);
    assert_eq!(
        expr.span,
        Span {
            start: 0,
            end: source.len()
        }
    );
    let ExprKind::Configurable { settings, body } = expr.kind else {
        panic!()
    };
    assert_eq!(settings.len(), 3);
    assert!(matches!(&settings[0].value.kind, ExprKind::Name(name) if name.text == "base"));
    assert!(matches!(settings[1].value.kind, ExprKind::Map(_)));
    assert!(matches!(&settings[2].value.kind, ExprKind::Name(name) if name.text == "over"));
    assert_eq!(
        &source[body.span.start..body.span.end],
        "{\n value := a + 1\n value\n tail := 2\n}"
    );
    assert!(matches!(
        &body.items[..],
        [BodyItem::Local(_), BodyItem::Result(_), BodyItem::Local(_)]
    ));
    for source in [
        ":{} {1}",
        ":(base <> over) {c}",
        ":ns::settings {c}",
        ":choose(x) {c}",
        ":settings[i] {c}",
        ":{c:1} {:other {c}}",
    ] {
        assert!(
            matches!(expression(source).kind, ExprKind::Configurable { .. }),
            "{source}"
        );
    }
    let source = "base = {c: 1}\nf(x) = :base :{d: 2} {local := x + c\n local + d}\nvalue = f(3):{c: 4}.{d: live}";
    let document = ast::parse_document(source).unwrap();
    assert_eq!(document.items.len(), 3);
    let ItemKind::Function(function) = &document.items[1].kind else {
        panic!()
    };
    assert!(matches!(function.body.kind, ExprKind::Configurable { .. }));
    assert_expr_spans(source, &function.body);
    let ItemKind::Binding(binding) = &document.items[2].kind else {
        panic!()
    };
    assert!(matches!(
        binding.value.kind,
        ExprKind::Configure {
            kind: ConfigureKind::Dot,
            ..
        }
    ));
    assert_expr_spans(source, &binding.value);
}

#[test]
fn configuration_chains_bind_left_and_keep_atom_call_index_and_dot_literal() {
    let source = "value :base :settings[i].{a: live} :next";
    let ExprKind::Configure {
        kind: ConfigureKind::Colon,
        target,
        settings,
        ..
    } = expression(source).kind
    else {
        panic!()
    };
    assert!(matches!(settings.kind, ExprKind::Name(name) if name.text == "next"));
    let ExprKind::Configure {
        kind: ConfigureKind::Dot,
        target,
        settings,
        ..
    } = target.kind
    else {
        panic!()
    };
    assert!(matches!(settings.kind, ExprKind::Map(_)));
    let ExprKind::Configure {
        kind: ConfigureKind::Colon,
        target,
        settings,
        ..
    } = target.kind
    else {
        panic!()
    };
    assert!(matches!(settings.kind, ExprKind::Index { .. }));
    assert!(matches!(
        target.kind,
        ExprKind::Configure {
            kind: ConfigureKind::Colon,
            ..
        }
    ));
    for source in [
        "value:make(x)",
        "value:ns::settings",
        "value:(base<>over)",
        "value.{}",
        "(:{c:1}{c}):{c:2}.{c:live}",
        "value\n .{a:1}\n :next",
    ] {
        assert!(
            matches!(expression(source).kind, ExprKind::Configure { .. }),
            "{source}"
        );
    }
    let ExprKind::Binary {
        operator: BinaryOperator::Add,
        right,
        ..
    } = expression("a + value:settings").kind
    else {
        panic!()
    };
    assert!(matches!(right.kind, ExprKind::Configure { .. }));
}

#[test]
fn configurable_forms_preserve_supported_values_and_reject_unhandled_map_forms() {
    // Scope, types and configurable settings are graph decisions, not syntax checks.
    expression(":{x: (:base {y}), y: if flag then a else b} {missing}");
    for (source, kind) in [
        ("{(key): 1}", "computed_key"),
        ("{each k in keys {(k): 1}}", "map_walk"),
    ] {
        let errors = ast::parse_expression(source).unwrap_err();
        assert!(
            matches!(&errors[..], [Error::Form(FormDiagnostic { kind: FormDiagnosticKind::UnsupportedConstruct { kind: actual }, .. })] if actual == kind),
            "{source}: {errors:?}"
        );
    }
    for source in [":base {}", ":base {1,2}"] {
        let errors = ast::parse_expression(source).unwrap_err();
        assert!(
            matches!(
                &errors[..],
                [Error::Form(FormDiagnostic {
                    kind: FormDiagnosticKind::BodyResultCount { .. },
                    ..
                })]
            ),
            "{source}: {errors:?}"
        );
    }
    for source in [
        ":",
        ":base",
        "value.",
        "value.(settings)",
        "value.name",
        ":base {x := 1",
    ] {
        let tree = cartan_syntax::parse_expression(source);
        assert!(tree.has_errors(), "{source}");
        let errors = ast::parse_expression(source).unwrap_err();
        assert_eq!(
            errors,
            tree.diagnostics
                .into_iter()
                .map(Error::Grammar)
                .collect::<Vec<_>>(),
            "{source}"
        );
    }
}

fn template(source: &str) -> Template {
    match expression(source).kind {
        ExprKind::Template(template) => template,
        kind => panic!("{source:?}: {kind:?}"),
    }
}

fn text(piece: &Piece) -> &str {
    match piece {
        Piece::Text(text) => &text.value,
        Piece::Hole(hole) => panic!("expected text, found {hole:?}"),
    }
}

fn hole(piece: &Piece) -> &Hole {
    match piece {
        Piece::Hole(hole) => hole,
        Piece::Text(text) => panic!("expected a hole, found {text:?}"),
    }
}

#[test]
fn strings_keep_raw_spelling_beside_decoded_unicode_text() {
    let source = r#""λ \"🐈\" \n\t\\ \x {x}""#;
    let ExprKind::String(string) = expression(source).kind else {
        panic!("{source}");
    };
    assert_eq!(string.form, StringForm::Quoted);
    assert_eq!(string.raw, source);
    assert_eq!(string.value, "λ \"🐈\" \n\t\\ x {x}");
    let source = "\"\"\"\n    λ \\n \"so\"\n      {x}\n    \"\"\"";
    let ExprKind::String(string) = expression(source).kind else {
        panic!("{source}");
    };
    assert_eq!(string.form, StringForm::Triple);
    assert_eq!(string.value, "λ \\n \"so\"\n  {x}\n");
    let ExprKind::String(empty) = expression("\"\"").kind else {
        panic!("empty");
    };
    assert_eq!(empty.value, "");
    // A string value is not an import path, and a binding's value can be a string.
    let document = ast::parse_document("x = \"a\"\ny = \"\"\"\n  b\n  \"\"\"\n").unwrap();
    for item in &document.items {
        let ItemKind::Binding(binding) = &item.kind else {
            panic!("{item:?}");
        };
        assert!(matches!(binding.value.kind, ExprKind::String(_)));
    }
    // A standalone triple string is doc prose, not a string expression.
    assert!(matches!(
        ast::parse_document("\"\"\"Prose.\"\"\"\nx = 1\n").unwrap_err().as_slice(),
        [Error::Form(FormDiagnostic {
            kind: FormDiagnosticKind::UnsupportedConstruct { kind },
            ..
        })] if kind == "doc_declaration"
    ));
}

#[test]
fn templates_merge_text_and_keep_ordinary_hole_expressions() {
    let template_ = template("f\"x = {x:.2} and {{literal}} {a + f(b)}\"");
    assert_eq!(template_.form, StringForm::Quoted);
    assert_eq!(template_.pieces.len(), 4);
    assert_eq!(text(&template_.pieces[0]), "x = ");
    let first = hole(&template_.pieces[1]);
    assert!(matches!(&first.expr.kind, ExprKind::Name(name) if name.text == "x"));
    let format = first.format.as_ref().unwrap();
    assert_eq!(format.raw, ".2");
    assert_eq!(format.precision.as_ref().map(|p| p.digits), Some(2));
    assert_eq!(format.notation, None);
    // Escapes and chunks between holes form one piece with written spelling.
    let Piece::Text(middle) = &template_.pieces[2] else {
        panic!();
    };
    assert_eq!(middle.value, " and {literal} ");
    assert_eq!(middle.raw, " and {{literal}} ");
    let last = hole(&template_.pieces[3]);
    assert!(last.format.is_none());
    assert!(matches!(
        last.expr.kind,
        ExprKind::Binary {
            operator: BinaryOperator::Add,
            ..
        }
    ));
    // Empty, hole-only and adjacent holes produce no empty text pieces.
    assert!(template("f\"\"").pieces.is_empty());
    let adjacent = template("f\"{a}{b}\"");
    assert_eq!(adjacent.pieces.len(), 2);
    assert!(adjacent
        .pieces
        .iter()
        .all(|piece| matches!(piece, Piece::Hole(_))));
    assert_eq!(template("f\"plain\"").pieces.len(), 1);
    // Quoted escapes decode; untagged braces are literal text.
    assert_eq!(text(&template("f\"a\\n\\\"b\\{\"").pieces[0]), "a\n\"b{");
    // Nested templates and strings are ordinary hole expressions.
    let nested = template("f\"<{f\"{x}\"}|{\"s\"}>\"");
    assert!(matches!(
        hole(&nested.pieces[1]).expr.kind,
        ExprKind::Template(_)
    ));
    assert!(matches!(
        hole(&nested.pieces[3]).expr.kind,
        ExprKind::String(_)
    ));
}

#[test]
fn template_spans_are_utf8_byte_offsets() {
    let source = "f\"λ🐈 {e:.3sci} ü\"";
    let template_ = template(source);
    let Piece::Text(first) = &template_.pieces[0] else {
        panic!();
    };
    assert_eq!(&source[first.span.start..first.span.end], "λ🐈 ");
    let hole_ = hole(&template_.pieces[1]);
    assert_eq!(&source[hole_.span.start..hole_.span.end], "{e:.3sci}");
    let format = hole_.format.as_ref().unwrap();
    assert_eq!(&source[format.span.start..format.span.end], ":.3sci");
    let precision = format.precision.as_ref().unwrap();
    assert_eq!(&source[precision.span.start..precision.span.end], ".3");
    assert_eq!(
        (precision.digits, format.notation),
        (3, Some(Notation::Sci))
    );
    assert_eq!(text(&template_.pieces[2]), " ü");
}

#[test]
fn raw_templates_dedent_across_holes_and_keep_backslashes() {
    let source = "f\"\"\"\n    x is {x}\n      \\n {y:eng}\n    \"\"\"";
    let template_ = template(source);
    assert_eq!(template_.form, StringForm::Triple);
    assert_eq!(text(&template_.pieces[0]), "x is ");
    assert_eq!(text(&template_.pieces[2]), "\n  \\n ");
    let format = hole(&template_.pieces[3]).format.as_ref().unwrap();
    assert_eq!(
        (format.precision.as_ref(), format.notation),
        (None, Some(Notation::Eng))
    );
    assert_eq!(text(&template_.pieces[4]), "\n");
    // Braces are still escaped by doubling in the raw form.
    assert_eq!(
        text(&template("f\"\"\"{{ \"q\" }}\"\"\"").pieces[0]),
        "{ \"q\" }"
    );
}

#[test]
fn malformed_formats_are_typed_form_errors_with_spans() {
    for (source, expected, written) in [
        ("f\"{1.0:log}\"", "notation", ":log"),
        ("f\"{1.0:.x}\"", "precision", ":."),
        ("f\"{1.0:.}\"", "precision", ":."),
        ("f\"{1.0:.99999999999}\"", "precision", ":.99999999999"),
        ("f\"{1.0:.3sc}\"", "notation", "sc"),
        ("f\"{1.0:3}\"", "notation", ":3"),
    ] {
        let errors = ast::parse_expression(source).unwrap_err();
        let [Error::Form(diagnostic)] = errors.as_slice() else {
            panic!("{source}: {errors:?}");
        };
        let found = &source[diagnostic.span.start..diagnostic.span.end];
        match (&diagnostic.kind, expected) {
            (FormDiagnosticKind::FormatPrecision { written: text }, "precision")
            | (FormDiagnosticKind::FormatNotation { written: text }, "notation") => {
                assert_eq!(found, text, "{source}");
                assert!(written.ends_with(found), "{source}: {found}");
            }
            (kind, _) => panic!("{source}: {kind:?}"),
        }
    }
}

#[test]
fn struct_declarations_keep_ordered_duplicate_entries_and_named_annotations() {
    let source = "struct P = {x: Real, n: Int, x: Vector<Real, 2>, q: Q}\np: P = {x: 1}\n";
    let document = ast::parse_document(source).unwrap();
    let ItemKind::Struct(declaration) = &document.items[0].kind else {
        panic!("{:?}", document.items[0]);
    };
    assert_name_span(source, &declaration.name);
    let names: Vec<_> = declaration
        .entries
        .iter()
        .map(|entry| entry.name.text.as_str())
        .collect();
    assert_eq!(names, ["x", "n", "x", "q"]);
    for entry in &declaration.entries {
        assert_name_span(source, &entry.name);
        assert!(source[entry.span.start..entry.span.end].starts_with(&entry.name.text));
    }
    assert!(matches!(
        &declaration.entries[2].annotation.kind,
        AnnotationKind::Apply { name, .. } if name.text == "Vector"
    ));
    let ItemKind::Binding(binding) = &document.items[1].kind else {
        panic!();
    };
    assert!(matches!(
        &binding.annotation.as_ref().unwrap().kind,
        AnnotationKind::Named(name) if name.text == "P"
    ));
    assert!(matches!(binding.value.kind, ExprKind::Map(_)));
    // An inline struct may stand as an entry's type.
    let document = ast::parse_document("struct S = {p: {x: Real}}").unwrap();
    let ItemKind::Struct(declaration) = &document.items[0].kind else {
        panic!();
    };
    assert!(matches!(
        &declaration.entries[0].annotation.kind,
        AnnotationKind::Struct(entries) if entries.len() == 1 && entries[0].name.text == "x"
    ));
}

#[test]
fn raw_template_boundaries_survive_control_characters_beside_holes() {
    // Legacy split text runs on U+0001; text may contain it here.
    let source = "f\"\"\"\n  a\u{1}{x}\u{1}b\n  \u{1}{y}\n  \"\"\"";
    let template_ = template(source);
    assert_eq!(template_.pieces.len(), 5);
    assert_eq!(text(&template_.pieces[0]), "a\u{1}");
    assert!(
        matches!(&hole(&template_.pieces[1]).expr.kind, ExprKind::Name(name) if name.text == "x")
    );
    assert_eq!(text(&template_.pieces[2]), "\u{1}b\n\u{1}");
    assert!(
        matches!(&hole(&template_.pieces[3]).expr.kind, ExprKind::Name(name) if name.text == "y")
    );
    assert_eq!(text(&template_.pieces[4]), "\n");
    let ExprKind::String(string) = expression("\"\"\"\n  \u{1}x\n  \"\"\"").kind else {
        panic!();
    };
    assert_eq!(string.value, "\u{1}x\n");
}

#[test]
fn raw_templates_keep_backslashes_and_quotes_beside_holes_and_closers() {
    // A raw backslash before a hole, or before the closer, is text.
    let source = "f\"\"\"\\{n}\"\"\"";
    let template_ = template(source);
    let Piece::Text(slash) = &template_.pieces[0] else {
        panic!("{template_:?}");
    };
    assert_eq!((slash.value.as_str(), slash.raw.as_str()), ("\\", "\\"));
    assert_eq!(&source[slash.span.start..slash.span.end], "\\");
    let hole_ = hole(&template_.pieces[1]);
    assert_eq!(&source[hole_.span.start..hole_.span.end], "{n}");
    assert_eq!(template_.pieces.len(), 2);
    let trailing = template("f\"\"\"abc\\\"\"\"");
    assert_eq!(trailing.pieces.len(), 1);
    assert_eq!(text(&trailing.pieces[0]), "abc\\");
    // A quotation mark directly before a hole is text, not a closer.
    let source = "f\"\"\"\"{n}\"\"\"";
    let template_ = template(source);
    let Piece::Text(quote) = &template_.pieces[0] else {
        panic!("{template_:?}");
    };
    assert_eq!(quote.value, "\"");
    assert_eq!(&source[quote.span.start..quote.span.end], "\"");
    assert!(
        matches!(&hole(&template_.pieces[1]).expr.kind, ExprKind::Name(name) if name.text == "n")
    );
    // Quote runs merge with neighbouring chunks and doubled braces.
    let source = "f\"\"\"a\"\"{n}\"}}\"\"{{λ\"\"\"";
    let template_ = template(source);
    assert_eq!(template_.pieces.len(), 3);
    let Piece::Text(before) = &template_.pieces[0] else {
        panic!();
    };
    assert_eq!(before.value, "a\"\"");
    assert_eq!(&source[before.span.start..before.span.end], "a\"\"");
    let Piece::Text(after) = &template_.pieces[2] else {
        panic!();
    };
    assert_eq!(after.value, "\"}\"\"{λ");
    assert_eq!(after.raw, "\"}}\"\"{{λ");
    assert_eq!(&source[after.span.start..after.span.end], after.raw);
    // The quoted form keeps its backslash escapes, including before a brace.
    assert_eq!(text(&template("f\"a\\{\\\"\"").pieces[0]), "a{\"");
}

fn function(source: &str) -> Function {
    let document = ast::parse_document(source).unwrap_or_else(|errors| panic!("{errors:?}"));
    let Some(ItemKind::Function(function)) =
        document.items.into_iter().map(|item| item.kind).next_back()
    else {
        panic!("{source}");
    };
    function
}

fn signature(parameter: &Parameter) -> &Signature {
    match &parameter.annotation {
        Some(ParameterAnnotation::Signature(signature)) => signature,
        other => panic!("expected a signature, found {other:?}"),
    }
}

fn named(annotation: &Annotation) -> &str {
    match &annotation.kind {
        AnnotationKind::Named(name) => &name.text,
        kind => panic!("expected a named type, found {kind:?}"),
    }
}

#[test]
fn signature_parameters_keep_ordered_slots_result_and_spans() {
    let source = "call_something(g: (Real, Real) -> Real, x: Real) = { g(1.0, x) }\n";
    let function_ = function(source);
    let text = |span: Span| &source[span.start..span.end];
    let g = &function_.parameters[0];
    assert_eq!(text(g.span), "g: (Real, Real) -> Real");
    let signature_ = signature(g);
    assert_eq!(text(signature_.span), "(Real, Real) -> Real");
    assert_eq!(text(signature_.arrow_span), "->");
    let arguments: Vec<_> = signature_.arguments.iter().map(named).collect();
    assert_eq!(arguments, ["Real", "Real"]);
    assert_eq!(text(signature_.arguments[1].span), "Real");
    assert!(signature_.arguments[1].span.start > signature_.arguments[0].span.end);
    assert_eq!(named(&signature_.result), "Real");
    assert!(signature_.result.span.start > signature_.arrow_span.end);
    // Ordinary value and unannotated parameters are unchanged.
    assert!(matches!(
        &function_.parameters[1].annotation,
        Some(ParameterAnnotation::Value(annotation)) if named(annotation) == "Real"
    ));
    assert!(function("f(x) = x").parameters[0].annotation.is_none());
    // Zero and one argument slots keep their parentheses.
    let zero = function("f(g: () -> Real) = g()");
    assert!(signature(&zero.parameters[0]).arguments.is_empty());
    let one = function("f(g: (Int) -> Real, h: (Real, Int, Real) -> Int) = 1");
    assert_eq!(signature(&one.parameters[0]).arguments.len(), 1);
    assert_eq!(signature(&one.parameters[1]).arguments.len(), 3);
    assert_eq!(named(&signature(&one.parameters[1]).result), "Int");
}

#[test]
fn signature_slots_take_generic_annotations_at_utf8_offsets() {
    let source = "label = \"λ 🐈\"\nf(g: (Array<Real, 1>, Vector<Real, 2>) -> Mat<3, 3>) = 1\n";
    let function_ = function(source);
    let signature_ = signature(&function_.parameters[0]);
    let text = |span: Span| &source[span.start..span.end];
    assert_eq!(
        text(signature_.span),
        "(Array<Real, 1>, Vector<Real, 2>) -> Mat<3, 3>"
    );
    assert_eq!(text(signature_.arguments[0].span), "Array<Real, 1>");
    assert_eq!(text(signature_.arguments[1].span), "Vector<Real, 2>");
    assert_eq!(text(signature_.result.span), "Mat<3, 3>");
    assert_eq!(text(signature_.arrow_span), "->");
    assert!(matches!(
        &signature_.arguments[0].kind,
        AnnotationKind::Apply { name, arguments } if name.text == "Array" && arguments.len() == 2
    ));
}

#[test]
fn signatures_outside_parameters_are_typed_by_position_at_the_arrow() {
    for (source, expected, written) in [
        (
            "x: (Real) -> Real = 1",
            SignaturePosition::Binding,
            "(Real) -> Real",
        ),
        (
            "param p: () -> Real = 1",
            SignaturePosition::Binding,
            "() -> Real",
        ),
        (
            "x = { c: (Real) -> Real := 2\n c }",
            SignaturePosition::Local,
            "(Real) -> Real",
        ),
        (
            "x: List<(Real) -> Real> = 1",
            SignaturePosition::TypeArgument,
            "(Real) -> Real",
        ),
        (
            "f(g: ((Real) -> Real, Real) -> Real) = 1",
            SignaturePosition::SignatureArgument,
            "(Real) -> Real",
        ),
        (
            "f(g: (Real) -> (Int) -> Real) = 1",
            SignaturePosition::SignatureResult,
            "(Int) -> Real",
        ),
    ] {
        let errors = ast::parse_document(source).unwrap_err();
        let [Error::Form(FormDiagnostic {
            kind:
                FormDiagnosticKind::SignatureOutsideParameter {
                    position,
                    signature_span,
                },
            span,
        })] = errors.as_slice()
        else {
            panic!("{source}: {errors:?}");
        };
        assert_eq!(*position, expected, "{source}");
        assert_eq!(&source[span.start..span.end], "->", "{source}");
        assert_eq!(
            &source[signature_span.start..signature_span.end],
            written,
            "{source}"
        );
    }
    // The unary spelling is equally refused outside a parameter.
    for (source, expected) in [
        ("x: Real -> Real = 1", SignaturePosition::Binding),
        ("x = { c: Real -> Real := 2\n c }", SignaturePosition::Local),
        (
            "f(g: Real -> Int -> Real) = 1",
            SignaturePosition::SignatureResult,
        ),
    ] {
        let errors = ast::parse_document(source).unwrap_err();
        let [Error::Form(FormDiagnostic {
            kind:
                FormDiagnosticKind::SignatureOutsideParameter {
                    position,
                    signature_span,
                },
            span,
        })] = errors.as_slice()
        else {
            panic!("{source}: {errors:?}");
        };
        assert_eq!(*position, expected, "{source}");
        assert_eq!(&source[span.start..span.end], "->");
        assert!(source[signature_span.start..signature_span.end].ends_with("Real"));
    }
    // Struct entries and fiber components take a bare fiber type, so an arrow
    // there stays a structured grammar error.
    for source in [
        "struct S = {p: (Real) -> Real}",
        "fiber F = (p: (Real) -> Real)",
    ] {
        let errors = ast::parse_document(source).unwrap_err();
        assert!(
            errors
                .iter()
                .all(|error| matches!(error, Error::Grammar(_))),
            "{source}: {errors:?}"
        );
    }
    // Optional annotations and namespace holes remain unsupported.
    for source in [
        "f(g: (Real|None) -> Real) = 1",
        "namespace a() { f(g: (Real) -> Real) = 1 }",
    ] {
        let errors = ast::parse_document(source).unwrap_err();
        assert!(
            errors.iter().all(|error| matches!(
                error,
                Error::Form(FormDiagnostic {
                    kind: FormDiagnosticKind::UnsupportedConstruct { .. },
                    ..
                })
            )),
            "{source}: {errors:?}"
        );
    }
}

fn parameter_signature(source: &str) -> Signature {
    signature(&function(source).parameters[0]).clone()
}

fn without_spans(signature: &Signature) -> String {
    // Debug text with every span removed, for comparing shapes only.
    let text = format!("{:?}", (&signature.arguments, &signature.result));
    let mut out = String::new();
    let mut rest = text.as_str();
    while let Some(at) = rest.find("span: Span {") {
        out.push_str(&rest[..at]);
        rest = &rest[at + rest[at..].find('}').unwrap() + 1..];
    }
    out + rest
}

#[test]
fn unary_signatures_match_parenthesized_ones_with_written_spans() {
    let source = "f(x: Real, g: Real -> {a: Real, b: Real}) = x";
    let function_ = function(source);
    let text = |span: Span| &source[span.start..span.end];
    let unary = signature(&function_.parameters[1]);
    assert_eq!(text(unary.span), "Real -> {a: Real, b: Real}");
    assert_eq!(text(unary.arrow_span), "->");
    assert_eq!(text(unary.arguments[0].span), "Real");
    assert_eq!(text(unary.result.span), "{a: Real, b: Real}");
    let AnnotationKind::Struct(entries) = &unary.result.kind else {
        panic!("{unary:?}");
    };
    let names: Vec<_> = entries
        .iter()
        .map(|entry| entry.name.text.as_str())
        .collect();
    assert_eq!(names, ["a", "b"]);
    // The unary and parenthesized spellings give the same shape.
    for (unary, parenthesized) in [
        ("f(g: Real -> Real) = 1", "f(g: (Real) -> Real) = 1"),
        (
            "f(g: Array<Real, 1> -> Int) = 1",
            "f(g: (Array<Real, 1>) -> Int) = 1",
        ),
        (
            "f(g: {a: Real} -> {b: Int}) = 1",
            "f(g: ({a: Real}) -> {b: Int}) = 1",
        ),
    ] {
        let (left, right) = (
            parameter_signature(unary),
            parameter_signature(parenthesized),
        );
        assert_eq!(left.arguments.len(), 1);
        assert_eq!(without_spans(&left), without_spans(&right), "{unary}");
    }
    // Inline products, named signature formals and optional arguments stay out.
    for (source, kind) in [
        ("f(g: (x: Real) -> Real) = 1", "component"),
        ("f(p: (a: Real, b: Real)) = p", "component"),
        ("f(g: Real|None -> Real) = 1", "fiber_type"),
    ] {
        let errors = ast::parse_document(source).unwrap_err();
        assert!(
            matches!(
                errors.as_slice(),
                [Error::Form(FormDiagnostic {
                    kind: FormDiagnosticKind::UnsupportedConstruct { kind: found },
                    ..
                })] if found == kind
            ),
            "{source}: {errors:?}"
        );
    }
}

#[test]
fn inline_structs_nest_keep_order_duplicates_empty_forms_and_brace_spans() {
    let source = "label = \"λ\"\nf(x: {b: Int, a: {}, b: Array<{c: Real}, 1>}) = x\n";
    let document = ast::parse_document(source).unwrap();
    let ItemKind::Function(function_) = &document.items[1].kind else {
        panic!();
    };
    let Some(ParameterAnnotation::Value(annotation)) = &function_.parameters[0].annotation else {
        panic!();
    };
    let text = |span: Span| &source[span.start..span.end];
    assert_eq!(
        text(annotation.span),
        "{b: Int, a: {}, b: Array<{c: Real}, 1>}"
    );
    let AnnotationKind::Struct(entries) = &annotation.kind else {
        panic!();
    };
    let names: Vec<_> = entries
        .iter()
        .map(|entry| entry.name.text.as_str())
        .collect();
    assert_eq!(names, ["b", "a", "b"]);
    for entry in entries {
        assert_name_span(source, &entry.name);
        assert!(text(entry.span).starts_with(&entry.name.text));
    }
    assert!(
        matches!(&entries[1].annotation.kind, AnnotationKind::Struct(empty) if empty.is_empty())
    );
    assert_eq!(text(entries[1].annotation.span), "{}");
    let AnnotationKind::Apply { arguments, .. } = &entries[2].annotation.kind else {
        panic!();
    };
    let TypeArgument::Type(element) = &arguments[0] else {
        panic!();
    };
    assert_eq!(text(element.span), "{c: Real}");
    assert!(matches!(&element.kind, AnnotationKind::Struct(inner) if inner[0].name.text == "c"));
    // Bindings, locals, fiber components and generic arguments admit the form;
    // graph decides where a struct is semantically eligible.
    for source in [
        "x: {a: Real} = 1",
        "x = { c: {a: Real} := 2\n c }",
        "fiber F = (p: {x: Real})",
        "x: List<{a: Real}> = 1",
    ] {
        assert!(ast::parse_document(source).is_ok(), "{source}");
    }
}
