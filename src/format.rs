//! Deterministic whitespace formatting of source documents.

use crate::{ast, parse_document, Node, Span};

/// A document that could not be formatted safely.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FormatError {
    /// The original document has grammar or source-form diagnostics, including
    /// constructs outside the supported source AST.
    Syntax(Vec<ast::Error>),
    /// The proposed spacing changed a written token or its parse structure.
    /// The span is in the original document.
    ChangedSyntax { span: Span },
}

/// Format a source document. Adjacent top-level functions are separated by one
/// blank line. Other line breaks and literal and comment text are preserved.
/// Only whitespace between written tokens is changed.
/// Formatting requires a document that the source AST accepts, and returns a
/// separate string only after the formatted text parses to the same concrete
/// syntax, apart from its byte positions.
pub fn format_document(source: &str) -> Result<String, FormatError> {
    let original = parse_document(source);
    ast::check_document(&original).map_err(FormatError::Syntax)?;

    let mut writer = Writer {
        source,
        output: String::with_capacity(source.len()),
        previous: None,
        end: 0,
        depth: 0,
    };
    writer.children(&original.root);
    let tail = &source[writer.end..];
    if !horizontal(tail) {
        writer.output.push_str(tail);
    }
    let formatted = writer.output;
    if formatted == source {
        return Ok(formatted);
    }

    let reparsed = parse_document(&formatted);
    if let Some(span) = first_change(&original.root, &reparsed.root, source, &formatted) {
        return Err(FormatError::ChangedSyntax { span });
    }
    if reparsed.has_errors() {
        return Err(FormatError::ChangedSyntax {
            span: span(&original.root),
        });
    }
    Ok(formatted)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Spacing {
    Plain,
    Comment,
    Prefix,
    Suffix,
    Tight,
    Open,
    AttachedOpen,
    Close,
    OpenBrace,
    CloseBrace,
    Range { start: bool, end: bool },
}

impl Spacing {
    fn of(node: &Node, parent: &Node) -> Self {
        if node.extra {
            return Self::Comment;
        }
        if node.named {
            return Self::Plain;
        }
        match node.kind.as_str() {
            "(" if matches!(
                parent.kind.as_str(),
                "parameter_list" | "argument_list" | "variant_arguments" | "constructor_pattern"
            ) =>
            {
                Self::AttachedOpen
            }
            "[" if parent.kind == "subscript" => Self::AttachedOpen,
            "<" if parent.kind == "fiber_type" => Self::AttachedOpen,
            ">" if parent.kind == "fiber_type" => Self::Close,
            "(" | "[" => Self::Open,
            ")" | "]" => Self::Close,
            "{" => Self::OpenBrace,
            "}" => Self::CloseBrace,
            "+" | "-" if parent.kind == "unary_expression" => Self::Prefix,
            ":" if node.field.as_deref() == Some("operator") => Self::Plain,
            ":" | "," | ";" | "?" => Self::Suffix,
            "::" => Self::Tight,
            _ if parent.kind == "primes" => Self::Tight,
            ".." | "..=" => Self::Range {
                start: parent
                    .child("left")
                    .or_else(|| parent.child("start"))
                    .is_some(),
                end: parent
                    .child("right")
                    .or_else(|| parent.child("end"))
                    .or_else(|| parent.child("axis"))
                    .is_some(),
            },
            _ => Self::Plain,
        }
    }

    fn opens(self) -> bool {
        matches!(self, Self::Open | Self::AttachedOpen | Self::OpenBrace)
    }

    fn closes(self) -> bool {
        matches!(self, Self::Close | Self::CloseBrace)
    }

    fn space_after(self, right: Self) -> bool {
        if right == Self::Comment {
            return true;
        }
        !matches!(
            self,
            Self::Prefix
                | Self::Tight
                | Self::Open
                | Self::AttachedOpen
                | Self::Range { end: true, .. }
        ) && !matches!(
            right,
            Self::Suffix
                | Self::Tight
                | Self::AttachedOpen
                | Self::Close
                | Self::Range { start: true, .. }
        ) && (self, right) != (Self::OpenBrace, Self::CloseBrace)
    }
}

struct Writer<'a> {
    source: &'a str,
    output: String,
    previous: Option<(&'a str, Spacing)>,
    end: usize,
    depth: usize,
}

impl Writer<'_> {
    fn children(&mut self, parent: &Node) {
        let mut separator = None;
        if parent.kind == "source_file" {
            let items: Vec<_> = parent
                .children
                .iter()
                .enumerate()
                .filter(|(_, node)| {
                    !node.extra && !matches!(node.kind.as_str(), "\n" | "doc_declaration")
                })
                .collect();
            separator = Some(
                items
                    .windows(2)
                    .filter(|pair| {
                        pair.iter()
                            .all(|(_, node)| node.kind == "function_definition")
                    })
                    .filter_map(|pair| {
                        let start = pair[0].0 + 1;
                        let end = pair[1].0;
                        (start..end).find(|&at| parent.children[at].kind == "\n")
                    })
                    .collect::<Vec<_>>(),
            );
        }
        let mut at = 0;
        while at < parent.children.len() {
            let node = &parent.children[at];
            if separator
                .as_ref()
                .is_some_and(|starts| starts.contains(&at))
            {
                self.token(node, parent);
                let newline = if self.source[..node.start].ends_with('\r') {
                    "\r\n"
                } else {
                    "\n"
                };
                self.output.push_str(newline);
                while parent
                    .children
                    .get(at + 1)
                    .is_some_and(|next| next.kind == "\n")
                {
                    at += 1;
                    self.end = parent.children[at].end;
                }
            } else if node.children.is_empty()
                || node.extra
                || matches!(
                    node.kind.as_str(),
                    "string" | "triple_string" | "template_string"
                )
            {
                self.token(node, parent);
            } else {
                self.children(node);
            }
            at += 1;
        }
    }

    fn token(&mut self, node: &Node, parent: &Node) {
        let text = node.text(self.source);
        let spacing = Spacing::of(node, parent);
        if spacing.closes() {
            self.depth = self.depth.saturating_sub(1);
        }
        let gap = &self.source[self.end..node.start];
        if !horizontal(gap) {
            self.output.push_str(gap);
        } else if !text.starts_with('\n') {
            match self.previous {
                None => {}
                Some((last, _)) if last.ends_with('\n') => {
                    for _ in 0..self.depth {
                        self.output.push_str("    ");
                    }
                }
                Some((last, previous))
                    if !last.ends_with([' ', '\t', '\r']) && previous.space_after(spacing) =>
                {
                    self.output.push(' ');
                }
                Some(_) => {}
            }
        }
        self.output.push_str(text);
        self.previous = Some((text, spacing));
        self.end = node.end;
        self.depth += usize::from(spacing.opens());
    }
}

fn horizontal(text: &str) -> bool {
    text.bytes().all(|byte| matches!(byte, b' ' | b'\t'))
}

fn span(node: &Node) -> Span {
    Span {
        start: node.start,
        end: node.end,
    }
}

fn first_change(before: &Node, after: &Node, source: &str, formatted: &str) -> Option<Span> {
    if before.kind != after.kind
        || before.field != after.field
        || before.named != after.named
        || before.extra != after.extra
    {
        return Some(span(before));
    }
    if before.children.is_empty() && before.kind != "source_file" {
        return (before.text(source) != after.text(formatted)).then(|| span(before));
    }
    // Only document-level separators may change count. Newlines inside a
    // function or expression remain part of its exact concrete syntax.
    fn children(node: &Node) -> Vec<&Node> {
        node.children
            .iter()
            .filter(|child| node.kind != "source_file" || child.kind != "\n")
            .collect()
    }
    let old = children(before);
    let new = children(after);
    if old.len() != new.len() {
        return Some(span(before));
    }
    old.into_iter()
        .zip(new)
        .find_map(|(before, after)| first_change(before, after, source, formatted))
}
