use cartan_syntax::literal::{
    parse_assignment, parse_literal, AssignmentError, Literal, LiteralError,
};

#[test]
fn literals_parse_as_source_spells_them() {
    for (text, expected) in [
        ("64", Literal::Int(64)),
        ("-2147483648", Literal::Int(i32::MIN)),
        ("+7", Literal::Int(7)),
        ("1_000", Literal::Int(1000)),
        ("1e-4", Literal::Real(1e-4)),
        ("-2.5", Literal::Real(-2.5)),
        ("1.0", Literal::Real(1.0)),
        ("true", Literal::Bool(true)),
        ("none", Literal::None),
        ("\"run \\\"1\\\"\"", Literal::String("run \"1\"".into())),
    ] {
        assert_eq!(parse_literal(text), Ok(expected), "{text:?}");
    }
    assert!(matches!(
        parse_literal("2147483648"),
        Err(LiteralError::IntRange { .. })
    ));
    assert!(matches!(
        parse_literal("1e40"),
        Err(LiteralError::RealRange { .. })
    ));
    for text in ["n * 2", "f\"x\"", "-n", "[1]"] {
        assert!(
            matches!(parse_literal(text), Err(LiteralError::NotLiteral { .. })),
            "{text:?}"
        );
    }
    let Err(LiteralError::Syntax(errors)) = parse_literal("1 +") else {
        panic!()
    };
    assert!(matches!(&errors[0], cartan_syntax::ast::Error::Grammar(d) if d.incomplete));
}

#[test]
fn assignments_name_a_param_and_index_the_whole_text() {
    assert_eq!(
        parse_assignment("dt=1e-4"),
        Ok(cartan_syntax::literal::Assignment {
            name: "dt".into(),
            value: Literal::Real(1e-4),
        })
    );
    assert_eq!(
        parse_assignment("label=\"a=b\"").unwrap().value,
        Literal::String("a=b".into())
    );
    assert!(matches!(
        parse_assignment("nx"),
        Err(AssignmentError::MissingEquals { .. })
    ));
    for name in ["", "if", "2x"] {
        let text = format!("{name}=1");
        assert!(
            matches!(parse_assignment(&text), Err(AssignmentError::InvalidName { name: n }) if n == name),
            "{text:?}"
        );
    }
    let Err(AssignmentError::Value {
        error: LiteralError::RealRange { span, .. },
        ..
    }) = parse_assignment("dt=1e99")
    else {
        panic!()
    };
    assert_eq!((span.start, span.end), (3, 7));
}
