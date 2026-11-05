//! A tree-sitter query, read and held to the grammar it queries.
//!
//! An editor's `.scm` file is a consumer of the grammar's node roster:
//! it names node types, fields and child positions, and an editor
//! refuses to load a language whose query names one the compiled
//! grammar does not produce. The grammar is a [`Grammar`] here, so the
//! roster is computable — [`Grammar::roster`] states every visible node
//! type, the fields each node states and the node types each field and
//! child position holds — and [`Grammar::check_query`] holds a query
//! file to it, returning one [`QueryProblem`] per departure with the
//! line and the column the editor would report.
//!
//! # What the reader accepts
//!
//! The forms tree-sitter documents: a named node `(name …)`, an
//! anonymous node `"of"`, the wildcards `_` and `(_)`, a field
//! `field: pattern`, a negated field `!field`, a capture `@name` or
//! `@name.sub`, an alternation `[ … ]`, a grouping `( … )` of
//! siblings, the quantifiers `?`, `*` and `+`, the anchor `.`, the
//! predicates and directives `(#match? …)`, `(#eq? …)`, `(#set! …)`,
//! and comments opening with `;`. A predicate is skipped whole: the
//! strings inside one are arguments rather than node types.
//!
//! # Where the roster over-approximates
//!
//! The check must never refuse a query tree-sitter would accept, so
//! every place the exact set is expensive to compute is widened toward
//! acceptance, and each widening is stated here.
//!
//! - An alternation stands in a position where any one of its
//!   alternatives stands. Every alternative is still held to naming a
//!   node type and a field the grammar states.
//! - A negated field `!field` is held to naming a field the grammar
//!   states somewhere, rather than one the enclosing node states.
//! - The named extras — a comment — and `ERROR` and `MISSING` stand as
//!   children of every node.
//! - A pattern token produces no queryable anonymous node type, and
//!   neither does a `token(…)` whose body is more than one literal.
//!   Such a token reaches a query through the named rule that holds it.
//! - Nothing here reads the order of a pattern's children, its
//!   quantifiers or its anchors: a pattern whose children cannot stand
//!   in the order written passes.

use std::collections::{BTreeMap, BTreeSet};

use crate::rule::{is_hidden, Body, Form, Grammar, Rule};

/// One departure of a query from the grammar: where it stands, and one
/// sentence stating what the grammar requires and what the query wrote.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueryProblem {
    /// The 1-based line the pattern opens on.
    pub line: usize,
    /// The 1-based column, in characters.
    pub col: usize,
    pub msg: String,
}

/// What one visible named node holds: the fields it states with the
/// node types each takes, and the node types standing under it outside
/// any field. Both sets hold anonymous node types beside named ones,
/// spelled as the token's own text.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NodeType {
    pub fields: BTreeMap<String, BTreeSet<String>>,
    pub children: BTreeSet<String>,
}

/// The grammar's node roster: what `tree-sitter generate` writes as
/// `node-types.json`, computed from the grammar value.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Roster {
    /// Every visible named node type: a rule whose name is not hidden,
    /// and the node each operator of a [`crate::rule::Climb`] builds.
    pub named: BTreeSet<String>,
    /// Every anonymous node type: a literal a rule states outside a
    /// multi-part `token(…)`, and every `alias(…, "name")` name.
    pub anonymous: BTreeSet<String>,
    /// One entry per named node type.
    pub nodes: BTreeMap<String, NodeType>,
    /// Every field name the grammar states anywhere.
    pub fields: BTreeSet<String>,
    /// The named extras, which stand under every node.
    pub extras: BTreeSet<String>,
}

// ---- the roster ---------------------------------------------------------

