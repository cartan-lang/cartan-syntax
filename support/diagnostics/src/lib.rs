//! Source-independent diagnostic descriptions and plain-text/JSON rendering.
//!
//! Reports are presentation values, not semantic errors. Callers keep the original
//! typed failure. Byte spans and source IDs retain their original source-table owner.
//! Rendering is ANSI-free; terminal callers choose color and output policy.

use serde_json::{json, Value};
use std::path::Path;

/// A borrowed source snapshot; its position in a table is a diagnostic-local identity.
#[derive(Clone, Copy, Debug)]
pub struct Source<'a> {
    pub name: &'a Path,
    pub text: &'a str,
}

/// Half-open UTF-8 byte offsets in the original source.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

/// An index in the caller-supplied source snapshot, never an executable identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceLocation {
    pub source: usize,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Note,
}

impl Severity {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Note => "note",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Label {
    pub location: SourceLocation,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Report {
    pub code: &'static str,
    pub severity: Severity,
    pub message: String,
    pub primary: Option<Label>,
    pub related: Vec<Label>,
    pub notes: Vec<String>,
    pub fields: Value,
}

impl Report {
    pub fn error(
        code: &'static str,
        message: impl Into<String>,
        location: Option<SourceLocation>,
    ) -> Self {
        Self {
            code,
            severity: Severity::Error,
            message: message.into(),
            primary: location.map(|location| Label {
                location,
                message: String::new(),
            }),
            related: Vec::new(),
            notes: Vec::new(),
            fields: json!({}),
        }
    }

    /// Stable envelope; facts never depend on parsing the human message.
    pub fn to_json(&self) -> Value {
        let label = |label: &Label| {
            json!({"source":label.location.source,
            "span":{"start":label.location.span.start,"end":label.location.span.end},
            "message":label.message})
        };
        json!({"code":self.code,"severity":self.severity.as_str(),"message":self.message,
            "primary":self.primary.as_ref().map(label),
            "related":self.related.iter().map(label).collect::<Vec<_>>(),
            "notes":self.notes,"fields":self.fields})
    }
}

/// JSON envelope with source names from the original snapshot. Byte spans remain exact.
pub fn to_json(sources: &[Source<'_>], report: &Report) -> Value {
    let mut value = report.to_json();
    value["sources"] = json!(sources
        .iter()
        .enumerate()
        .map(|(id, source)| json!({"id":id,"name":source.name.to_string_lossy(),"name_lossy":source.name.to_str().is_none()}))
        .collect::<Vec<_>>());
    value
}

/// Show the first source line at each location, retaining full byte spans in the
/// report. Columns count Unicode scalar values; tabs expand to four-column stops.
/// Invalid/missing source locations use their original byte range, never a guessed caret.
pub fn render(sources: &[Source<'_>], report: &Report) -> String {
    let mut text = format!(
        "{}[{}]: {}",
        report.severity.as_str(),
        report.code,
        report.message
    );
    for label in report.primary.iter().chain(&report.related) {
        render_label(&mut text, sources, label);
    }
    for note in &report.notes {
        text.push_str(&format!("\n  note: {note}"));
    }
    text
}

fn render_label(text: &mut String, sources: &[Source<'_>], label: &Label) {
    let SourceLocation {
        source,
        span: Span { start, end },
    } = label.location;
    let Some(document) = sources.get(source) else {
        text.push_str(&format!(
            "\n  --> <unknown source {}>:{start}..{end}",
            source
        ));
        if !label.message.is_empty() {
            text.push_str(&format!(": {}", label.message));
        }
        return;
    };
    let content = &document.text;
    if start > end
        || end > content.len()
        || !content.is_char_boundary(start)
        || !content.is_char_boundary(end)
    {
        text.push_str(&format!(
            "\n  --> {}:{start}..{end}",
            document.name.display()
        ));
        if !label.message.is_empty() {
            text.push_str(&format!(": {}", label.message));
        }
        return;
    }
    let line_start = content[..start].rfind('\n').map_or(0, |i| i + 1);
    let line_end = content[start..]
        .find('\n')
        .map_or(content.len(), |i| start + i);
    let number = content[..start].bytes().filter(|&b| b == b'\n').count() + 1;
    let (expanded, line_width) = expand(&content[line_start..line_end]);
    let first = expand(&content[line_start..start]).1;
    let last = if end >= line_end {
        line_width
    } else {
        expand(&content[line_start..end]).1
    };
    let width = last.saturating_sub(first).max(1);
    let gutter = number.to_string().len();
    text.push_str(&format!(
        "\n  --> {}:{number}:{}\n{:gutter$} |\n{number} | {expanded}\n{:gutter$} | {}{}",
        document.name.display(),
        first + 1,
        "",
        "",
        " ".repeat(first),
        "^".repeat(width)
    ));
    if !label.message.is_empty() {
        text.push(' ');
        text.push_str(&label.message);
    }
    if end > line_end {
        text.push_str(" (continues on following lines)");
    }
}

/// The 1-based line and display column of byte `offset` in `text`, as the
/// renderer places a caret: columns count Unicode scalar values, tabs expand to
/// four-column stops, other control characters take their escaped width and a
/// carriage return none. `None` when `offset` is past the text or inside a
/// character.
pub fn line_column(text: &str, offset: usize) -> Option<(usize, usize)> {
    if offset > text.len() || !text.is_char_boundary(offset) {
        return None;
    }
    let line_start = text[..offset].rfind('\n').map_or(0, |i| i + 1);
    let line = text[..offset].bytes().filter(|&b| b == b'\n').count() + 1;
    Some((line, expand(&text[line_start..offset]).1 + 1))
}

/// One line's displayed text and its display width.
fn expand(line: &str) -> (String, usize) {
    let mut expanded = String::new();
    let mut column = 0;
    for ch in line.chars() {
        if ch == '\t' {
            let width = 4 - column % 4;
            expanded.push_str(&" ".repeat(width));
            column += width;
        } else if ch == '\r' {
            // a CRLF terminator displays nothing
        } else if ch.is_control() {
            let escaped = ch.escape_default().to_string();
            column += escaped.len();
            expanded.push_str(&escaped);
        } else {
            expanded.push(ch);
            column += 1;
        }
    }
    (expanded, column)
}
