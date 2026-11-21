//! One Cartan literal from text, as command lines give param values:
//! `--set nx=64 dt=1e-4 label="run 1"`.
//!
//! The text is read by the ordinary expression parser, so a value is spelled
//! exactly as in source. A number containing `.`, `e` or `E` is a Real and any
//! other number an Int: a signed 32-bit Int, and a Real converted once, directly
//! from its spelling, to the [`RealWidth`] the caller asks for; `_` digit
//! separators are allowed. A leading `-` or `+` applies to a number.
//! `true`, `false`, `none` and quoted or triple-quoted strings are the other
//! literals; tagged templates and every other expression are not.

use crate::ast::{self, ExprKind, UnaryOperator};
use crate::grammar::grammar;
use crate::{Span, KEYWORDS};
use treegram::{Body, Rule};

/// The binary width a Real spelling is converted to. It names a conversion
/// only: which width a run uses, and where, is the caller's to decide.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RealWidth {
    F32,
    F64,
}

/// A Real value at the width its spelling was converted to. An `F32` is the
/// spelling rounded once to binary32, never by way of binary64, so no wider
/// value is recovered from it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Real {
    F32(f32),
    F64(f64),
}

impl Real {
    /// The width this value was converted to.
    pub fn width(self) -> RealWidth {
        match self {
            Real::F32(_) => RealWidth::F32,
            Real::F64(_) => RealWidth::F64,
        }
    }
}

/// A literal value.
#[derive(Clone, Debug, PartialEq)]
pub enum Literal {
    Int(i32),
    Real(Real),
    Bool(bool),
    /// The decoded text of a quoted or triple-quoted string.
    String(String),
    None,
}

/// Why text is not one literal. Spans index the parsed text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LiteralError {
    /// The text is not one well-formed expression.
    Syntax(Vec<ast::Error>),
    /// A well-formed expression other than a literal, such as `n * 2`.
    NotLiteral { span: Span },
    /// An Int spelling outside the signed 32-bit range.
    IntRange { written: String, span: Span },
    /// A Real spelling that is not finite at the width asked for.
    RealRange { written: String, span: Span },
}

/// One `NAME=VALUE` assignment.
#[derive(Clone, Debug, PartialEq)]
pub struct Assignment {
    pub name: String,
    pub value: Literal,
}

/// Why text is not one `NAME=VALUE` assignment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AssignmentError {
    /// No `=` separates a name from a value.
    MissingEquals { written: String },
    /// The text before the first `=` is not a name: empty, not a word, or a
    /// reserved word.
    InvalidName { name: String },
    /// The value is not a literal. Its spans index the whole assignment text,
    /// so a renderer can show the value in place.
    Value { name: String, error: LiteralError },
}

/// Parse `text` as exactly one literal, a Real at `width`.
pub fn parse_literal(text: &str, width: RealWidth) -> Result<Literal, LiteralError> {
    let expr = ast::parse_expression(text).map_err(LiteralError::Syntax)?;
    let span = expr.span;
    match expr.kind {
        ExprKind::Number(written) => literal_number(&written, false, width, span),
        ExprKind::Unary {
            operator: operator @ (UnaryOperator::Negate | UnaryOperator::Plus),
            operand,
        } => match &operand.kind {
            ExprKind::Number(written) => {
                literal_number(written, operator == UnaryOperator::Negate, width, span)
            }
            _ => Err(LiteralError::NotLiteral { span }),
        },
        ExprKind::Bool(value) => Ok(Literal::Bool(value)),
        ExprKind::String(string) => Ok(Literal::String(string.value)),
        ExprKind::None => Ok(Literal::None),
        _ => Err(LiteralError::NotLiteral { span }),
    }
}

/// Parse `text` as `NAME=VALUE`, splitting at the first `=`, a Real value at
/// `width`.
pub fn parse_assignment(text: &str, width: RealWidth) -> Result<Assignment, AssignmentError> {
    let Some(equals) = text.find('=') else {
        return Err(AssignmentError::MissingEquals {
            written: text.into(),
        });
    };
    let name = &text[..equals];
    if !is_name(name) {
        return Err(AssignmentError::InvalidName { name: name.into() });
    }
    // Blank the name and `=` so value spans index the whole assignment.
    let value = format!("{}{}", " ".repeat(equals + 1), &text[equals + 1..]);
    parse_literal(&value, width)
        .map(|value| Assignment {
            name: name.into(),
            value,
        })
        .map_err(|error| AssignmentError::Value {
            name: name.into(),
            error,
        })
}

fn is_name(text: &str) -> bool {
    let word = match grammar().def("identifier").map(|def| &def.body) {
        Some(Body::Rule(Rule::Pat(word))) => word,
        _ => unreachable!("the grammar's words are the identifier pattern"),
    };
    !text.is_empty() && word.longest(text, 0) == Some(text.len()) && !KEYWORDS.contains(&text)
}

/// Why a number spelling has no value at the Int width or the Real width
/// asked for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumberError {
    /// An Int spelling outside the signed 32-bit range.
    IntRange,
    /// A Real spelling that is not finite at the width asked for.
    RealRange,
}

/// The value of one number spelling as source writes it, such as the text of
/// an `ast::ExprKind::Number`: a Real at `width` when it contains `.`, `e` or
/// `E`, else an Int, with `_` digit separators allowed. The spelling has no
/// sign.
pub fn number(written: &str, width: RealWidth) -> Result<Literal, NumberError> {
    signed_number(written, false, width)
}

/// The value of a number spelling under a sign written before it, as one
/// literal: the sign is applied before the value is narrowed, so the least
/// Int, `-2147483648`, is a literal though `2147483648` is not. Source
/// analysis takes a negated number spelling through this, as
/// [`parse_literal`] does a configured one. A Real is rounded once, from the
/// decimal spelling to `width`.
pub fn signed_number(
    written: &str,
    negative: bool,
    width: RealWidth,
) -> Result<Literal, NumberError> {
    let digits: String = written.chars().filter(|&c| c != '_').collect();
    if written.contains(['.', 'e', 'E']) {
        let value = match width {
            RealWidth::F32 => digits
                .parse::<f32>()
                .ok()
                .filter(|value| value.is_finite())
                .map(|value| Real::F32(if negative { -value } else { value })),
            RealWidth::F64 => digits
                .parse::<f64>()
                .ok()
                .filter(|value| value.is_finite())
                .map(|value| Real::F64(if negative { -value } else { value })),
        };
        return value.map(Literal::Real).ok_or(NumberError::RealRange);
    }
    digits
        .parse::<i64>()
        .ok()
        .map(|value| if negative { -value } else { value })
        .and_then(|value| i32::try_from(value).ok())
        .map(Literal::Int)
        .ok_or(NumberError::IntRange)
}

fn literal_number(
    written: &str,
    negative: bool,
    width: RealWidth,
    span: Span,
) -> Result<Literal, LiteralError> {
    signed_number(written, negative, width).map_err(|error| match error {
        NumberError::IntRange => LiteralError::IntRange {
            written: written.into(),
            span,
        },
        NumberError::RealRange => LiteralError::RealRange {
            written: written.into(),
            span,
        },
    })
}
