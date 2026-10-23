//! Cartan's source grammar and concrete syntax tree (CST).
//!
//! A CST retains grammar nodes, punctuation, field names, comments, and UTF-8
//! byte spans. This bootstrap uses the established Cartan grammar directly;
//! the same grammar generates editor tooling. It performs no name resolution,
//! type checking, graph construction, evaluation, or normalized AST lowering.
//!
//! Grammar acceptance is narrower evidence than acceptance by the language's
//! complete parser: legacy AST lowering also validates forms such as comparison
//! chains, range chains, and bodies requiring one result. Those additional form
//! checks are not implemented here. A malformed document still returns a tree
//! with error nodes and structured diagnostics for editor recovery.

pub mod diagnostic;
pub mod grammar;
mod lexer;

pub use diagnostic::{Comprehension, Diagnostic, DiagnosticKind, Field, Rule, Span};
pub use lexer::{KEYWORDS, LOCKED, NAMESPACE, PARAM, PERIODIC, WHEN};
pub use treegram::Node;

/// Source text, its concrete syntax tree, and recoverable grammar diagnostics.
#[derive(Clone, Debug)]
pub struct SyntaxTree {
    pub source: String,
    pub root: Node,
    pub diagnostics: Vec<Diagnostic>,
}

impl SyntaxTree {
    /// Whether the grammar reported diagnostics or produced an error node.
    pub fn has_errors(&self) -> bool {
        !self.diagnostics.is_empty() || self.root.has_error()
    }

    /// The source slice covered by a node, when its span belongs to this text.
    pub fn text(&self, node: &Node) -> Option<&str> {
        self.source.get(node.start..node.end)
    }
}

/// Parse a complete document against the shared source-file grammar.
pub fn parse_document(source: &str) -> SyntaxTree {
    parse(source, "source_file")
}

/// Parse one expression, preserving offsets in the supplied source text.
pub fn parse_expression(source: &str) -> SyntaxTree {
    parse(source, "_expression")
}

fn parse(source: &str, entry: &str) -> SyntaxTree {
    let parsed = treegram::parse(grammar::grammar(), source, entry);
    SyntaxTree {
        source: source.to_owned(),
        root: parsed.root,
        diagnostics: parsed.diags.into_iter().map(Diagnostic::from_raw).collect(),
    }
}
