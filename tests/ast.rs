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

fn assert_key_spans(source: &str, key: &ComponentKey) {
    match key {
        ComponentKey::Name(name) => assert_name_span(source, name),
        ComponentKey::Position(number) => {
            assert_eq!(&source[number.span.start..number.span.end], number.text)
        }
        ComponentKey::Pair { row, column, span } => {
            let text = &source[span.start..span.end];
            assert!(text.starts_with('(') && text.ends_with(')'));
            for number in [row, column] {
                assert_eq!(&source[number.span.start..number.span.end], number.text);
            }
        }
    }
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
        PatternKind::Constructor { head, items } => {
            assert_name_span(source, head);
            assert!(text.starts_with(&head.text));
            for pattern in items {
                assert_pattern_spans(source, pattern);
            }
        }
        PatternKind::Components(components) => {
            for component in components {
                assert_key_spans(source, &component.key);
                assert_pattern_spans(source, &component.pattern);
            }
        }
        PatternKind::Entries(entries) => {
            for entry in entries {
                assert_name_span(source, &entry.key);
                if let Some(pattern) = &entry.pattern {
                    assert_pattern_spans(source, pattern);
                }
            }
        }
    }
}

fn assert_variant_spans(source: &str, variants: &[Variant]) {
    for variant in variants {
        assert_name_span(source, &variant.name);
        let written = &source[variant.span.start..variant.span.end];
        assert!(written.starts_with(&variant.name.text));
        if let Some(payload) = &variant.payload {
            assert!(
                variant.span.start < payload.span.start && payload.span.end == variant.span.end
            );
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
            for expr in [condition, consequence].into_iter().chain(alternative) {
                assert_expr_spans(source, expr);
            }
        }
        ExprKind::None => assert_eq!(&source[expr.span.start..expr.span.end], "none"),
        ExprKind::Default {
            value,
            fallback,
            operator_span,
        } => {
            assert_eq!(&source[operator_span.start..operator_span.end], "??");
            assert_expr_spans(source, value);
            assert_expr_spans(source, fallback);
        }
        ExprKind::Lift { value, mark_span } => {
            assert_eq!(&source[mark_span.start..mark_span.end], "?");
            assert_eq!(mark_span.end, expr.span.end);
            assert_expr_spans(source, value);
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
        ExprKind::Construct {
            head,
            variant,
            payload,
        } => {
            match head {
                ConstructorHead::Named(NameRef::Local(name)) => assert_name_span(source, name),
                ConstructorHead::Named(NameRef::Qualified(name)) => {
                    assert_qualified_spans(source, name)
                }
                ConstructorHead::Inline { variants, span } => {
                    let written = &source[span.start..span.end];
                    assert!(written.starts_with('(') && written.ends_with(')'));
                    assert_variant_spans(source, variants);
                }
            }
            assert_name_span(source, variant);
            if let Some(payload) = payload {
                assert_expr_spans(source, payload);
            }
        }
        ExprKind::Index { target, indices } => {
            assert_expr_spans(source, target);
            for expr in indices {
                assert_expr_spans(source, expr);
            }
        }
        ExprKind::Projection { key, value } => {
            assert_key_spans(source, key);
            assert_expr_spans(source, value);
        }
        ExprKind::Product(entries) => {
            for entry in entries {
                assert!(source.get(entry.span.start..entry.span.end).is_some());
                assert_key_spans(source, &entry.key);
                assert_expr_spans(source, &entry.value);
            }
        }
        ExprKind::Update {
            base,
            overrides,
            operator_span,
        } => {
            assert_eq!(&source[operator_span.start..operator_span.end], "but");
            assert_expr_spans(source, base);
            assert_expr_spans(source, overrides);
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
        ExprKind::Match(written) => {
            assert_expr_spans(source, &written.scrutinee);
            for arm in &written.arms {
                assert_name_span(source, &arm.variant);
                assert!(source[arm.span.start..arm.span.end].starts_with(&arm.variant.text));
                if let Some(payload) = &arm.payload {
                    assert_pattern_spans(source, payload);
                }
                assert_expr_spans(source, &arm.body);
                assert_eq!(arm.body.span.end, arm.span.end);
            }
            if let Some(otherwise) = &written.otherwise {
                assert!(source[otherwise.span.start..].starts_with("else"));
                assert_expr_spans(source, &otherwise.body);
                assert_eq!(otherwise.body.span.end, otherwise.span.end);
            }
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
                ItemKind::Sum(_) => panic!("workloads declare no sums"),
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
    assert!(matches!(key, ComponentKey::Name(name) if name.text == "pre"));
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
        "f(x: Real = 1) = x",
        "f(x) : { pad: 1 } = x",
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
        "x = (A: Real)(1)",
        "x = (A: Real)(A: 1)",
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
    // Text runs are kept apart without any separator character, so text may contain U+0001.
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
    // Namespace holes remain unsupported.
    let source = "namespace a() { f(g: (Real) -> Real) = 1 }";
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
    // Inline products and named signature formals stay out.
    for (source, kind) in [
        ("f(g: (x: Real) -> Real) = 1", "component"),
        ("f(p: (a: Real, b: Real)) = p", "component"),
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

fn key_text<'a>(source: &'a str, key: &ComponentKey) -> &'a str {
    let span = match key {
        ComponentKey::Name(name) => name.span,
        ComponentKey::Position(number) => number.span,
        ComponentKey::Pair { span, .. } => *span,
    };
    &source[span.start..span.end]
}

#[test]
fn component_reads_take_name_position_and_pair_keys_at_their_precedence() {
    for (source, key) in [
        ("rho of u", "rho"),
        ("0 of v", "0"),
        ("(0, 1) of m", "(0, 1)"),
    ] {
        let ExprKind::Projection { key: found, value } = expression(source).kind else {
            panic!("{source}");
        };
        assert_eq!(key_text(source, &found), key);
        assert!(matches!(value.kind, ExprKind::Name(_)), "{source}");
    }
    // `of` binds tighter than arithmetic and unary minus, looser than postfix.
    let ExprKind::Binary {
        operator: BinaryOperator::Add,
        left,
        ..
    } = expression("0 of v + 1").kind
    else {
        panic!();
    };
    assert!(matches!(left.kind, ExprKind::Projection { .. }));
    let ExprKind::Unary {
        operator: UnaryOperator::Negate,
        operand,
    } = expression("-0 of v").kind
    else {
        panic!();
    };
    assert!(matches!(operand.kind, ExprKind::Projection { .. }));
    for source in ["0 of v[s]", "0 of f(x)"] {
        let ExprKind::Projection { value, .. } = expression(source).kind else {
            panic!("{source}");
        };
        assert!(
            matches!(value.kind, ExprKind::Index { .. } | ExprKind::Call { .. }),
            "{source}"
        );
    }
    // Reads nest from the right.
    let ExprKind::Projection { key, value } = expression("a of b of c").kind else {
        panic!();
    };
    assert!(matches!(key, ComponentKey::Name(name) if name.text == "a"));
    assert!(matches!(value.kind, ExprKind::Projection { .. }));
    // A real spelling is kept for analysis to refuse; a pair needs two positions.
    let ExprKind::Projection {
        key: ComponentKey::Position(number),
        ..
    } = expression("1.5 of v").kind
    else {
        panic!();
    };
    assert_eq!(number.text, "1.5");
    let errors = ast::parse_expression("(0, 1, 2) of m").unwrap_err();
    assert!(matches!(
        errors.as_slice(),
        [Error::Form(FormDiagnostic { kind: FormDiagnosticKind::ComponentKeyPositions { found: 3 }, span })]
            if span.start == 0 && span.end == 9
    ));
}

#[test]
fn updates_take_partial_products_chain_left_and_bind_looser_than_reads() {
    let source = "u but (D: 9.0, 0: x, (0, 1): 1.0, D: 2.0)";
    let ExprKind::Update {
        base,
        overrides,
        operator_span,
    } = expression(source).kind
    else {
        panic!();
    };
    assert_eq!(&source[operator_span.start..operator_span.end], "but");
    assert!(matches!(base.kind, ExprKind::Name(name) if name.text == "u"));
    let ExprKind::Product(entries) = &overrides.kind else {
        panic!();
    };
    let keys: Vec<_> = entries
        .iter()
        .map(|entry| key_text(source, &entry.key))
        .collect();
    assert_eq!(keys, ["D", "0", "(0, 1)", "D"]);
    assert_eq!(
        &source[entries[2].span.start..entries[2].span.end],
        "(0, 1): 1.0"
    );
    // Updates group left, continue across a line, and bind looser than `of`.
    let ExprKind::Update { base, .. } = expression("u but (D: 9.0) but (S: 7.0)").kind else {
        panic!();
    };
    assert!(matches!(base.kind, ExprKind::Update { .. }));
    assert!(matches!(
        expression("u\n but (D: 1.0)").kind,
        ExprKind::Update { .. }
    ));
    let ExprKind::Update { base, .. } = expression("0 of v but (D: 1.0)").kind else {
        panic!();
    };
    assert!(matches!(base.kind, ExprKind::Projection { .. }));
    // Overrides are ordinary values, and nested partial products stay nested.
    for source in ["u but o", "u but f(x)", "u but (D: (x: 1.0))"] {
        assert!(
            matches!(expression(source).kind, ExprKind::Update { .. }),
            "{source}"
        );
    }
}

#[test]
fn the_removed_spatial_update_is_a_typed_error_at_its_colon() {
    for (source, key) in [
        ("u but 0..4: 1.0", "0..4"),
        ("u but (n - 1): 0.0", "(n - 1)"),
        ("u but [0..1, 1..2]: v", "[0..1, 1..2]"),
        ("u but mask: v", "mask"),
    ] {
        let errors = ast::parse_expression(source).unwrap_err();
        let [Error::Form(FormDiagnostic {
            kind: FormDiagnosticKind::RemovedSpatialUpdate { key_span },
            span,
        })] = errors.as_slice()
        else {
            panic!("{source}: {errors:?}");
        };
        assert_eq!(&source[span.start..span.end], ":", "{source}");
        assert_eq!(&source[key_span.start..key_span.end], key, "{source}");
    }
}

fn local_pattern(source: &str) -> Pattern {
    let ExprKind::Body(body) = expression(source).kind else {
        panic!("{source}");
    };
    let BodyItem::Local(local) = &body.items[0] else {
        panic!("{source}");
    };
    assert_pattern_spans(source, &local.pattern);
    local.pattern.clone()
}

#[test]
fn constructor_component_and_brace_patterns_nest_with_spans() {
    let source = "{ P(_, (k: a, 0: [b, c]), { rho, vel: (x: v) }) := m\n a }";
    let pattern = local_pattern(source);
    let text = |span: Span| &source[span.start..span.end];
    let PatternKind::Constructor { head, items } = &pattern.kind else {
        panic!("{pattern:?}");
    };
    assert_eq!(head.text, "P");
    assert_eq!(items.len(), 3);
    assert!(matches!(items[0].kind, PatternKind::Wildcard));
    let PatternKind::Components(components) = &items[1].kind else {
        panic!();
    };
    assert_eq!(text(items[1].span), "(k: a, 0: [b, c])");
    assert!(matches!(&components[0].key, ComponentKey::Name(name) if name.text == "k"));
    assert!(matches!(&components[1].key, ComponentKey::Position(number) if number.text == "0"));
    assert!(matches!(components[1].pattern.kind, PatternKind::List(_)));
    assert_eq!(text(components[1].span), "0: [b, c]");
    let PatternKind::Entries(entries) = &items[2].kind else {
        panic!();
    };
    assert_eq!(text(items[2].span), "{ rho, vel: (x: v) }");
    assert!(entries[0].pattern.is_none());
    assert_eq!(entries[0].key.text, "rho");
    assert!(matches!(
        entries[1].pattern.as_ref().map(|pattern| &pattern.kind),
        Some(PatternKind::Components(_))
    ));
    // Every admitted position takes the new forms.
    for source in [
        "{ rho } = m",
        "(k: a) = m",
        "f(P(a, b), { rho }, (k: c)) = a",
        "x = for { rho } in d { rho }",
        "x = for P(a, b) in d { a }",
        "x = reduce s := 0 for (k: a) in d { s + a }",
        "f((a, P(b, c)): F) = a",
    ] {
        assert!(ast::parse_document(source).is_ok(), "{source}");
    }
    // An inline description head is outside this subset.
    let errors = ast::parse_document("x = { (D: Real, E: Real)(d, e) := m\n d }").unwrap_err();
    assert!(matches!(
        errors.as_slice(),
        [Error::Form(FormDiagnostic { kind: FormDiagnosticKind::UnsupportedConstruct { kind }, .. })]
            if kind == "described_head"
    ));
}

#[test]
fn only_names_discards_and_positional_tuples_take_a_stated_type() {
    for (source, pattern, written) in [
        ("x = { [a, b]: T := m\n a }", "[a, b]", "T"),
        ("x = { { rho }: S := m\n rho }", "{ rho }", "S"),
        ("x = { P(a): Q := m\n a }", "P(a)", "Q"),
        ("f([a, b]: T) = a", "[a, b]", "T"),
    ] {
        let errors = ast::parse_document(source).unwrap_err();
        let [Error::Form(FormDiagnostic {
            kind: FormDiagnosticKind::PatternTypeAnnotation { pattern_span },
            span,
        })] = errors.as_slice()
        else {
            panic!("{source}: {errors:?}");
        };
        assert_eq!(
            &source[pattern_span.start..pattern_span.end],
            pattern,
            "{source}"
        );
        assert_eq!(&source[span.start..span.end], written, "{source}");
    }
    // A typed discard takes part in overload selection.
    for source in [
        "x = { a: Real := m\n a }",
        "f((a, b): F) = a",
        "x: Real = 1",
        "f(g: (Real) -> Real) = 1",
        "f(_: Real) = 1",
        "x = { _: Real := m\n 1 }",
    ] {
        assert!(ast::parse_document(source).is_ok(), "{source}");
    }
}

fn default_parts(expr: Expr) -> (Expr, Expr) {
    match expr.kind {
        ExprKind::Default {
            value, fallback, ..
        } => (*value, *fallback),
        kind => panic!("expected a default, found {kind:?}"),
    }
}

#[test]
fn presence_defaults_group_right_between_arithmetic_and_comparisons() {
    // Right-associative: `a ?? b ?? c` is `a ?? (b ?? c)`.
    let (value, fallback) = default_parts(expression("a ?? b ?? c"));
    assert!(matches!(value.kind, ExprKind::Name(_)));
    assert!(matches!(fallback.kind, ExprKind::Default { .. }));
    // Looser than arithmetic, tighter than comparisons and Boolean words.
    let (_, fallback) = default_parts(expression("m ?? n + 1"));
    assert!(matches!(
        fallback.kind,
        ExprKind::Binary {
            operator: BinaryOperator::Add,
            ..
        }
    ));
    for (source, operator) in [
        ("m ?? 0 > 3", BinaryOperator::Greater),
        ("a ?? b and c", BinaryOperator::And),
    ] {
        let ExprKind::Binary {
            operator: found,
            left,
            ..
        } = expression(source).kind
        else {
            panic!("{source}");
        };
        assert_eq!(found, operator, "{source}");
        assert!(matches!(left.kind, ExprKind::Default { .. }), "{source}");
    }
    let ExprKind::Binary { right, .. } = expression("a == b ?? c").kind else {
        panic!();
    };
    assert!(matches!(right.kind, ExprKind::Default { .. }));
    let ExprKind::Unary {
        operator: UnaryOperator::Not,
        operand,
    } = expression("not m ?? false").kind
    else {
        panic!();
    };
    assert!(matches!(operand.kind, ExprKind::Default { .. }));
    // A line beginning with `??` continues the expression; spans are exact.
    let source = "xs[i]\n  ?? 0.0";
    let expr = expression(source);
    let ExprKind::Default { operator_span, .. } = &expr.kind else {
        panic!();
    };
    assert_eq!(&source[operator_span.start..operator_span.end], "??");
    let (value, _) = default_parts(expression("(u ?? w)[i] ?? 0.0"));
    assert!(matches!(value.kind, ExprKind::Index { .. }));
}

#[test]
fn absence_comes_from_none_and_else_less_conditionals() {
    assert_eq!(expression("none").kind, ExprKind::None);
    let ExprKind::If { alternative, .. } = expression("if c then a").kind else {
        panic!();
    };
    assert!(alternative.is_none());
    // `else` binds to the nearest `if`.
    let ExprKind::If {
        consequence,
        alternative,
        ..
    } = expression("if a then if b then x else y").kind
    else {
        panic!();
    };
    assert!(alternative.is_none());
    assert!(matches!(
        &consequence.kind,
        ExprKind::If {
            alternative: Some(_),
            ..
        }
    ));
}

#[test]
fn absence_marks_stand_at_each_lifting_position() {
    let text = |source: &str, expr: &Expr| source[expr.span.start..expr.span.end].to_owned();
    let lifted = |expr: &Expr| matches!(expr.kind, ExprKind::Lift { .. });
    let ExprKind::Call { arguments, .. } = expression("f(x?, y)").kind else {
        panic!();
    };
    assert!(lifted(&arguments[0]) && !lifted(&arguments[1]));
    let ExprKind::Index { target, .. } = expression("m?[0]").kind else {
        panic!();
    };
    assert!(lifted(&target));
    let ExprKind::Projection { value, .. } = expression("k of a?").kind else {
        panic!();
    };
    assert!(lifted(&value));
    let ExprKind::For(comprehension) = expression("for u in m? { u }").kind else {
        panic!();
    };
    assert!(lifted(&comprehension.collection));
    let ExprKind::For(comprehension) = expression("for [a, b] in [xs?, ys] { a }").kind else {
        panic!();
    };
    let ExprKind::List(collections) = &comprehension.collection.kind else {
        panic!();
    };
    assert!(lifted(&collections[0]) && !lifted(&collections[1]));
    let source = "{ p := g?\n p }";
    let ExprKind::Body(body) = expression(source).kind else {
        panic!();
    };
    let BodyItem::Local(local) = &body.items[0] else {
        panic!();
    };
    assert!(lifted(&local.value));
    assert_eq!(text(source, &local.value), "g?");
    // Whether a mark has an operation to lift through is analysis's check.
    assert!(matches!(expression("x? + 1").kind, ExprKind::Binary { .. }));
}

#[test]
fn optional_types_take_only_a_none_alternative() {
    for source in [
        "x: Real|None = 1",
        "x: List<Real|None> = 1",
        "state m: Real|None = none",
        "f(x: Real|None) = x",
        "f(g: Real|None -> Real) = 1",
        "f(g: (Real|None) -> Real|None) = 1",
    ] {
        let document =
            ast::parse_document(source).unwrap_or_else(|errors| panic!("{source}: {errors:?}"));
        let debug = format!("{document:?}");
        assert!(debug.contains("Optional"), "{source}");
    }
    let source = "x: Array<Real, 1>|None = 1";
    let document = ast::parse_document(source).unwrap();
    let ItemKind::Binding(binding) = &document.items[0].kind else {
        panic!();
    };
    let annotation = binding.annotation.as_ref().unwrap();
    assert_eq!(
        &source[annotation.span.start..annotation.span.end],
        "Array<Real, 1>|None"
    );
    let AnnotationKind::Optional { value, none_span } = &annotation.kind else {
        panic!();
    };
    assert_eq!(&source[none_span.start..none_span.end], "None");
    assert!(matches!(value.kind, AnnotationKind::Apply { .. }));
    // `T|None` is the only union.
    let source = "x: Real|Int = 1";
    let errors = ast::parse_document(source).unwrap_err();
    let [Error::Form(FormDiagnostic {
        kind: FormDiagnosticKind::UnionAlternative,
        span,
    })] = errors.as_slice()
    else {
        panic!("{errors:?}");
    };
    assert_eq!(&source[span.start..span.end], "Int");
}

fn roster(variants: &[Variant]) -> Vec<(&str, Option<&str>)> {
    variants
        .iter()
        .map(|variant| {
            let payload = variant.payload.as_ref().map(|payload| match &payload.kind {
                AnnotationKind::Named(name) | AnnotationKind::Apply { name, .. } => {
                    name.text.as_str()
                }
                AnnotationKind::Struct(_) => "{}",
                AnnotationKind::Sum(_) => "|",
                AnnotationKind::Optional { .. } => "?",
            });
            (variant.name.text.as_str(), payload)
        })
        .collect()
}

#[test]
fn sum_rosters_keep_declared_order_unit_variants_duplicates_and_nesting() {
    let source = "fiber Shape = (A: Real | B: Vector<Real, 3>)\n\
                  fiber Mode = (Binary\n  | Single)\n\
                  fiber Mixed = (Point | Segment: Real | Point | Inner: (C | D: Int))\n";
    let document = ast::parse_document(source).unwrap_or_else(|errors| panic!("{errors:?}"));
    let sums: Vec<_> = document
        .items
        .iter()
        .map(|item| match &item.kind {
            ItemKind::Sum(sum) => {
                assert_name_span(source, &sum.name);
                assert_variant_spans(source, &sum.variants);
                sum
            }
            other => panic!("{other:?}"),
        })
        .collect();
    assert_eq!(
        roster(&sums[0].variants),
        [("A", Some("Real")), ("B", Some("Vector"))]
    );
    assert_eq!(
        roster(&sums[1].variants),
        [("Binary", None), ("Single", None)]
    );
    assert_eq!(
        roster(&sums[2].variants),
        [
            ("Point", None),
            ("Segment", Some("Real")),
            ("Point", None),
            ("Inner", Some("|")),
        ]
    );
    let inner = sums[2].variants[3].payload.as_ref().unwrap();
    assert_eq!(&source[inner.span.start..inner.span.end], "(C | D: Int)");
    let AnnotationKind::Sum(inner) = &inner.kind else {
        panic!();
    };
    assert_eq!(roster(inner), [("C", None), ("D", Some("Int"))]);
    // A product's components still normalize as a product fiber.
    let document = ast::parse_document("fiber P: Module = (x: Real, y: Real)").unwrap();
    assert!(
        matches!(&document.items[0].kind, ItemKind::Fiber(fiber) if fiber.components.len() == 2)
    );
}

#[test]
fn inline_sums_stand_where_a_fiber_type_does() {
    for (source, written) in [
        ("a: (Point | Segment: Real) = x", "(Point | Segment: Real)"),
        ("f(s: (A | B)) = s", "(A | B)"),
        ("x: List<(A | B: Int)> = l", "(A | B: Int)"),
        ("x: (A | B)|None = none", "(A | B)"),
        ("struct S = {mode: (On | Off)}", "(On | Off)"),
        ("fiber P = (m: (On | Off), v: Real)", "(On | Off)"),
    ] {
        let document =
            ast::parse_document(source).unwrap_or_else(|errors| panic!("{source}: {errors:?}"));
        let debug = format!("{document:?}");
        assert!(debug.contains("Sum(["), "{source}: {debug}");
        let start = source.find(written).unwrap();
        assert!(
            debug.contains(&format!(
                "span: Span {{ start: {start}, end: {} }}",
                start + written.len()
            )),
            "{source}: {debug}"
        );
    }
}

#[test]
fn constructors_name_one_variant_under_a_named_or_inline_head() {
    let source = "Shape(A: 1.0)";
    let expr = expression(source);
    assert_expr_spans(source, &expr);
    let ExprKind::Construct {
        head: ConstructorHead::Named(NameRef::Local(head)),
        variant,
        payload: Some(payload),
    } = &expr.kind
    else {
        panic!("{expr:?}");
    };
    assert_eq!((head.text.as_str(), variant.text.as_str()), ("Shape", "A"));
    assert!(matches!(&payload.kind, ExprKind::Number(value) if value == "1.0"));

    let source = "orbit::Shape(B: (X: 1, Y: v + 1))";
    let expr = expression(source);
    assert_expr_spans(source, &expr);
    let ExprKind::Construct {
        head: ConstructorHead::Named(NameRef::Qualified(head)),
        payload: Some(payload),
        ..
    } = &expr.kind
    else {
        panic!("{expr:?}");
    };
    assert_eq!(head.namespace.text, "orbit");
    assert!(matches!(&payload.kind, ExprKind::Product(entries) if entries.len() == 2));

    // An inline roster heads a payload or a unit constructor.
    for (source, variant, has_payload) in [
        ("(A: Real | B)(A: 1.0)", "A", true),
        ("(A: Real | B)(B)", "B", false),
    ] {
        let expr = expression(source);
        assert_expr_spans(source, &expr);
        let ExprKind::Construct {
            head: ConstructorHead::Inline { variants, span },
            variant: written,
            payload,
        } = &expr.kind
        else {
            panic!("{expr:?}");
        };
        assert_eq!(&source[span.start..span.end], "(A: Real | B)");
        assert_eq!(roster(variants), [("A", Some("Real")), ("B", None)]);
        assert_eq!(written.text, variant);
        assert_eq!(payload.is_some(), has_payload);
    }

    // Only name resolution can tell these from calls, products and names.
    assert!(matches!(
        expression("Mode(Binary)").kind,
        ExprKind::Call { .. }
    ));
    assert!(matches!(expression("(B: 1.0)").kind, ExprKind::Product(_)));
    assert!(matches!(expression("Binary").kind, ExprKind::Name(_)));

    // A constructor over an array of payloads is still one constructor.
    let expr = expression("Shape(A: xs)");
    assert!(
        matches!(&expr.kind, ExprKind::Construct { payload: Some(payload), .. } if matches!(payload.kind, ExprKind::Name(_)))
    );
}

#[test]
fn sum_form_errors_are_typed_at_their_spans() {
    for (source, written, expected) in [
        (
            "fiber M: Module = (A | B)",
            "Module",
            FormDiagnosticKind::SumAlgebra,
        ),
        (
            "x = Shape(A: 1, B: 2)",
            "(A: 1, B: 2)",
            FormDiagnosticKind::ConstructorVariantCount { found: 2 },
        ),
        (
            "x = (A: Real | B)(1.0)",
            "(1.0)",
            FormDiagnosticKind::ConstructorWithoutVariant,
        ),
        (
            "x = (A: Real | B)(B, A)",
            "(B, A)",
            FormDiagnosticKind::ConstructorWithoutVariant,
        ),
        (
            "x = (A: Real | B)(_)",
            "(_)",
            FormDiagnosticKind::ConstructorWithoutVariant,
        ),
    ] {
        assert!(
            !cartan_syntax::parse_document(source).has_errors(),
            "{source}"
        );
        let errors = ast::parse_document(source).unwrap_err();
        let [Error::Form(FormDiagnostic { kind, span })] = errors.as_slice() else {
            panic!("{source}: {errors:?}");
        };
        assert_eq!(kind, &expected, "{source}");
        assert_eq!(&source[span.start..span.end], written, "{source}");
    }
    // The grammar refuses one-variant rosters and mixed separators.
    // So are arm guards, literal and alternative arm patterns, and several
    // scrutinees.
    for source in [
        "fiber M = (A | B, C: Real)",
        "fiber M = (A: Real, B | C)",
        "x = match s { A(x) if c then 1 }",
        "x = match s { 1 then 2 }",
        "x = match s { A | B then 1 }",
        "x = match a, b { A then 1 }",
    ] {
        assert!(
            cartan_syntax::parse_document(source).has_errors(),
            "{source}"
        );
    }
}

fn match_of(source: &str) -> Match {
    let expr = expression(source);
    assert_expr_spans(source, &expr);
    let ExprKind::Match(written) = expr.kind else {
        panic!("{expr:?}");
    };
    *written
}

/// A payload pattern's shape without spans.
fn payload_shape(pattern: &Pattern) -> String {
    match &pattern.kind {
        PatternKind::Name(name) => name.text.clone(),
        PatternKind::Wildcard => "_".into(),
        PatternKind::Components(components) => {
            let items: Vec<_> = components
                .iter()
                .map(|component| {
                    let ComponentKey::Name(key) = &component.key else {
                        panic!();
                    };
                    format!("{}: {}", key.text, payload_shape(&component.pattern))
                })
                .collect();
            format!("({})", items.join(", "))
        }
        other => format!("{other:?}"),
    }
}

#[test]
fn match_arms_keep_order_payload_patterns_and_a_final_else() {
    let source = "match shape(i) { A(x) then x, B(X: x, Y: y) then x + y, \
                  C((X: x)) then x, D(_) then 0, Unit then 1, A(z) then z, else 2 }";
    let written = match_of(source);
    assert!(matches!(written.scrutinee.kind, ExprKind::Call { .. }));
    let arms: Vec<_> = written
        .arms
        .iter()
        .map(|arm| {
            (
                arm.variant.text.as_str(),
                arm.payload.as_ref().map(payload_shape),
            )
        })
        .collect();
    assert_eq!(
        arms,
        [
            ("A", Some("x".into())),
            ("B", Some("(X: x, Y: y)".into())),
            ("C", Some("(X: x)".into())),
            ("D", Some("_".into())),
            ("Unit", None),
            // A repeated arm is kept for analysis to refuse.
            ("A", Some("z".into())),
        ]
    );
    // The shorthand's payload spans its components; the explicit form its parens.
    let shorthand = written.arms[1].payload.as_ref().unwrap().span;
    assert_eq!(&source[shorthand.start..shorthand.end], "X: x, Y: y");
    let explicit = written.arms[2].payload.as_ref().unwrap().span;
    assert_eq!(&source[explicit.start..explicit.end], "(X: x)");
    let otherwise = written.otherwise.unwrap();
    assert_eq!(&source[otherwise.span.start..otherwise.span.end], "else 2");

    // Newlines separate arms as commas do, and arms nest.
    let source = "match s {\n  A(x) then match x {\n    P then 1\n    Q then 2\n  }\n  B then 3\n\n  else 4\n}";
    let written = match_of(source);
    assert_eq!(written.arms.len(), 2);
    assert!(
        matches!(&written.arms[0].body.kind, ExprKind::Match(inner) if inner.arms.len() == 2 && inner.otherwise.is_none())
    );
    assert!(written.otherwise.is_some());

    // A conditional arm body: same-line `else`, or a comma before the `else` arm.
    let written = match_of("match s { A(x) then if x > 0 then 1 else 2, else 0 }");
    assert!(matches!(
        &written.arms[0].body.kind,
        ExprKind::If {
            alternative: Some(_),
            ..
        }
    ));
    assert!(written.otherwise.is_some());
    let written = match_of("match s {\n  A(x) then if x > 0 then 1,\n  else 0\n}");
    assert!(matches!(
        &written.arms[0].body.kind,
        ExprKind::If {
            alternative: None,
            ..
        }
    ));
    assert!(written.otherwise.is_some());

    // Coverage is analysis: an empty or else-only match normalizes.
    assert!(match_of("match s { }").arms.is_empty());
    let written = match_of("match s { else 1 }");
    assert!(written.arms.is_empty() && written.otherwise.is_some());
}

#[test]
fn else_arms_stand_once_and_last() {
    for (source, written, first) in [
        ("x = match s { else 1, else 2 }", "else 2", "else 1"),
        (
            "x = match s { A then 1, else 2, B then 3 }",
            "B then 3",
            "else 2",
        ),
    ] {
        assert!(
            !cartan_syntax::parse_document(source).has_errors(),
            "{source}"
        );
        let errors = ast::parse_document(source).unwrap_err();
        let [Error::Form(FormDiagnostic { kind, span })] = errors.as_slice() else {
            panic!("{source}: {errors:?}");
        };
        let earlier = match kind {
            FormDiagnosticKind::DuplicateElseArm { first_span } => {
                assert!(written.starts_with("else"));
                first_span
            }
            FormDiagnosticKind::ArmAfterElse { else_span } => else_span,
            other => panic!("{source}: {other:?}"),
        };
        assert_eq!(&source[span.start..span.end], written, "{source}");
        assert_eq!(&source[earlier.start..earlier.end], first, "{source}");
    }
}

#[test]
fn a_line_leading_else_after_an_unbracketed_arm_conditional_is_refused() {
    for (source, conditional) in [
        (
            "x = match s {\n  A(x) then if x > 0 then 1\n  else 0\n}",
            "if x > 0 then 1\n  else 0",
        ),
        // Wherever the conditional stands in the body, outside brackets.
        (
            "x = match s {\n  A(x) then 1 + if x > 0 then 1\n  else 0\n}",
            "if x > 0 then 1\n  else 0",
        ),
        // The inner conditional takes the `else`.
        (
            "x = match s {\n  A(x) then if a then if b then 1\n  else 2\n}",
            "if b then 1\n  else 2",
        ),
        // The `else` arm's body is an arm body too.
        (
            "x = match s {\n  A then 1\n  else if c then 2\n  else 3\n}",
            "if c then 2\n  else 3",
        ),
        // A nested match's own arm is checked against its own list.
        (
            "x = match s {\n  A then match t {\n    P then if c then 1\n    else 2\n  }\n}",
            "if c then 1\n    else 2",
        ),
    ] {
        assert!(
            !cartan_syntax::parse_document(source).has_errors(),
            "{source}"
        );
        let errors = ast::parse_document(source).unwrap_err();
        let [Error::Form(FormDiagnostic {
            kind:
                FormDiagnosticKind::LineLeadingArmElse {
                    else_span,
                    conditional_span,
                },
            span,
        })] = errors.as_slice()
        else {
            panic!("{source}: {errors:?}");
        };
        assert_eq!(span, else_span);
        assert_eq!(&source[span.start..span.end], "else", "{source}");
        assert!(source[..span.start].ends_with(' '), "{source}");
        assert_eq!(
            &source[conditional_span.start..conditional_span.end],
            conditional,
            "{source}"
        );
    }
}

#[test]
fn arm_conditionals_take_a_same_line_bracketed_or_comma_ended_else() {
    let alternative = |arm: &MatchArm| match &arm.body.kind {
        ExprKind::If { alternative, .. } => alternative.is_some(),
        ExprKind::Body(body) => matches!(
            body.items.as_slice(),
            [BodyItem::Result(Expr {
                kind: ExprKind::If {
                    alternative: Some(_),
                    ..
                },
                ..
            })]
        ),
        other => panic!("{other:?}"),
    };
    for (source, takes_else) in [
        // `else` on the consequence's line.
        (
            "match s {\n  A(x) then if x > 0 then 1 else 2\n  else 0\n}",
            true,
        ),
        // A bracketed conditional keeps the ordinary continuation rule.
        (
            "match s {\n  A(x) then (if x > 0 then 1\n  else 2)\n  else 0\n}",
            true,
        ),
        (
            "match s {\n  A(x) then { if x > 0 then 1\n  else 2 }\n  else 0\n}",
            true,
        ),
        // A comma ends the arm before the `else` arm.
        (
            "match s {\n  A(x) then if x > 0 then 1,\n  else 0\n}",
            false,
        ),
    ] {
        let written = match_of(source);
        assert_eq!(alternative(&written.arms[0]), takes_else, "{source}");
        assert!(written.otherwise.is_some(), "{source}");
    }
    // Brackets inside the body enclose the conditional too.
    let written = match_of("match s {\n  A(x) then f(if x > 0 then 1\n  else 2)\n}");
    assert!(written.otherwise.is_none());

    // Outside `match` a line-leading `else` still continues the nearest `if`.
    let source = "x = if a then if b then 1\n  else 2\ny = { if c then 3\n  else 4 }";
    let document = ast::parse_document(source).unwrap_or_else(|errors| panic!("{errors:?}"));
    let ItemKind::Binding(binding) = &document.items[0].kind else {
        panic!();
    };
    let ExprKind::If {
        consequence,
        alternative: None,
        ..
    } = &binding.value.kind
    else {
        panic!("{:?}", binding.value);
    };
    assert!(matches!(
        consequence.kind,
        ExprKind::If {
            alternative: Some(_),
            ..
        }
    ));
    assert_eq!(document.items.len(), 2);
}

#[test]
fn struct_entries_take_an_optional_type() {
    for (source, written) in [
        ("struct P = {name: Real|None, n: Int}", "Real|None"),
        ("x: {a: Real|None} = {a: none}", "Real|None"),
        (
            "f(p: {name: Vector<Real, 2>|None}) = 1.0",
            "Vector<Real, 2>|None",
        ),
        ("struct S = {inner: {a: Int|None}}", "Int|None"),
        ("struct S = {mode: (On | Off)|None}", "(On | Off)|None"),
    ] {
        let document =
            ast::parse_document(source).unwrap_or_else(|errors| panic!("{source}: {errors:?}"));
        let mut optionals = Vec::new();
        fn collect(entries: &[StructEntry], into: &mut Vec<(Span, Span)>) {
            for entry in entries {
                match &entry.annotation.kind {
                    AnnotationKind::Optional { none_span, .. } => {
                        into.push((entry.annotation.span, *none_span))
                    }
                    AnnotationKind::Struct(inner) => collect(inner, into),
                    _ => {}
                }
            }
        }
        match &document.items[0].kind {
            ItemKind::Struct(declaration) => collect(&declaration.entries, &mut optionals),
            ItemKind::Binding(Binding {
                annotation:
                    Some(Annotation {
                        kind: AnnotationKind::Struct(entries),
                        ..
                    }),
                ..
            }) => collect(entries, &mut optionals),
            ItemKind::Function(function) => {
                let Some(ParameterAnnotation::Value(Annotation {
                    kind: AnnotationKind::Struct(entries),
                    ..
                })) = &function.parameters[0].annotation
                else {
                    panic!("{source}");
                };
                collect(entries, &mut optionals);
            }
            other => panic!("{source}: {other:?}"),
        }
        let [(whole, none)] = optionals.as_slice() else {
            panic!("{source}: {optionals:?}");
        };
        assert_eq!(&source[whole.start..whole.end], written, "{source}");
        assert_eq!(&source[none.start..none.end], "None", "{source}");
    }
    // `T|None` is still the only union, and one alternative is all it takes.
    let source = "struct P = {a: Real|Int}";
    let errors = ast::parse_document(source).unwrap_err();
    let [Error::Form(FormDiagnostic {
        kind: FormDiagnosticKind::UnionAlternative,
        span,
    })] = errors.as_slice()
    else {
        panic!("{errors:?}");
    };
    assert_eq!(&source[span.start..span.end], "Int");
    assert!(cartan_syntax::parse_document("struct P = {a: Real|None|None}").has_errors());
    // Fiber components still take a bare fiber type.
    assert!(cartan_syntax::parse_document("fiber P = (a: Real|None, b: Real)").has_errors());
}

#[test]
fn unary_plus_reads_as_unary_minus_does() {
    fn unary(expr: &Expr) -> (UnaryOperator, &Expr) {
        let ExprKind::Unary { operator, operand } = &expr.kind else {
            panic!("{expr:?}");
        };
        (*operator, operand)
    }
    for (plus, minus) in [
        ("+0.5", "-0.5"),
        ("+x ^ 2", "-x ^ 2"),
        ("+x of v", "-x of v"),
    ] {
        let (plus, minus) = (expression(plus), expression(minus));
        let (plus_operator, plus_operand) = unary(&plus);
        let (minus_operator, minus_operand) = unary(&minus);
        assert_eq!(
            (plus_operator, minus_operator),
            (UnaryOperator::Plus, UnaryOperator::Negate)
        );
        assert_eq!(plus_operand, minus_operand);
    }
    let source = "[1, -0.5, +0.5]";
    let ExprKind::List(items) = expression(source).kind else {
        panic!();
    };
    assert_eq!(unary(&items[2]).0, UnaryOperator::Plus);
    assert_eq!(&source[items[2].span.start..items[2].span.end], "+0.5");
    // Binary forms are unchanged, and a unary operand follows a binary operator.
    assert!(matches!(
        expression("1 +0.5").kind,
        ExprKind::Binary {
            operator: BinaryOperator::Add,
            ..
        }
    ));
    let ExprKind::Binary { right, .. } = expression("1 + +0.5").kind else {
        panic!();
    };
    assert_eq!(unary(&right).0, UnaryOperator::Plus);
    // A line opening with `+` continues the line above; one opening with `-` does not.
    let ExprKind::List(items) = expression("[1\n +2]").kind else {
        panic!();
    };
    assert!(matches!(
        items.as_slice(),
        [Expr {
            kind: ExprKind::Binary {
                operator: BinaryOperator::Add,
                ..
            },
            ..
        }]
    ));
    let ExprKind::List(items) = expression("[1\n -2]").kind else {
        panic!();
    };
    assert_eq!(items.len(), 2);
}
