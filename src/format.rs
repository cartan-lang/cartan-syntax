//! Deterministic whitespace formatting of source documents.

use crate::{ast, parse_document, Node, Span};
use std::collections::HashMap;

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

/// Format a source document.
///
/// Only whitespace between written tokens changes: no token and no line break
/// is added or removed, except that adjacent top-level functions are separated
/// by one blank line. Literal and comment text is kept as written, and so are
/// CRLF line ends.
///
/// - Spacing within a line is fixed per token: one space around binary
///   operators, none inside brackets or before `,` and `:`, and none around
///   `^` or the `|` of a type.
/// - Each line is indented from the line that opened the bracket around it,
///   four columns a level; a line that continues an unfinished one is
///   indented one level more.
/// - An operator that opens a line continues the expression above. The
///   operators of one chain stand in one column, and a first operand written
///   on its own line stands in the operand column, so a sum written one term
///   to a line reads `      a` / `    + b` / `    - c`.
/// - `else` that opens a line stands under the `if` that begins its chain.
/// - The `=` or `:=` of declarations on adjacent lines, at one indentation,
///   of one kind, each naming one name without an annotation, are lined up
///   in one column; any other line ends the run.
///
/// The work is linear in the document, times the bracket depth.
/// Formatting requires a document that the source AST accepts, and returns a
/// separate string only after the formatted text parses to the same concrete
/// syntax, apart from its byte positions and the indentation that a
/// line-opening operator carries.
pub fn format_document(source: &str) -> Result<String, FormatError> {
    let original = parse_document(source);
    ast::check_document(&original).map_err(FormatError::Syntax)?;

    // once to find where the `=` of adjacent declarations stand, and once
    // more with each moved under the furthest of its run
    let (plain, marks) = write(source, &original.root, &HashMap::new());
    let pads = aligned(&plain, &marks);
    let formatted = if pads.is_empty() {
        plain
    } else {
        write(source, &original.root, &pads).0
    };
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
                "parameter_list"
                    | "argument_list"
                    | "variant_arguments"
                    | "constructor_pattern"
                    | "match_arm"
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
            "|" if parent.kind == "type_annotation" => Self::Tight,
            "^" if parent.kind == "binary_expression" => Self::Tight,
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

/// One written token, comment or newline, in source order.
struct Leaf<'a> {
    node: &'a Node,
    parent: &'a Node,
    /// The source between the leaf before it and this one.
    gap: &'a str,
    /// For a newline: the line it ends has not finished, so the next line
    /// continues it.
    unfinished: bool,
    /// For a newline: one blank line follows it, whatever was written.
    separate: bool,
    /// For the `if` and `else` of a conditional: where the first
    /// conditional of its `else if` chain starts in the source.
    chain: Option<usize>,
}

/// The leaves of `parent` in order, with the newlines between adjacent
/// functions reduced to the one that a blank line follows.
fn leaves<'a>(
    source: &'a str,
    parent: &'a Node,
    chain: Option<usize>,
    end: &mut usize,
    out: &mut Vec<Leaf<'a>>,
) {
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
        let separate = separator
            .as_ref()
            .is_some_and(|starts| starts.contains(&at));
        if separate
            || node.children.is_empty()
            || node.extra
            || matches!(
                node.kind.as_str(),
                "string" | "triple_string" | "template_string"
            )
        {
            // a newline after a token that wants more leaves its line
            // unfinished: an operator, `=`, a keyword. After an item, an
            // opener or a comma it separates.
            let unfinished = node.kind == "\n"
                && parent.children[..at]
                    .iter()
                    .rev()
                    .find(|before| !before.extra && before.kind != "\n")
                    .is_some_and(|before| {
                        let spacing = Spacing::of(before, parent);
                        !before.named
                            && before.children.is_empty()
                            && !spacing.opens()
                            && !spacing.closes()
                            && spacing != Spacing::Suffix
                    });
            out.push(Leaf {
                node,
                parent,
                gap: &source[*end..node.start],
                unfinished,
                separate,
                chain: (parent.kind == "conditional_expression")
                    .then_some(chain)
                    .flatten(),
            });
            *end = node.end;
            if separate {
                while parent
                    .children
                    .get(at + 1)
                    .is_some_and(|next| next.kind == "\n")
                {
                    at += 1;
                    *end = parent.children[at].end;
                }
            }
        } else {
            let after_else = parent.kind == "conditional_expression"
                && parent.children[..at]
                    .iter()
                    .rev()
                    .find(|before| !before.extra && before.kind != "\n")
                    .is_some_and(|before| before.kind == "else");
            let chain = match node.kind.as_str() {
                "conditional_expression" if after_else => chain,
                "conditional_expression" => Some(node.start),
                _ => None,
            };
            leaves(source, node, chain, end, out);
        }
        at += 1;
    }
}