impl Grammar {
    /// The grammar's node roster, walked off the rules the way
    /// [`crate::print()`] walks them to write `grammar.js`. The module
    /// doc states where the walk over-approximates.
    pub fn roster(&self) -> Roster {
        let mut r = Roster::default();
        for d in &self.rules {
            if !is_hidden(&d.name) {
                r.named.insert(d.name.clone());
            }
            if let Body::Climb(c) = &d.body {
                for op in &c.ops {
                    if op_prints(op.form.text()) {
                        r.named.insert(op.node.clone());
                    }
                }
            }
        }
        for d in &self.rules {
            if !is_hidden(&d.name) {
                let mut n = NodeType::default();
                self.walk_def(&d.name, &mut n, &mut Vec::new());
                r.nodes.insert(d.name.clone(), n);
            }
            let Body::Climb(c) = &d.body else { continue };
            let whole = self.produces(&Rule::Sym(d.name.clone()));
            for op in &c.ops {
                if !op_prints(op.form.text()) {
                    continue;
                }
                let n = r.nodes.entry(op.node.clone()).or_default();
                match &op.form {
                    Form::Prefix { head, operand } => {
                        self.walk(head, n, &mut Vec::new());
                        n.fields
                            .entry(operand.clone())
                            .or_default()
                            .extend(whole.iter().cloned());
                    }
                    Form::Infix {
                        left,
                        op: text,
                        right,
                    } => {
                        n.fields
                            .entry(left.clone())
                            .or_default()
                            .extend(whole.iter().cloned());
                        self.walk(text, n, &mut Vec::new());
                        n.fields
                            .entry(right.clone())
                            .or_default()
                            .extend(whole.iter().cloned());
                    }
                    Form::Postfix {
                        target,
                        tail,
                        targets,
                    } => {
                        let set: BTreeSet<String> = match targets {
                            Some(ks) => ks.iter().cloned().collect(),
                            None => whole.clone(),
                        };
                        n.fields.entry(target.clone()).or_default().extend(set);
                        self.walk(tail, n, &mut Vec::new());
                    }
                }
            }
        }
        for n in r.nodes.values() {
            for (f, types) in &n.fields {
                r.anonymous.extend(types.iter().cloned());
                r.fields.insert(f.clone());
            }
            r.anonymous.extend(n.children.iter().cloned());
        }
        r.anonymous.retain(|t| !r.named.contains(t));
        for e in &self.extras {
            if let Rule::Sym(n) = e {
                if !is_hidden(n) {
                    r.extras.insert(n.clone());
                }
            }
        }
        r
    }

    /// The node types a rule produces at its top level — what a field
    /// holding it can hold.
    fn produces(&self, r: &Rule) -> BTreeSet<String> {
        self.produces_in(r, &mut Vec::new())
    }

    fn produces_in(&self, r: &Rule, seen: &mut Vec<String>) -> BTreeSet<String> {
        let mut n = NodeType::default();
        self.walk(r, &mut n, seen);
        let mut out = n.children;
        for types in n.fields.into_values() {
            out.extend(types);
        }
        out
    }

    /// A named rule's body, spliced into the node being walked.
    fn walk_def(&self, name: &str, out: &mut NodeType, seen: &mut Vec<String>) {
        if seen.iter().any(|s| s == name) {
            return;
        }
        seen.push(name.to_string());
        if let Some(d) = self.def(name) {
            match &d.body {
                // a visible rule whose whole body is one literal is
                // that token: tree-sitter names the token for the rule
                // and states no child under it
                Body::Rule(r) if !is_hidden(name) && sole_literal(r).is_some() => {}
                Body::Rule(r) => self.walk(r, out, seen),
                Body::Climb(c) => {
                    self.walk(&c.primary, out, seen);
                    for op in &c.ops {
                        if op_prints(op.form.text()) {
                            out.children.insert(op.node.clone());
                        }
                    }
                }
            }
        }
        seen.pop();
    }

