//! The readable source syntax tree: an indented outline of the serialized AST.
//!
//! The JSON view and this outline print the same serialized value. Here a node
//! with a `kind` and a `span` reads as its variant name, its written text and
//! its source position; other fields are indented beneath it. Positions are
//! `line:column` ranges, columns counting Unicode scalar values; the JSON view
//! keeps the exact byte spans. Color changes only escapes, never the text.

use serde_json::{Map, Value};
use std::path::Path;

/// The outline of `items`, the serialized items of the document at `path`.
pub(crate) fn render(path: &Path, text: &str, items: &Value, colored: bool) -> String {
    let printer = Printer {
        lines: line_starts(text),
        text,
        colored,
    };
    let mut out = printer.paint(BOLD, &path.display().to_string());
    for item in items.as_array().into_iter().flatten() {
        printer.node(&mut out, 1, None, item);
    }
    out
}

const BOLD: &str = "1";
const VARIANT: &str = "1;36";
const FIELD: &str = "2";
const WRITTEN: &str = "33";
const POSITION: &str = "2";

struct Printer<'a> {
    lines: Vec<usize>,
    text: &'a str,
    colored: bool,
}

/// One node as an outline line reads it.
#[derive(Default)]
struct Line<'v> {
    label: Option<String>,
    written: Option<String>,
    span: Option<(usize, usize)>,
    children: Vec<(Option<&'v str>, &'v Value)>,
}

impl Printer<'_> {
    fn node(&self, out: &mut String, depth: usize, field: Option<&str>, value: &Value) {
        let line = describe(value);
        out.push('\n');
        out.push_str(&"  ".repeat(depth));
        match field {
            Some(field) => out.push_str(&self.paint(FIELD, &format!("{field}: "))),
            None if depth > 1 => out.push_str("- "),
            None => {}
        }
        let mut words = Vec::new();
        if let Some(label) = &line.label {
            words.push(self.paint(VARIANT, label));
        }
        if let Some(written) = &line.written {
            words.push(self.paint(WRITTEN, written));
        }
        if let Some((start, end)) = line.span {
            words.push(self.paint(POSITION, &self.position(start, end)));
        }
        out.push_str(&words.join(" "));
        for (name, child) in line.children {
            match child {
                // A list's elements stand one level below its field name.
                Value::Array(elements) => {
                    out.push('\n');
                    out.push_str(&"  ".repeat(depth + 1));
                    out.push_str(&self.paint(FIELD, &format!("{}:", name.unwrap_or("items"))));
                    for element in elements {
                        self.node(out, depth + 2, None, element);
                    }
                }
                child => self.node(out, depth + 1, name, child),
            }
        }
    }

    /// `line:column-line:column`, 1-based, the end exclusive.
    fn position(&self, start: usize, end: usize) -> String {
        let (line, column) = self.line_column(start);
        let (end_line, end_column) = self.line_column(end);
        if line == end_line {
            format!("{line}:{column}-{end_column}")
        } else {
            format!("{line}:{column}-{end_line}:{end_column}")
        }
    }

    fn line_column(&self, offset: usize) -> (usize, usize) {
        let line = self.lines.partition_point(|&start| start <= offset);
        let start = self.lines[line - 1];
        let column = self
            .text
            .get(start..offset)
            .map_or(offset - start, |text| text.chars().count());
        (line, column + 1)
    }

    fn paint(&self, style: &str, text: &str) -> String {
        if self.colored {
            format!("\x1b[{style}m{text}\x1b[0m")
        } else {
            text.to_owned()
        }
    }
}

fn line_starts(text: &str) -> Vec<usize> {
    std::iter::once(0)
        .chain(text.match_indices('\n').map(|(at, _)| at + 1))
        .collect()
}

fn span(value: &Value) -> Option<(usize, usize)> {
    let span = value.as_object()?;
    if span.len() != 2 {
        return None;
    }
    Some((
        span.get("start")?.as_u64()? as usize,
        span.get("end")?.as_u64()? as usize,
    ))
}

/// A serialized enum variant: an object whose one key is a capitalized name.
fn variant(object: &Map<String, Value>) -> Option<(&str, &Value)> {
    let (name, value) = object.iter().next().filter(|_| object.len() == 1)?;
    name.starts_with(|c: char| c.is_ascii_uppercase())
        .then_some((name.as_str(), value))
}

fn describe(value: &Value) -> Line<'_> {
    let mut line = Line::default();
    match value {
        Value::Object(object) => {
            if let Some((name, inner)) = variant(object) {
                line = describe(inner);
                // A wrapped variant names the node; a payload's own label
                // (a nested variant) reads after it.
                line.written = match (line.label.take(), line.written.take()) {
                    (Some(label), Some(written)) => Some(format!("{label} {written}")),
                    (label, written) => label.or(written),
                };
                line.label = Some(name.to_owned());
                return line;
            }
            // A node: its `kind` names it and its `span` places it.
            let node = object.contains_key("span") && object.contains_key("kind");
            if node {
                line = describe(&object["kind"]);
            }
            for (name, field) in object {
                match name.as_str() {
                    "kind" if node => {}
                    "span" if span(field).is_some() => line.span = line.span.or(span(field)),
                    "text" | "raw" if field.is_string() && line.written.is_none() => {
                        line.written = field.as_str().map(str::to_owned);
                    }
                    _ if field.is_null() => {}
                    _ if field.as_array().is_some_and(Vec::is_empty) => {}
                    _ => line.children.push((Some(name), field)),
                }
            }
        }
        Value::Array(elements) => line.children = elements.iter().map(|e| (None, e)).collect(),
        Value::String(text) => line.written = Some(text.clone()),
        other => line.written = Some(other.to_string()),
    }
    line
}