/// What a bracket holds: where the items inside it are indented, where
/// the line that opened it is, and where the operators of the chain being
/// written inside it stand.
struct Frame {
    inside: usize,
    line: usize,
    operators: Option<usize>,
}

/// How the line being started relates to the one above it.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Line {
    /// Not at the start of a line.
    Within,
    /// A fresh item.
    Item,
    /// The line above had not finished.
    Continued,
}

const INDENT: usize = 4;

/// The `=` or `:=` of a declaration that begins its line, has no annotation
/// and names one name on its left: where it was written and what it could
/// line up with. A declaration whose value runs onto later lines joins a run
/// by the line it begins on.
struct Mark {
    leaf: usize,
    /// Where the operator begins in the output.
    at: usize,
    column: usize,
    indent: usize,
    form: (String, String),
}

/// The spaces to put before each marked operator so that those of
/// declarations of one form on adjacent lines stand in one column.
fn aligned(output: &str, marks: &[Mark]) -> HashMap<usize, usize> {
    let mut pads = HashMap::new();
    let mut lines = 0;
    let mut scanned = 0;
    let lines: Vec<usize> = marks
        .iter()
        .map(|mark| {
            lines += output[scanned..mark.at].matches('\n').count();
            scanned = mark.at;
            lines
        })
        .collect();
    let mut start = 0;
    while start < marks.len() {
        let mut end = start + 1;
        while end < marks.len()
            && lines[end] == lines[end - 1] + 1
            && marks[end].indent == marks[start].indent
            && marks[end].form == marks[start].form
        {
            end += 1;
        }
        let column = marks[start..end].iter().map(|mark| mark.column).max();
        for mark in &marks[start..end] {
            let pad = column.unwrap_or(mark.column) - mark.column;
            if pad > 0 {
                pads.insert(mark.leaf, pad);
            }
        }
        start = end;
    }
    pads
}

/// The document written out, with the given spaces before the operators
/// at those leaves, and the operators that could be lined up.
fn write(source: &str, root: &Node, pads: &HashMap<usize, usize>) -> (String, Vec<Mark>) {
    let mut end = 0;
    let mut written = Vec::new();
    leaves(source, root, None, &mut end, &mut written);
    let mut writer = Writer {
        source,
        leaves: written,
        output: String::with_capacity(source.len()),
        previous: None,
        frames: vec![Frame {
            inside: 0,
            line: 0,
            operators: None,
        }],
        line: Line::Item,
        spaced: false,
        conditionals: HashMap::new(),
        pads,
        marks: Vec::new(),
        opened: None,
        operand: None,
    };
    for at in 0..writer.leaves.len() {
        writer.leaf(at);
    }
    let tail = &source[end..];
    if !horizontal(tail) {
        writer.output.push_str(tail);
    }
    (writer.output, writer.marks)
}

