//! Facts an interactive prompt needs while text is being typed: whether the
//! text is ready to submit, and how to color it.
//!
//! [`completeness`] reads the text as a document and as one expression, using
//! only the typed [`Diagnostic::incomplete`](crate::Diagnostic::incomplete)
//! fact. [`highlight`] colors text line by line from the grammar's lexical
//! layer, the same keyword, constant and operator sets the editor grammars
//! print, so partially typed text colors before it parses.

use crate::grammar::{grammar, CONSTANTS};
use crate::{parse_document, parse_expression, Span, SyntaxTree};
use std::sync::OnceLock;
use treegram::{Body, Lexical, Pat, Rule};

/// Whether prompt text is ready to submit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Completeness {
    /// The text parses as a document or as one expression.
    Complete,
    /// Neither reading parses, and at least one stopped at the end of the
    /// text waiting for more, such as an open bracket or a trailing `=`.
    Incomplete,
    /// Neither reading parses, and neither is waiting for more text.
    Invalid,
}

/// How a span of prompt text is colored.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HighlightClass {
    Keyword,
    /// `true`, `false` and `none`.
    Constant,
    Number,
    /// A quoted, triple-quoted or tagged literal, its delimiters included.
    String,
    Comment,
    Operator,
    /// Brackets and commas.
    Punctuation,
    /// Any other word.
    Name,
}

/// One colored span of prompt text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Highlight {
    pub span: Span,
    pub class: HighlightClass,
}

/// Classify prompt text by the grammar alone. Complete text can still fail
/// source normalization; parse it with [`crate::ast::parse_document`] to
/// report that.
pub fn completeness(text: &str) -> Completeness {
    let readings = [
        reading(&parse_document(text)),
        reading(&parse_expression(text)),
    ];
    if readings.contains(&Reading::Closed) {
        Completeness::Complete
    } else if readings.contains(&Reading::Waiting) {
        Completeness::Incomplete
    } else {
        Completeness::Invalid
    }
}

#[derive(PartialEq)]
enum Reading {
    Closed,
    Waiting,
    Wrong,
}

fn reading(tree: &SyntaxTree) -> Reading {
    match tree.diagnostics.first() {
        None => Reading::Closed,
        Some(diagnostic) if diagnostic.incomplete => Reading::Waiting,
        Some(_) => Reading::Wrong,
    }
}

struct Lexis {
    lexical: Lexical,
    word: &'static Pat,
}

fn lexis() -> &'static Lexis {
    static LEXIS: OnceLock<Lexis> = OnceLock::new();
    LEXIS.get_or_init(|| {
        let g = grammar();
        let word = match g.def("identifier").map(|def| &def.body) {
            Some(Body::Rule(Rule::Pat(word))) => word,
            _ => unreachable!("the grammar's words are the identifier pattern"),
        };
        Lexis {
            lexical: Lexical::of(g, CONSTANTS),
            word,
        }
    })
}

/// Color `text` in source order with non-overlapping byte spans. Whitespace
/// and characters no class describes have no entry. Template holes color as
/// part of their literal, and an unterminated literal colors to its line's
/// end (a triple-quoted one to the end of the text).
pub fn highlight(text: &str) -> Vec<Highlight> {
    let lexis = lexis();
    let mut highlights = Vec::new();
    let mut pos = 0;
    while let Some(c) = text[pos..].chars().next() {
        let rest = &text[pos..];
        let (class, end) = if c.is_whitespace() {
            pos += c.len_utf8();
            continue;
        } else if c == '#' {
            (HighlightClass::Comment, line_end(text, pos))
        } else if let Some(open) = ["f\"\"\"", "\"\"\""].iter().find(|q| rest.starts_with(**q)) {
            let body = pos + open.len();
            let end = text[body..]
                .find("\"\"\"")
                .map_or(text.len(), |at| body + at + 3);
            (HighlightClass::String, end)
        } else if let Some(open) = ["f\"", "\""].iter().find(|q| rest.starts_with(**q)) {
            (HighlightClass::String, quoted_end(text, pos + open.len()))
        } else if c.is_ascii_digit() {
            (HighlightClass::Number, number_end(text, pos))
        } else if let Some(end) = lexis.word.longest(text, pos).filter(|&end| end > pos) {
            let word = &text[pos..end];
            let class = if lexis.lexical.constants.iter().any(|w| w == word) {
                HighlightClass::Constant
            } else if lexis.lexical.keywords.iter().any(|w| w == word) {
                HighlightClass::Keyword
            } else {
                HighlightClass::Name
            };
            (class, end)
        } else if let Some(op) = lexis
            .lexical
            .operators
            .iter()
            .find(|op| rest.starts_with(op.as_str()))
        {
            (HighlightClass::Operator, pos + op.len())
        } else if "()[]{},".contains(c) {
            (HighlightClass::Punctuation, pos + 1)
        } else {
            pos += c.len_utf8();
            continue;
        };
        highlights.push(Highlight {
            span: Span { start: pos, end },
            class,
        });
        pos = end;
    }
    highlights
}

fn line_end(text: &str, pos: usize) -> usize {
    text[pos..].find('\n').map_or(text.len(), |at| pos + at)
}

/// The end of a quoted literal whose body starts at `body`: after its closing
/// quote, or at the line's end when unterminated. A backslash escapes the next
/// character.
fn quoted_end(text: &str, body: usize) -> usize {
    let mut chars = text[body..].char_indices();
    while let Some((at, c)) = chars.next() {
        match c {
            '"' => return body + at + 1,
            '\n' => return body + at,
            '\\' => {
                chars.next();
            }
            _ => {}
        }
    }
    text.len()
}

/// The end of a number as the grammar spells one: digits with `_`, then an
/// optional `.digits` (so `0..10` keeps its range) and exponent.
fn number_end(text: &str, pos: usize) -> usize {
    let bytes = text.as_bytes();
    let digits = |mut at: usize| {
        while at < bytes.len() && (bytes[at].is_ascii_digit() || bytes[at] == b'_') {
            at += 1;
        }
        at
    };
    let mut end = digits(pos);
    if bytes.get(end) == Some(&b'.') && bytes.get(end + 1).is_some_and(u8::is_ascii_digit) {
        end = digits(end + 1);
    }
    if matches!(bytes.get(end), Some(b'e' | b'E')) {
        let sign = usize::from(matches!(bytes.get(end + 1), Some(b'+' | b'-')));
        if bytes.get(end + 1 + sign).is_some_and(u8::is_ascii_digit) {
            end = digits(end + 1 + sign);
        }
    }
    end
}