    /// One rule, collecting the fields it states and the nodes standing
    /// under it. A hidden rule splices, as tree-sitter splices it.
    fn walk(&self, r: &Rule, out: &mut NodeType, seen: &mut Vec<String>) {
        match r {
            Rule::Str(s) => {
                out.children.insert(s.clone());
            }
            Rule::Alias(_, name) => {
                out.children.insert(name.clone());
            }
            Rule::Sym(n) => match is_hidden(n) {
                true => self.walk_def(n, out, seen),
                false => {
                    out.children.insert(n.clone());
                }
            },
            Rule::Without(n, excluded, _) => {
                // the expression rule with those operator nodes left
                // out, which is the rule the printer writes as an
                // exclusion copy with every node aliased back
                let mut inner = NodeType::default();
                self.walk_def(n, &mut inner, seen);
                for node in excluded {
                    inner.children.remove(node);
                }
                out.children.extend(inner.children);
                for (f, types) in inner.fields {
                    out.fields.entry(f).or_default().extend(types);
                }
            }
            Rule::Field(name, inner) => {
                let types = self.produces_in(inner, seen);
                out.fields.entry(name.clone()).or_default().extend(types);
            }
            Rule::Token(inner) | Rule::Immediate(inner) => {
                // one token: a bare literal keeps its text, and anything
                // longer reaches a query through the rule that holds it
                if let Rule::Str(s) = &**inner {
                    out.children.insert(s.clone());
                }
            }
            Rule::Seq(rs) | Rule::Choice(rs) => rs.iter().for_each(|r| self.walk(r, out, seen)),
            Rule::Repeat(inner)
            | Rule::Repeat1(inner)
            | Rule::Optional(inner)
            | Rule::Prec(_, inner)
            | Rule::PrecLeft(_, inner)
            | Rule::PrecRight(_, inner)
            | Rule::PrecDynamic(_, inner)
            | Rule::Expect(inner, _)
            | Rule::Recover(inner, _) => self.walk(inner, out, seen),
            Rule::Blank
            | Rule::Pat(_)
            | Rule::NotAhead(_)
            | Rule::NotAheadSoft(_)
            | Rule::Ahead(_)
            | Rule::Refuse(..) => {}
        }
    }
}

impl Form {
    /// The text the operator matches beside its operands.
    fn text(&self) -> &Rule {
        match self {
            Form::Prefix { head, .. } => head,
            Form::Infix { op, .. } => op,
            Form::Postfix { tail, .. } => tail,
        }
    }
}

/// The one literal a rule is, where the rule is nothing else.
fn sole_literal(r: &Rule) -> Option<&str> {
    match r {
        Rule::Str(s) => Some(s),
        Rule::Token(inner) | Rule::Immediate(inner) => sole_literal(inner),
        _ => None,
    }
}

/// Whether an operator's text reaches tree-sitter at all: an operator
/// whose text is the interpreter's alone — a refusal — builds no node.
/// This mirrors what [`crate::print()`] writes.
fn op_prints(r: &Rule) -> bool {
    match r {
        Rule::Seq(rs) => rs.iter().any(prints),
        r => prints(r),
    }
}

/// Whether a rule prints as anything at all.
fn prints(r: &Rule) -> bool {
    match r {
        Rule::NotAhead(_) | Rule::NotAheadSoft(_) | Rule::Ahead(_) | Rule::Refuse(..) => false,
        Rule::Choice(rs) => rs.iter().any(prints),
        Rule::Repeat(r)
        | Rule::Repeat1(r)
        | Rule::Optional(r)
        | Rule::Prec(_, r)
        | Rule::PrecLeft(_, r)
        | Rule::PrecRight(_, r)
        | Rule::PrecDynamic(_, r)
        | Rule::Token(r)
        | Rule::Immediate(r)
        | Rule::Field(_, r)
        | Rule::Alias(r, _)
        | Rule::Expect(r, _)
        | Rule::Recover(r, _) => prints(r),
        _ => true,
    }
}

// ---- the query ----------------------------------------------------------

/// A position in the query file, as a byte offset; a problem states it
/// as a line and a column.
type Pos = usize;

#[derive(Clone, Debug)]
enum Pattern {
    /// `(name …)`
    Named {
        name: String,
        at: Pos,
        items: Vec<Item>,
    },
    /// `_`, `(_)` and `(_ …)`
    Wildcard { items: Vec<Item> },
    /// `( … )` — siblings, each standing where the group stands
    Group { items: Vec<Item> },
    /// `"of"`
    Anonymous { text: String, at: Pos },
    /// `[ … ]`
    Alternation { at: Pos, alts: Vec<Pattern> },
}