struct Writer<'a> {
    source: &'a str,
    leaves: Vec<Leaf<'a>>,
    output: String,
    previous: Option<(&'a str, Spacing)>,
    frames: Vec<Frame>,
    line: Line,
    /// A space follows the leaf just written whatever its spacing says.
    spaced: bool,
    /// The column of the `if` that begins each `else if` chain.
    conditionals: HashMap<usize, usize>,
    pads: &'a HashMap<usize, usize>,
    marks: Vec<Mark>,
    /// Where in the source the item that began this line starts, and the
    /// line's indentation.
    opened: Option<(usize, usize)>,
    /// After an operator that opens a line and ends it too: the column its
    /// operand takes on the next line.
    operand: Option<usize>,
}

/// The operator of a token that opens a line and continues the one above:
/// its leading newlines, the operator, and whether a newline ends it.
fn continuation(text: &str) -> Option<(&str, &str, bool)> {
    let operator = text.trim_start_matches(['\n', ' ', '\t', '\r']);
    let lead = &text[..text.len() - operator.len()];
    lead.contains('\n').then(|| {
        (
            lead,
            operator.trim_end_matches(['\n', ' ', '\t', '\r']),
            operator.ends_with('\n'),
        )
    })
}

impl<'a> Writer<'a> {
    fn column(&self) -> usize {
        let line = self.output.rfind('\n').map_or(0, |at| at + 1);
        self.output[line..].chars().count()
    }

