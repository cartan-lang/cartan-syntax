//! Choosing one top-level item of a source document, as inspection commands
//! select it: `BINDING`, a name a top-level binding or function writes, or
//! `UPDATE`, a whole authored update block by source order.
//!
//! Selection reads written syntax only. It resolves no imports, scopes or
//! types and plans no updates: an update block is the whole authored block,
//! its guard and every write, numbered among the root document's top-level
//! update blocks from zero. Only `while` blocks normalize as update blocks,
//! and namespace bodies and imported documents are never searched.

use crate::ast::{Document, Item, ItemKind, Pattern, PatternKind};
use crate::Span;
use std::fmt;
use std::str::FromStr;

/// One requested item. The default selects the binding `main`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Selector {
    Binding(String),
    /// The zero-based index of an authored update block.
    Update(usize),
}

/// A selector whose spelling names neither a binding nor an update block.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SelectorError {
    Malformed { written: String },
}

/// Why a well-formed selector chose no single item.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SelectionError {
    /// No top-level binding or function writes `name`. `available` lists the
    /// names that are written, in source order.
    UnknownBinding {
        name: String,
        available: Vec<String>,
    },
    /// The document has `count` update blocks, none at `index`.
    UnknownUpdate { index: usize, count: usize },
    /// Several top-level items write `name`; their spans, in source order.
    /// Syntax keeps duplicates; which one a program means is not its question.
    AmbiguousBinding { name: String, candidates: Vec<Span> },
}

impl Default for Selector {
    fn default() -> Self {
        Selector::Binding("main".into())
    }
}

impl Selector {
    /// `update` is `update:0`; `update:N` takes decimal digits only; any
    /// other text without whitespace, `:` or `=` is a binding name. `update`
    /// is a selector spelling here, not a language keyword.
    pub fn parse(text: &str) -> Result<Self, SelectorError> {
        let malformed = || SelectorError::Malformed {
            written: text.into(),
        };
        if text == "update" {
            return Ok(Selector::Update(0));
        }
        if let Some(index) = text.strip_prefix("update:") {
            if index.is_empty() || !index.bytes().all(|b| b.is_ascii_digit()) {
                return Err(malformed());
            }
            return index.parse().map(Selector::Update).map_err(|_| malformed());
        }
        if text.is_empty() || text.contains(|c: char| c.is_whitespace() || c == ':' || c == '=') {
            return Err(malformed());
        }
        Ok(Selector::Binding(text.into()))
    }
}

impl FromStr for Selector {
    type Err = SelectorError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Selector::parse(text)
    }
}

impl fmt::Display for Selector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Selector::Binding(name) => f.write_str(name),
            Selector::Update(index) => write!(f, "update:{index}"),
        }
    }
}

/// The root document's authored update blocks, in source order.
pub fn updates(document: &Document) -> impl Iterator<Item = &Item> {
    document
        .items
        .iter()
        .filter(|item| matches!(item.kind, ItemKind::While(_)))
}

/// The one top-level item `selector` names.
pub fn select<'d>(document: &'d Document, selector: &Selector) -> Result<&'d Item, SelectionError> {
    match selector {
        Selector::Update(index) => {
            updates(document)
                .nth(*index)
                .ok_or_else(|| SelectionError::UnknownUpdate {
                    index: *index,
                    count: updates(document).count(),
                })
        }
        Selector::Binding(name) => {
            let candidates: Vec<&Item> = document
                .items
                .iter()
                .filter(|item| written_names(item).iter().any(|written| written == name))
                .collect();
            match candidates.as_slice() {
                [item] => Ok(item),
                [] => Err(SelectionError::UnknownBinding {
                    name: name.clone(),
                    available: document.items.iter().flat_map(written_names).collect(),
                }),
                _ => Err(SelectionError::AmbiguousBinding {
                    name: name.clone(),
                    candidates: candidates.iter().map(|item| item.span).collect(),
                }),
            }
        }
    }
}

/// The names a top-level item writes as bindings: a binding's pattern names
/// and a function's name. Imports, type declarations, update blocks and
/// requirements write none.
fn written_names(item: &Item) -> Vec<String> {
    let mut names = Vec::new();
    match &item.kind {
        ItemKind::Binding(binding) => pattern_names(&binding.pattern, &mut names),
        ItemKind::Function(function) => names.push(function.name.text.clone()),
        _ => {}
    }
    names
}

fn pattern_names(pattern: &Pattern, names: &mut Vec<String>) {
    match &pattern.kind {
        PatternKind::Name(name) => names.push(name.text.clone()),
        PatternKind::Wildcard => {}
        PatternKind::List(items) | PatternKind::Site(items) => {
            items.iter().for_each(|item| pattern_names(item, names))
        }
        PatternKind::Constructor { items, .. } => {
            items.iter().for_each(|item| pattern_names(item, names))
        }
        PatternKind::Components(components) => components
            .iter()
            .for_each(|component| pattern_names(&component.pattern, names)),
        PatternKind::Entries(entries) => {
            for entry in entries {
                match &entry.pattern {
                    Some(pattern) => pattern_names(pattern, names),
                    None => names.push(entry.key.text.clone()),
                }
            }
        }
    }
}