#[derive(Clone, Debug)]
enum Item {
    Field {
        name: String,
        at: Pos,
        value: Pattern,
    },
    Negated {
        name: String,
        at: Pos,
    },
    Child(Pattern),
}

struct Reader<'s> {
    src: &'s str,
    b: &'s [u8],
    i: usize,
}

/// A byte offset as a 1-based line and a 1-based column in characters.
fn line_col(src: &str, at: Pos) -> (usize, usize) {
    let head = &src[..at.min(src.len())];
    let line = head.matches('\n').count() + 1;
    let col = head.rsplit('\n').next().unwrap_or("").chars().count() + 1;
    (line, col)
}

impl<'s> Reader<'s> {
    fn new(src: &'s str) -> Reader<'s> {
        Reader {
            src,
            b: src.as_bytes(),
            i: 0,
        }
    }

    fn problem(&self, at: Pos, msg: String) -> QueryProblem {
        let (line, col) = line_col(self.src, at);
        QueryProblem { line, col, msg }
    }

    /// Whitespace and comments.
    fn skip(&mut self) {
        loop {
            while self.i < self.b.len() && self.b[self.i].is_ascii_whitespace() {
                self.i += 1;
            }
            if self.i < self.b.len() && self.b[self.i] == b';' {
                while self.i < self.b.len() && self.b[self.i] != b'\n' {
                    self.i += 1;
                }
                continue;
            }
            return;
        }
    }

    fn peek(&mut self) -> Option<u8> {
        self.skip();
        self.b.get(self.i).copied()
    }

    fn name(&mut self) -> String {
        let start = self.i;
        while self.i < self.b.len() && is_name_byte(self.b[self.i]) {
            self.i += 1;
        }
        self.src[start..self.i].to_string()
    }

    /// The whole file: every pattern at the top level.
    fn file(&mut self) -> Result<Vec<Pattern>, QueryProblem> {
        let mut out = Vec::new();
        while let Some(c) = self.peek() {
            if c == b')' || c == b']' {
                let at = self.i;
                return Err(self.problem(
                    at,
                    format!(
                        "a query closes each bracket it opens; this file closes a `{}` it did not open",
                        c as char
                    ),
                ));
            }
            match self.item()? {
                Some(Item::Child(p)) => out.push(p),
                Some(Item::Field { at, .. }) | Some(Item::Negated { at, .. }) => {
                    return Err(self.problem(
                        at,
                        "a field must stand inside a node pattern; this field stands at the file's top level".into(),
                    ))
                }
                None => {}
            }
        }
        Ok(out)
    }

    /// One item: a pattern, a field-prefixed pattern, a negated field,
    /// or a piece of syntax that states no structure — an anchor, a
    /// capture, a quantifier, a predicate.
    fn item(&mut self) -> Result<Option<Item>, QueryProblem> {
        let Some(c) = self.peek() else {
            return Ok(None);
        };
        let at = self.i;
        match c {
            b'.' | b'?' | b'*' | b'+' => {
                self.i += 1;
                Ok(None)
            }
            b'@' => {
                self.i += 1;
                self.name();
                Ok(None)
            }
            b'!' => {
                self.i += 1;
                self.skip();
                let name = self.name();
                match name.is_empty() {
                    true => Err(self.problem(
                        at,
                        "a negated field must name a field; this `!` names none".into(),
                    )),
                    false => Ok(Some(Item::Negated { name, at })),
                }
            }
            b'#' => Err(self.problem(
                at,
                "a predicate must stand in parens, as `(#eq? @a @b)`; this predicate stands bare"
                    .into(),
            )),
            _ if is_name_byte(c) && c != b'_' => {
                let name = self.name();
                if self.b.get(self.i) == Some(&b':') {
                    self.i += 1;
                    let value = self.pattern()?;
                    return Ok(Some(Item::Field { name, at, value }));
                }
                Err(self.problem(
                    at,
                    format!("a node type must stand in parens, as `({name})`; this query writes `{name}` bare"),
                ))
            }
            _ => Ok(Some(Item::Child(self.pattern()?))),
        }
    }

    fn pattern(&mut self) -> Result<Pattern, QueryProblem> {
        let Some(c) = self.peek() else {
            let at = self.i;
            return Err(self.problem(
                at,
                "a query must state a pattern here; this one reaches the end of the file".into(),
            ));
        };
        let at = self.i;
        match c {
            b'(' => self.parens(),
            b'[' => {
                self.i += 1;
                let mut alts = Vec::new();
                loop {
                    match self.peek() {
                        Some(b']') => {
                            self.i += 1;
                            return Ok(Pattern::Alternation { at, alts });
                        }
                        None => {
                            return Err(self.problem(
                                at,
                                "an alternation must close its `[`; this alternation reaches the end of the file".into(),
                            ))
                        }
                        _ => match self.item()? {
                            Some(Item::Child(p)) => alts.push(p),
                            Some(Item::Field { at, .. }) | Some(Item::Negated { at, .. }) => {
                                return Err(self.problem(
                                    at,
                                    "an alternation states patterns alone; this alternation states a field".into(),
                                ))
                            }
                            None => {}
                        },
                    }
                }
            }
            b'"' => {
                let text = self.string()?;
                Ok(Pattern::Anonymous { text, at })
            }
            b'_' => {
                self.name();
                Ok(Pattern::Wildcard { items: Vec::new() })
            }
            _ => Err(self.problem(
                at,
                format!(
                    "a pattern must open with `(`, `[`, `\"` or `_`; this pattern opens with `{}`",
                    c as char
                ),
            )),
        }
    }

    /// A parenthesized pattern: a named node, a wildcard, a predicate,
    /// or a grouping of siblings.
    fn parens(&mut self) -> Result<Pattern, QueryProblem> {
        let at = self.i;
        self.i += 1; // `(`
        let Some(c) = self.peek() else {
            return Err(self.problem(
                at,
                "a pattern must close its `(`; this pattern reaches the end of the file".into(),
            ));
        };
        if c == b'#' {
            self.predicate(at)?;
            return Ok(Pattern::Group { items: Vec::new() });
        }
        let head = match is_name_byte(c) {
            true => Some(self.name()),
            false => None,
        };
        let mut items = Vec::new();
        loop {
            match self.peek() {
                Some(b')') => {
                    self.i += 1;
                    break;
                }
                None => {
                    return Err(self.problem(
                        at,
                        "a pattern must close its `(`; this pattern reaches the end of the file"
                            .into(),
                    ))
                }
                _ => {
                    if let Some(item) = self.item()? {
                        items.push(item);
                    }
                }
            }
        }
        Ok(match head {
            Some(name) if name == "_" => Pattern::Wildcard { items },
            Some(name) => Pattern::Named { name, at, items },
            None => Pattern::Group { items },
        })
    }

    /// A predicate or directive, skipped whole: the strings inside one
    /// are arguments rather than node types.
    fn predicate(&mut self, at: Pos) -> Result<(), QueryProblem> {
        let mut depth = 1usize;
        while self.i < self.b.len() {
            match self.b[self.i] {
                b'"' => {
                    self.string()?;
                }
                b'(' => {
                    depth += 1;
                    self.i += 1;
                }
                b')' => {
                    depth -= 1;
                    self.i += 1;
                    if depth == 0 {
                        return Ok(());
                    }
                }
                b';' => {
                    while self.i < self.b.len() && self.b[self.i] != b'\n' {
                        self.i += 1;
                    }
                }
                _ => self.i += 1,
            }
        }
        Err(self.problem(
            at,
            "a predicate must close its `(`; this predicate reaches the end of the file".into(),
        ))
    }

    /// A double-quoted string, with `\` escaping the character after it.
    fn string(&mut self) -> Result<String, QueryProblem> {
        let at = self.i;
        self.i += 1;
        let mut out = String::new();
        while self.i < self.b.len() {
            match self.b[self.i] {
                b'"' => {
                    self.i += 1;
                    return Ok(out);
                }
                b'\\' if self.i + 1 < self.b.len() => {
                    let c = self.src[self.i + 1..].chars().next().unwrap();
                    out.push(match c {
                        'n' => '\n',
                        't' => '\t',
                        'r' => '\r',
                        c => c,
                    });
                    self.i += 1 + c.len_utf8();
                }
                _ => {
                    let c = self.src[self.i..].chars().next().unwrap();
                    out.push(c);
                    self.i += c.len_utf8();
                }
            }
        }
        Err(self.problem(
            at,
            "a string must close its `\"`; this string reaches the end of the file".into(),
        ))
    }
}

fn is_name_byte(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_' || c == b'-' || c == b'.'
}

// ---- the check ----------------------------------------------------------

/// The node types every pattern accepts, whatever the grammar states:
/// tree-sitter's own two.
const BUILTIN: [&str; 2] = ["ERROR", "MISSING"];

impl Grammar {
    /// Every departure of a tree-sitter query from this grammar, in the
    /// order the query states them. An empty result means the query
    /// names node types, fields and child positions the grammar
    /// produces, which is what an editor requires to load the language
    /// at all.
    ///
    /// The module doc states the query forms the reader accepts and the
    /// places the roster is widened toward acceptance.
    pub fn check_query(&self, src: &str) -> Vec<QueryProblem> {
        let mut reader = Reader::new(src);
        let patterns = match reader.file() {
            Ok(p) => p,
            Err(e) => return vec![e],
        };
        let mut check = Check {
            src,
            roster: self.roster(),
            out: Vec::new(),
            children: BTreeMap::new(),
        };
        for p in &patterns {
            check.pattern(p, None);
        }
        check.out
    }
}

struct Check<'s> {
    src: &'s str,
    roster: Roster,
    out: Vec<QueryProblem>,
    /// Per node type, the node types a child pattern with no field may
    /// stand for: the children outside a field, every field's own
    /// types, the named extras, and tree-sitter's two.
    children: BTreeMap<String, BTreeSet<String>>,
}

