//! The concrete tree a parse returns: every node a rule name over a
//! byte range, anonymous tokens included, so the text of any node is a
//! slice of the source and nothing is copied.

/// One node of the tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Node {
    /// The rule that built the node, or the token's own text where the
    /// node is anonymous, or `ERROR`.
    pub kind: String,
    /// Whether the node stands for a named rule (or an error) rather
    /// than a literal token.
    pub named: bool,
    /// The field name the parent's rule gave this node, if any.
    pub field: Option<String>,
    /// Whether the node is a named extra — a comment — standing where
    /// it was skipped rather than where a rule asked for it.
    pub extra: bool,
    pub start: usize,
    pub end: usize,
    pub children: Vec<Node>,
}

impl Node {
    /// The node's own text.
    pub fn text<'s>(&self, src: &'s str) -> &'s str {
        &src[self.start..self.end]
    }

    /// The first child with this field name.
    pub fn child(&self, field: &str) -> Option<&Node> {
        self.children
            .iter()
            .find(|c| c.field.as_deref() == Some(field))
    }

    /// Every child with this field name, in order.
    pub fn children_named<'a>(&'a self, field: &'a str) -> impl Iterator<Item = &'a Node> + 'a {
        self.children
            .iter()
            .filter(move |c| c.field.as_deref() == Some(field))
    }

    /// The named children, in order.
    pub fn named_children(&self) -> impl Iterator<Item = &Node> {
        self.children.iter().filter(|c| c.named)
    }

    /// Whether an `ERROR` node stands anywhere under this one.
    pub fn has_error(&self) -> bool {
        self.kind == "ERROR" || self.children.iter().any(Node::has_error)
    }

    /// The tree as tree-sitter's `parse` command prints one: named
    /// nodes alone, each field-prefixed, on one line —
    /// `(binding name: (identifier) value: (number))`.
    pub fn sexp(&self) -> String {
        let mut s = String::new();
        self.write_sexp(&mut s);
        s
    }

    fn write_sexp(&self, out: &mut String) {
        if let Some(f) = &self.field {
            out.push_str(f);
            out.push_str(": ");
        }
        out.push('(');
        out.push_str(&self.kind);
        for c in self.named_children() {
            out.push(' ');
            c.write_sexp(out);
        }
        out.push(')');
    }
}

/// **One diagnostic, as facts**: where it stands, what stood there,
/// and why the parse stopped. The interpreter states no sentence; its
/// reader words these facts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diag {
    pub pos: usize,
    /// Where the text the diagnostic is about ends: the refused text's
    /// end for a [`crate::rule::Rule::Refuse`], and the end of
    /// [`Diag::found`] otherwise.
    pub end: usize,
    /// The text standing at `pos` as a diagnostic quotes it — the next
    /// run of word characters, or the next character — and `None` at
    /// the end of the text.
    pub found: Option<String>,
    /// **The parse asked for a token and found the end of the text.**
    /// An open bracket, a trailing operator and a `=` with nothing
    /// after it each leave the parse waiting on text that was never
    /// written, which a reader typing a line at a time is about to
    /// write on the next one; a diagnostic about a token the text does
    /// hold states `false` here. A prompt reads this to decide whether
    /// a typed line ends the item.
    pub incomplete: bool,
    pub said: Said,
}

/// **Why a parse stopped.**
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Said {
    /// No rule said why: the tokens looked for at the furthest position
    /// reached, in the order first looked for, each as a reader names
    /// it (`` `)` ``, `newline`) — none where the text held more than
    /// the grammar reads.
    Expected(Vec<String>),
    /// A rule's own diagnostic: the label the grammar gave it, whether
    /// it refused text it matched (else it expected text it did not
    /// find), and the fields of the enclosing sequences matched by
    /// then, nearest first, each name once.
    Stated {
        label: crate::rule::Label,
        refused: bool,
        fields: Vec<Field>,
    },
}

/// One field a [`Said::Stated`] diagnostic read: its name and the byte
/// range of the node that matched it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Field {
    pub name: String,
    pub start: usize,
    pub end: usize,
}

impl Diag {
    /// The text of the field `name` a stated diagnostic read, in the
    /// source it was parsed from.
    pub fn field<'s>(&self, src: &'s str, name: &str) -> Option<&'s str> {
        match &self.said {
            Said::Stated { fields, .. } => fields
                .iter()
                .find(|f| f.name == name)
                .map(|f| &src[f.start..f.end]),
            Said::Expected(_) => None,
        }
    }
}

/// The 1-based line a byte position stands on.
pub fn line_of(src: &str, pos: usize) -> usize {
    src[..pos.min(src.len())].matches('\n').count() + 1
}
