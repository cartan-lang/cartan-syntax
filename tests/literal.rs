use cartan_syntax::literal::{
    number, parse_assignment, parse_literal, signed_number, AssignmentError, Literal, LiteralError,
    NumberError, Real, RealWidth,
};

const F32: RealWidth = RealWidth::F32;
const F64: RealWidth = RealWidth::F64;

fn real32(value: f32) -> Literal {
    Literal::Real(Real::F32(value))
}

#[test]
fn literals_parse_as_source_spells_them() {
    for (text, expected) in [
        ("64", Literal::Int(64)),
        ("-2147483648", Literal::Int(i32::MIN)),
        ("+7", Literal::Int(7)),
        ("1_000", Literal::Int(1000)),
        ("1e-4", real32(1e-4)),
        ("-2.5", real32(-2.5)),
        ("1.0", real32(1.0)),
        ("true", Literal::Bool(true)),
        ("none", Literal::None),
        ("\"run \\\"1\\\"\"", Literal::String("run \"1\"".into())),
    ] {
        assert_eq!(parse_literal(text, F32), Ok(expected), "{text:?}");
    }
    assert!(matches!(
        parse_literal("2147483648", F32),
        Err(LiteralError::IntRange { .. })
    ));
    assert!(matches!(
        parse_literal("1e40", F32),
        Err(LiteralError::RealRange { .. })
    ));
    for text in ["n * 2", "f\"x\"", "-n", "[1]"] {
        assert!(
            matches!(
                parse_literal(text, F32),
                Err(LiteralError::NotLiteral { .. })
            ),
            "{text:?}"
        );
    }
    let Err(LiteralError::Syntax(errors)) = parse_literal("1 +", F32) else {
        panic!()
    };
    assert!(matches!(&errors[0], cartan_syntax::ast::Error::Grammar(d) if d.incomplete));
}

#[test]
fn assignments_name_a_param_and_index_the_whole_text() {
    assert_eq!(
        parse_assignment("dt=1e-4", F32),
        Ok(cartan_syntax::literal::Assignment {
            name: "dt".into(),
            value: real32(1e-4),
        })
    );
    assert_eq!(
        parse_assignment("label=\"a=b\"", F32).unwrap().value,
        Literal::String("a=b".into())
    );
    assert!(matches!(
        parse_assignment("nx", F32),
        Err(AssignmentError::MissingEquals { .. })
    ));
    for name in ["", "if", "2x"] {
        let text = format!("{name}=1");
        assert!(
            matches!(parse_assignment(&text, F32), Err(AssignmentError::InvalidName { name: n }) if n == name),
            "{text:?}"
        );
    }
    let Err(AssignmentError::Value {
        error: LiteralError::RealRange { span, .. },
        ..
    }) = parse_assignment("dt=1e99", F32)
    else {
        panic!()
    };
    assert_eq!((span.start, span.end), (3, 7));
}

#[test]
fn source_number_spellings_convert_at_the_widths_asked_for() {
    assert_eq!(number("1_000", F32), Ok(Literal::Int(1000)));
    assert_eq!(number("2.5e1", F32), Ok(real32(25.0)));
    assert_eq!(number("2147483648", F32), Err(NumberError::IntRange));
    assert_eq!(number("1e40", F32), Err(NumberError::RealRange));
    // A sign belongs to the literal: the least Int has no positive spelling.
    assert_eq!(
        signed_number("2147483648", true, F32),
        Ok(Literal::Int(i32::MIN))
    );
    assert_eq!(
        signed_number("2147483649", true, F32),
        Err(NumberError::IntRange)
    );
    assert_eq!(
        signed_number("2147483648", false, F32),
        Err(NumberError::IntRange)
    );
    assert_eq!(signed_number("2.5", true, F32), Ok(real32(-2.5)));
}

/// Each width converts a Real spelling once, directly from its decimal
/// digits. Int spellings do not depend on the Real width.
#[test]
fn a_real_converts_once_to_the_width_asked_for() {
    // binary64 holds what binary32 cannot
    assert_eq!(
        number("1.0000000000000002", F64),
        Ok(Literal::Real(Real::F64(1.0000000000000002)))
    );
    assert_eq!(number("1.0000000000000002", F32), Ok(real32(1.0)));
    assert_eq!(number("1e40", F64), Ok(Literal::Real(Real::F64(1e40))));
    assert_eq!(number("1e400", F64), Err(NumberError::RealRange));
    assert_eq!(number("64", F64), Ok(Literal::Int(64)));
    // a negated zero keeps its sign at each width
    let Ok(Literal::Real(Real::F32(zero))) = signed_number("0.0", true, F32) else {
        panic!()
    };
    assert_eq!(zero.to_bits(), (-0.0f32).to_bits());
    let Ok(Literal::Real(Real::F64(zero))) = signed_number("0.0", true, F64) else {
        panic!()
    };
    assert_eq!(zero.to_bits(), (-0.0f64).to_bits());
    // a value configured on a command line is the value source spells
    for width in [F32, F64] {
        for (configured, written, negative) in
            [("-2.5e-3", "2.5e-3", true), ("1_0.25", "10.25", false)]
        {
            assert_eq!(
                parse_literal(configured, width).unwrap(),
                signed_number(written, negative, width).unwrap(),
                "{configured} at {width:?}"
            );
        }
    }
    // 1 + 2^-24 + 2^-80 rounds up to the next binary32, 0x3f800001, when
    // converted directly; through binary64 it would first lose 2^-80, land
    // exactly halfway and round to even, 1.0.
    let written =
        "1.00000005960464477539062582718061255302767487140869206996285356581211090087890625";
    let Ok(Literal::Real(Real::F32(direct))) = number(written, F32) else {
        panic!()
    };
    assert_eq!(direct.to_bits(), 0x3f80_0001);
    assert_eq!(
        (written.parse::<f64>().unwrap() as f32).to_bits(),
        0x3f80_0000
    );
    assert_eq!(Real::F32(direct).width(), F32);
}