impl Check<'_> {
    fn say(&mut self, at: Pos, msg: String) {
        let (line, col) = line_col(self.src, at);
        self.out.push(QueryProblem { line, col, msg });
    }

    /// The node types that may stand under `node` outside a field.
    fn child_set(&mut self, node: &str) -> BTreeSet<String> {
        if let Some(s) = self.children.get(node) {
            return s.clone();
        }
        let mut set: BTreeSet<String> = self.roster.extras.clone();
        set.extend(BUILTIN.iter().map(|s| s.to_string()));
        if let Some(n) = self.roster.nodes.get(node) {
            set.extend(n.children.iter().cloned());
            for types in n.fields.values() {
                set.extend(types.iter().cloned());
            }
        }
        self.children.insert(node.to_string(), set.clone());
        set
    }

    /// One pattern, against the node types its position holds. `ctx` is
    /// `None` where the position holds anything — the file's top level,
    /// or a position under a wildcard.
    fn pattern(&mut self, p: &Pattern, ctx: Option<&BTreeSet<String>>) {
        match p {
            Pattern::Named { name, at, items } => {
                let known = self.roster.named.contains(name) || BUILTIN.contains(&name.as_str());
                if !known {
                    self.say(
                        *at,
                        format!("a query pattern must name a node type the grammar states; `{name}` names none"),
                    );
                } else if let Some(set) = ctx {
                    if !set.contains(name) {
                        self.say(
                            *at,
                            format!(
                                "a query pattern must name a node type its position holds ({}); this pattern names `{name}`",
                                listing(set)
                            ),
                        );
                    }
                }
                let inner = match known {
                    true => Some(self.child_set(name)),
                    false => None,
                };
                for item in items {
                    self.item(item, name, inner.as_ref(), known);
                }
            }
            Pattern::Wildcard { items, .. } => {
                for item in items {
                    self.item(item, "_", None, false);
                }
            }
            Pattern::Group { items, .. } => {
                for item in items {
                    match item {
                        Item::Child(p) => self.pattern(p, ctx),
                        Item::Field { name, at, .. } => self.say(
                            *at,
                            format!("a field must stand inside a node pattern; `{name}:` stands in a group, which names no node"),
                        ),
                        Item::Negated { name, at } => self.say(
                            *at,
                            format!("a negated field must stand inside a node pattern; `!{name}` stands in a group, which names no node"),
                        ),
                    }
                }
            }
            Pattern::Anonymous { text, at } => {
                let known = self.roster.anonymous.contains(text);
                if !known {
                    self.say(
                        *at,
                        format!("a query pattern must name a token the grammar's rules produce; `\"{text}\"` names none"),
                    );
                } else if let Some(set) = ctx {
                    if !set.contains(text) {
                        self.say(
                            *at,
                            format!(
                                "a query pattern must name a node type its position holds ({}); this pattern names `\"{text}\"`",
                                listing(set)
                            ),
                        );
                    }
                }
            }
            Pattern::Alternation { at, alts } => {
                for a in alts {
                    self.pattern(a, None);
                }
                let fits = |set: &BTreeSet<String>| {
                    alts.iter().any(|a| match a {
                        Pattern::Named { name, .. } => set.contains(name),
                        Pattern::Anonymous { text, .. } => set.contains(text),
                        _ => true,
                    })
                };
                if let Some(set) = ctx {
                    if !alts.is_empty() && !fits(set) {
                        self.say(
                            *at,
                            format!(
                                "an alternation must state one node type its position holds ({}); this alternation states none of them",
                                listing(set)
                            ),
                        );
                    }
                }
            }
        }
    }

    /// One item of a node pattern. `node` names the enclosing pattern
    /// and `known` states whether the grammar has it; where it does
    /// not, the item is read for its own spelling alone.
    fn item(&mut self, item: &Item, node: &str, children: Option<&BTreeSet<String>>, known: bool) {
        match item {
            Item::Child(p) => {
                let ctx = children.cloned();
                self.pattern(p, ctx.as_ref());
            }
            Item::Field { name, at, value } => {
                let types = match known {
                    true => self
                        .roster
                        .nodes
                        .get(node)
                        .and_then(|n| n.fields.get(name))
                        .cloned(),
                    false => None,
                };
                if known && types.is_none() {
                    let states: Vec<String> = self
                        .roster
                        .nodes
                        .get(node)
                        .map(|n| n.fields.keys().map(|f| format!("`{f}`")).collect())
                        .unwrap_or_default();
                    let msg = match states.is_empty() {
                        true => format!(
                            "a `{node}` pattern names no field; `{node}` states none, and this pattern names `{name}`"
                        ),
                        false => format!(
                            "a `{node}` pattern must name a field `{node}` states ({}); this pattern names `{name}`",
                            states.join(", ")
                        ),
                    };
                    self.say(*at, msg);
                }
                self.pattern(value, types.as_ref());
            }
            Item::Negated { name, at } => {
                if !self.roster.fields.contains(name) {
                    self.say(
                        *at,
                        format!("a negated field must name a field the grammar states; `{name}` names none"),
                    );
                }
            }
        }
    }
}