    /// The operator that opens the next line at this nesting, where that
    /// line continues an expression beginning at the leaf `at`.
    fn chain_after(&self, at: usize) -> Option<&'a str> {
        let mut nesting = 0usize;
        for (offset, leaf) in self.leaves[at..].iter().enumerate() {
            let spacing = Spacing::of(leaf.node, leaf.parent);
            if spacing.closes() {
                nesting = nesting.checked_sub(1)?;
            }
            if nesting == 0 && offset > 0 {
                let text = leaf.node.text(self.source);
                if let Some((_, operator, _)) = continuation(text) {
                    return (leaf.parent.kind == "binary_expression"
                        && leaf.parent.start == self.leaves[at].node.start)
                        .then_some(operator);
                }
                if leaf.node.kind == "\n" || leaf.gap.contains('\n') {
                    return None;
                }
            }
            nesting += usize::from(spacing.opens());
        }
        None
    }

    fn indent(&mut self, width: usize) {
        for _ in 0..width {
            self.output.push(' ');
        }
        self.line = Line::Within;
    }

    fn leaf(&mut self, at: usize) {
        let leaf = &self.leaves[at];
        let (node, parent, gap) = (leaf.node, leaf.parent, leaf.gap);
        let (unfinished, separate, chain) = (leaf.unfinished, leaf.separate, leaf.chain);
        let text = node.text(self.source);
        let spacing = Spacing::of(node, parent);
        let closed = spacing
            .closes()
            .then(|| (self.frames.len() > 1).then(|| self.frames.pop()).flatten())
            .flatten();
        if !horizontal(gap) {
            // written whitespace the grammar gave no token: kept as it is
            self.output.push_str(gap);
            self.line = Line::Within;
            self.previous = None;
        }
        let top = self.frames.last().expect("the document's frame");
        let (inside, operators) = (top.inside, top.operators);
        let mut written = text;
        if node.kind == "\n" {
            self.output.push_str(text);
            if separate {
                let newline = if self.source[..node.start].ends_with('\r') {
                    "\r\n"
                } else {
                    "\n"
                };
                self.output.push_str(newline);
            }
            self.line = if unfinished {
                Line::Continued
            } else {
                Line::Item
            };
            self.opened = None;
            if !unfinished {
                self.operand = None;
            }
            self.previous = Some((text, spacing));
            return;
        }
        let mut ends_line = false;
        if let Some((lead, operator, newline)) = continuation(text) {
            // the newlines the token carries, without their indentation
            let mut characters = lead.chars().peekable();
            while let Some(character) = characters.next() {
                if character == '\n' || (character == '\r' && characters.peek() == Some(&'\n')) {
                    self.output.push(character);
                }
            }
            let after = self.leaves[..at]
                .iter()
                .rev()
                .find(|before| !before.node.extra && before.node.kind != "\n")
                .map(|before| Spacing::of(before.node, before.parent));
            let fresh = after.is_none_or(|after| after.opens() || after == Spacing::Suffix);
            let width = if node.kind == "else" && parent.kind == "conditional_expression" {
                chain
                    .and_then(|chain| self.conditionals.get(&chain).copied())
                    .unwrap_or(inside)
            } else if fresh || node.kind == "else" {
                inside
            } else {
                operators.unwrap_or(inside + INDENT)
            };
            self.indent(width);
            written = operator;
            ends_line = newline;
            self.operand = newline.then(|| width + operator.chars().count() + 1);
        } else if self.line != Line::Within {
            let width = if let Some(frame) = &closed {
                frame.line
            } else {
                let base = match self.line {
                    Line::Continued => self.operand.unwrap_or(inside + INDENT),
                    _ => inside,
                };
                if !node.extra {
                    self.opened = (self.line == Line::Item).then_some((node.start, base));
                }
                let sign = spacing == Spacing::Prefix && matches!(text, "+" | "-");
                let operator = (!node.extra).then(|| self.chain_after(at)).flatten();
                if !node.extra {
                    self.frames.last_mut().expect("a frame").operators = operator.map(|_| base);
                }
                match operator {
                    // a sign that leads the first term of a continued line
                    // stands where the operators do, one space from its term
                    Some(_) if sign && self.line == Line::Continued => {
                        self.spaced = true;
                        base
                    }
                    Some(operator) => base + operator.chars().count() + 1,
                    None => base,
                }
            };
            self.indent(width);
        } else if !text.starts_with('\n') {
            match self.previous {
                Some((last, previous))
                    if !last.ends_with([' ', '\t', '\r', '\n'])
                        && (std::mem::take(&mut self.spaced) || previous.space_after(spacing)) =>
                {
                    self.output.push(' ');
                }
                _ => {}
            }
        }
        if matches!(node.kind.as_str(), "=" | ":=")
            && matches!(
                parent.kind.as_str(),
                "port_declaration" | "binding" | "local"
            )
            && parent.children.iter().all(|child| child.kind != ":")
            && parent
                .child("name")
                .or_else(|| parent.child("pattern"))
                .is_some_and(|left| tokens(left) == 1)
        {
            if let Some((_, indent)) = self.opened.filter(|(start, _)| *start == parent.start) {
                for _ in 0..self.pads.get(&at).copied().unwrap_or(0) {
                    self.output.push(' ');
                }
                self.marks.push(Mark {
                    leaf: at,
                    at: self.output.len(),
                    column: self.column(),
                    indent,
                    form: (parent.kind.clone(), text.to_string()),
                });
            }
        }
        if node.kind == "if" && parent.kind == "conditional_expression" {
            if let Some(chain) = chain {
                let column = self.column();
                self.conditionals.entry(chain).or_insert(column);
            }
        }
        self.output.push_str(written);
        self.previous = Some((written, spacing));
        if ends_line {
            self.output.push('\n');
            self.line = Line::Continued;
        }
        if spacing.opens() {
            let line = self.output.rfind('\n').map_or(0, |at| at + 1);
            let line = self.output[line..]
                .chars()
                .take_while(|character| *character == ' ')
                .count();
            self.frames.push(Frame {
                inside: line + INDENT,
                line,
                operators: None,
            });
        }
    }
}

/// The written tokens under `node`, comments aside.
fn tokens(node: &Node) -> usize {
    if node.extra {
        0
    } else if node.children.is_empty() {
        1
    } else {
        node.children.iter().map(tokens).sum()
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
        // a token that opens a line carries that line's indentation, which
        // is the formatter's to choose
        let written = |text: &'_ str| -> String {
            match continuation(text) {
                Some((_, operator, _)) if !before.named && operator == before.kind => text
                    .chars()
                    .filter(|character| !matches!(character, ' ' | '\t' | '\r'))
                    .collect(),
                _ => text.to_string(),
            }
        };
        return (written(before.text(source)) != written(after.text(formatted)))
            .then(|| span(before));
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