/// A set of node types as a diagnostic states it: the whole set where
/// it is short, and the first few with a count where it is long.
fn listing(set: &BTreeSet<String>) -> String {
    let names: Vec<String> = set.iter().map(|t| format!("`{t}`")).collect();
    match names.len() {
        0 => "no node type at all".to_string(),
        1..=6 => names.join(", "),
        n => format!("{} and {} more", names[..6].join(", "), n - 6),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rule::{def, field, optional, repeat, seq, sym};

    /// A grammar small enough to check by hand: a document of locals and
    /// calls, a local binding a `pattern` to a `value`, a call of one name.
    fn grammar() -> Grammar {
        Grammar {
            name: "tiny".into(),
            rules: vec![
                def("document", "", repeat(sym("_item"))),
                def(
                    "_item",
                    "",
                    crate::rule::choice(vec![sym("local"), sym("call")]),
                ),
                def(
                    "local",
                    "",
                    seq(vec![
                        field("pattern", sym("identifier")),
                        ":=".into(),
                        field(
                            "value",
                            crate::rule::choice(vec![sym("identifier"), sym("call")]),
                        ),
                    ]),
                ),
                def(
                    "call",
                    "",
                    seq(vec![
                        field("function", sym("identifier")),
                        "(".into(),
                        optional(sym("identifier")),
                        ")".into(),
                    ]),
                ),
                def("identifier", "", "x".into()),
            ],
            ..Grammar::default()
        }
    }

    fn problems(q: &str) -> Vec<QueryProblem> {
        grammar().check_query(q)
    }

    /// The roster states every visible node, each node's fields with the
    /// types they hold, the children outside a field, and the tokens.
    #[test]
    fn the_roster_is_the_grammar_read_as_node_types() {
        let r = grammar().roster();
        let named: Vec<&str> = r.named.iter().map(String::as_str).collect();
        assert_eq!(named, ["call", "document", "identifier", "local"]);
        let local = &r.nodes["local"];
        assert_eq!(
            local.fields["pattern"].iter().collect::<Vec<_>>(),
            ["identifier"]
        );
        assert_eq!(
            local.fields["value"].iter().collect::<Vec<_>>(),
            ["call", "identifier"]
        );
        assert_eq!(local.children.iter().collect::<Vec<_>>(), [":="]);
        let call = &r.nodes["call"];
        assert_eq!(
            call.children.iter().collect::<Vec<_>>(),
            ["(", ")", "identifier"]
        );
        assert!(r.anonymous.contains(":=") && r.anonymous.contains("("));
        assert!(
            r.nodes["identifier"].children.is_empty(),
            "a one-literal rule is a token"
        );
    }

    /// The first incident: a capture naming a rule the grammar no longer
    /// states fails at the pattern's own line and column.
    #[test]
    fn a_retired_node_is_one_problem_at_its_line() {
        let q = "; painted as a property\n(local pattern: (identifier) @variable)\n(projection_expression name: (identifier) @property)\n";
        let ps = problems(q);
        assert_eq!(ps.len(), 1, "{ps:?}");
        assert_eq!((ps[0].line, ps[0].col), (3, 1));
        assert!(
            ps[0].msg.contains("`projection_expression` names none"),
            "{}",
            ps[0].msg
        );
    }

    /// The second incident: a field the node no longer states, which
    /// tree-sitter reports as an impossible pattern.
    #[test]
    fn a_moved_field_is_one_problem_naming_the_fields_the_node_states() {
        let ps = problems("(local name: (identifier) @variable)\n");
        assert_eq!(ps.len(), 1, "{ps:?}");
        assert_eq!((ps[0].line, ps[0].col), (1, 8));
        assert!(
            ps[0].msg.contains("`pattern`, `value`") && ps[0].msg.contains("names `name`"),
            "{}",
            ps[0].msg
        );
    }

    /// A child pattern must name a type its position holds.
    #[test]
    fn a_child_the_position_cannot_hold_is_a_problem() {
        let ps = problems("(local value: (document))\n(call (local))\n");
        assert_eq!(ps.len(), 2, "{ps:?}");
        assert!(
            ps[0].msg.contains("`call`, `identifier`") && ps[0].msg.contains("names `document`"),
            "{}",
            ps[0].msg
        );
        assert!(ps[1].msg.contains("names `local`"), "{}", ps[1].msg);
    }

    /// Every form the reader accepts, in one query that states nothing
    /// the grammar lacks.
    #[test]
    fn every_query_form_passes_against_the_grammar_it_fits() {
        let q = r#"
; a comment
(local pattern: (identifier) @variable ":=" value: [(identifier) (call)] @value)
(local !value)
(call function: (identifier) @function . (identifier)? @argument)
(call (_) @any)
(_ (identifier))
_ @anything
((identifier) @kw (#match? @kw "^(x)$") (#set! priority 1))
((local) (call))*
[(local) (call)] @item
(document (local)+ (ERROR) (MISSING))
"#;
        let ps = problems(q);
        assert!(ps.is_empty(), "{ps:#?}");
    }

    /// Malformed syntax is reported where it stands rather than read past.
    #[test]
    fn an_unclosed_pattern_is_one_problem() {
        let ps = problems("(local pattern: (identifier)\n");
        assert_eq!(ps.len(), 1, "{ps:?}");
        assert!(ps[0].msg.contains("must close its `(`"), "{}", ps[0].msg);
    }
}
