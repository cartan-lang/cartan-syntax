//! The grammar's vocabulary: [`Rule`] is what a `grammar.js` rule body
//! is, combinator for combinator, and [`Climb`] is the expression rule
//! with its operator table stated outright. A [`Grammar`] holds the
//! named definitions and the header a `grammar.js` states — extras,
//! the word token, the conflicts.
//!
//! Two readers take a grammar, and each has a few constructs the other
//! ignores. **The printer alone reads** `PrecDynamic` and the
//! `conflicts` list, which are how tree-sitter's GLR parser settles an
//! ambiguity the ordered-choice interpreter settles by the order the
//! alternatives are written in. **The interpreter alone reads**
//! `NotAhead`, `NotAheadSoft`, `Expect`, `Refuse` and `Without`, which are lookahead,
//! diagnostics and an exclusion tree-sitter states through precedence
//! or through recovery. Each variant says which reader it belongs to.

use crate::pattern::Pat;

#[derive(Clone, Debug, PartialEq)]
pub enum Rule {
    /// Matches nothing and always succeeds — `blank()`.
    Blank,
    /// A literal token. One whose text matches the grammar's `word`
    /// pattern is a keyword: it matches a whole word only, and the
    /// word pattern refuses it.
    Str(String),
    /// A pattern token.
    Pat(Pat),
    /// A reference to a named rule — `$.name`.
    Sym(String),
    Seq(Vec<Rule>),
    /// Alternatives. The interpreter tries them in order and takes the
    /// first that matches; tree-sitter reads them as a set.
    Choice(Vec<Rule>),
    Repeat(Box<Rule>),
    Repeat1(Box<Rule>),
    Optional(Box<Rule>),
    /// `prec(n, …)` — tree-sitter's conflict resolution. The
    /// interpreter reads precedence only through a [`Climb`].
    Prec(i32, Box<Rule>),
    PrecLeft(i32, Box<Rule>),
    PrecRight(i32, Box<Rule>),
    /// `prec.dynamic(n, …)` — printer only.
    PrecDynamic(i32, Box<Rule>),
    /// `token(…)` — one token: the interpreter skips no extras inside it.
    Token(Box<Rule>),
    /// `token.immediate(…)` — a token with no extras before it.
    Immediate(Box<Rule>),
    /// `field("name", …)`: the nodes the rule produces take this name.
    Field(String, Box<Rule>),
    /// `alias(…, "name")` — the node the rule builds reads as `name`:
    /// how a token that includes its leading newlines still reads as
    /// the operator it is.
    Alias(Box<Rule>, String),
    /// Interpreter only: succeeds, consuming nothing, where the rule
    /// does not match here. The printer omits it.
    NotAhead(Box<Rule>),
    /// Interpreter only: negative lookahead whose diagnostic failures count
    /// as nonmatches rather than committing an error. Diagnostic state is
    /// restored after the probe. The printer omits it.
    NotAheadSoft(Box<Rule>),
    /// Interpreter only: succeeds, consuming nothing, where the rule
    /// matches here. The printer omits it.
    Ahead(Box<Rule>),
    /// Interpreter only: where the rule fails here, the parse of the
    /// enclosing item fails with the grammar's own diagnostic `Label`
    /// instead of the expected set, together with the fields the
    /// enclosing sequences have already matched and the text standing
    /// where the rule was looked for ([`crate::tree::Said::Stated`]).
    /// The printer prints the rule alone.
    Expect(Box<Rule>, Label),
    /// Interpreter only: where the rule matches here, the parse of the
    /// enclosing item fails with the grammar's diagnostic `Label`, the
    /// refused match's fields among those it reports. The printer
    /// omits it.
    Refuse(Box<Rule>, Label),
    /// Interpreter only: the expression rule named, parsed with these
    /// operator nodes excluded, in this expression and in the operands
    /// of its operators, up to the next ordinary rule. The printer
    /// prints the reference alone, to an exclusion copy that outranks
    /// the original unless the third field, `tied`, holds.
    Without(String, Vec<String>, bool),
    /// Interpreter only: where the first rule fails, the text from here
    /// to the next match of the second rule — the boundary, sought with
    /// the grammar's brackets kept balanced and its tokens skipped
    /// whole — becomes an `ERROR` node and the failure a diagnostic,
    /// and the parse goes on. The printer prints the first rule alone.
    Recover(Box<Rule>, Box<Rule>),
}

/// **A diagnostic a grammar states, by the number it gave it.** The
/// interpreter states no sentence: it reports the label back with the
/// facts it read ([`crate::tree::Said::Stated`]), and the grammar's
/// owner, who numbered its cases, says what each one means.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Label(pub u32);

/// How an operator groups with its neighbours of the same precedence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Assoc {
    None,
    Left,
    Right,
}

/// The shape of one operator of a [`Climb`].
#[derive(Clone, Debug, PartialEq)]
pub enum Form {
    /// `head operand` — `head` is any rule (it may hold whole
    /// expressions of its own, as `if c then a else` does), and the
    /// operand after it is parsed at this operator's precedence.
    Prefix { head: Rule, operand: String },
    /// `left op right`.
    Infix {
        left: String,
        op: Rule,
        right: String,
    },
    /// `target tail`. `targets`, when stated, names the node kinds the
    /// target may be; the printer prints the choice of them in the
    /// target's place.
    Postfix {
        target: String,
        tail: Rule,
        targets: Option<Vec<String>>,
    },
}

/// One operator of a [`Climb`]: the node it builds, its precedence and
/// associativity, and its shape. `dynamic` is a `prec.dynamic` the
/// printer adds to the tree-sitter rule, and nothing else reads.
#[derive(Clone, Debug, PartialEq)]
pub struct Op {
    pub node: String,
    /// Prose printed above the operator's alternative in `grammar.js`.
    pub doc: String,
    pub prec: i32,
    pub assoc: Assoc,
    pub form: Form,
    pub dynamic: i32,
}

/// The expression rule: the primary forms, and an operator table.
///
/// The interpreter parses it by precedence climbing — a primary or a
/// prefix operator first, then the infix and postfix operators whose
/// precedence the context allows, left to right — which is the reading
/// tree-sitter's shift/reduce resolution gives the same table once the
/// printer has written each operator as a `prec`-annotated rule. An
/// operator's right operand is parsed above its own precedence when it
/// groups left and at it when it groups right; a prefix operator's
/// operand is parsed at its own.
///
/// Several operators may build one node — every binary operator is a
/// `binary_expression` — and the printer gathers them into one rule.
/// A reference to an operator's node from outside the climb parses an
/// expression whose outermost node is that one.
#[derive(Clone, Debug, PartialEq)]
pub struct Climb {
    pub primary: Rule,
    pub ops: Vec<Op>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Body {
    Rule(Rule),
    Climb(Climb),
}

/// One named definition. A name opening with `_` is hidden: it builds
/// no node, and what it matches splices into its parent.
#[derive(Clone, Debug, PartialEq)]
pub struct Def {
    pub name: String,
    /// Prose printed above the rule in `grammar.js`.
    pub doc: String,
    pub body: Body,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Grammar {
    pub name: String,
    /// The comment at the head of `grammar.js`.
    pub doc: String,
    /// Skipped before every token that is not immediate.
    pub extras: Vec<Rule>,
    /// The rule whose pattern the keywords are extracted from.
    pub word: Option<String>,
    /// Printer only: tree-sitter's declared conflicts.
    pub conflicts: Vec<Vec<String>>,
    /// The bracket pairs error recovery keeps balanced when it skips to
    /// an item boundary.
    pub brackets: Vec<(String, String)>,
    pub rules: Vec<Def>,
}

impl Grammar {
    pub fn def(&self, name: &str) -> Option<&Def> {
        self.rules.iter().find(|d| d.name == name)
    }

    /// The keywords: every literal in the grammar whose text the word
    /// pattern matches whole.
    pub fn keywords(&self) -> Vec<String> {
        let Some(word) = self.word.as_ref().and_then(|w| self.def(w)) else {
            return Vec::new();
        };
        let Body::Rule(Rule::Pat(pat)) = &word.body else {
            return Vec::new();
        };
        let mut out: Vec<String> = Vec::new();
        for d in &self.rules {
            match &d.body {
                Body::Rule(r) => literals(r, &mut out),
                Body::Climb(c) => {
                    literals(&c.primary, &mut out);
                    for op in &c.ops {
                        match &op.form {
                            Form::Prefix { head, .. } => literals(head, &mut out),
                            Form::Infix { op, .. } => literals(op, &mut out),
                            Form::Postfix { tail, .. } => literals(tail, &mut out),
                        }
                    }
                }
            }
        }
        out.retain(|s| pat.longest(s, 0) == Some(s.len()));
        out.sort();
        out.dedup();
        out
    }
}

/// Every literal string a rule holds outside a `token(…)`.
fn literals(r: &Rule, out: &mut Vec<String>) {
    match r {
        Rule::Str(s) => out.push(s.clone()),
        Rule::Seq(rs) | Rule::Choice(rs) => rs.iter().for_each(|r| literals(r, out)),
        Rule::Repeat(r)
        | Rule::Repeat1(r)
        | Rule::Optional(r)
        | Rule::Prec(_, r)
        | Rule::PrecLeft(_, r)
        | Rule::PrecRight(_, r)
        | Rule::PrecDynamic(_, r)
        | Rule::Immediate(r)
        | Rule::Field(_, r)
        | Rule::Alias(r, _)
        | Rule::NotAhead(r)
        | Rule::NotAheadSoft(r)
        | Rule::Ahead(r)
        | Rule::Expect(r, _)
        | Rule::Refuse(r, _)
        | Rule::Recover(r, _) => literals(r, out),
        Rule::Token(_) | Rule::Blank | Rule::Pat(_) | Rule::Sym(_) | Rule::Without(..) => {}
    }
}

// ---- builders --------------------------------------------------------

impl From<&str> for Rule {
    fn from(s: &str) -> Rule {
        Rule::Str(s.to_string())
    }
}

impl From<Pat> for Rule {
    fn from(p: Pat) -> Rule {
        Rule::Pat(p)
    }
}

pub fn sym(name: &str) -> Rule {
    Rule::Sym(name.to_string())
}

pub fn seq(rs: Vec<Rule>) -> Rule {
    Rule::Seq(rs)
}

pub fn choice(rs: Vec<Rule>) -> Rule {
    Rule::Choice(rs)
}

pub fn repeat(r: impl Into<Rule>) -> Rule {
    Rule::Repeat(Box::new(r.into()))
}

pub fn repeat1(r: impl Into<Rule>) -> Rule {
    Rule::Repeat1(Box::new(r.into()))
}

pub fn optional(r: impl Into<Rule>) -> Rule {
    Rule::Optional(Box::new(r.into()))
}

pub fn prec(n: i32, r: impl Into<Rule>) -> Rule {
    Rule::Prec(n, Box::new(r.into()))
}

pub fn prec_left(n: i32, r: impl Into<Rule>) -> Rule {
    Rule::PrecLeft(n, Box::new(r.into()))
}

pub fn prec_right(n: i32, r: impl Into<Rule>) -> Rule {
    Rule::PrecRight(n, Box::new(r.into()))
}

pub fn prec_dynamic(n: i32, r: impl Into<Rule>) -> Rule {
    Rule::PrecDynamic(n, Box::new(r.into()))
}

pub fn token(r: impl Into<Rule>) -> Rule {
    Rule::Token(Box::new(r.into()))
}

pub fn immediate(r: impl Into<Rule>) -> Rule {
    Rule::Immediate(Box::new(r.into()))
}

pub fn field(name: &str, r: impl Into<Rule>) -> Rule {
    Rule::Field(name.to_string(), Box::new(r.into()))
}

pub fn alias(r: impl Into<Rule>, name: &str) -> Rule {
    Rule::Alias(Box::new(r.into()), name.to_string())
}

pub fn not_ahead(r: impl Into<Rule>) -> Rule {
    Rule::NotAhead(Box::new(r.into()))
}

pub fn not_ahead_soft(r: impl Into<Rule>) -> Rule {
    Rule::NotAheadSoft(Box::new(r.into()))
}

pub fn ahead(r: impl Into<Rule>) -> Rule {
    Rule::Ahead(Box::new(r.into()))
}

/// [`Rule::Expect`]: `r`, or the grammar's diagnostic `label` where it
/// fails.
pub fn expecting(r: impl Into<Rule>, label: Label) -> Rule {
    Rule::Expect(Box::new(r.into()), label)
}

/// [`Rule::Refuse`]: the grammar's diagnostic `label` where `r` matches.
pub fn refusing(r: impl Into<Rule>, label: Label) -> Rule {
    Rule::Refuse(Box::new(r.into()), label)
}

pub fn without(expr: &str, nodes: &[&str]) -> Rule {
    Rule::Without(
        expr.to_string(),
        nodes.iter().map(|s| s.to_string()).collect(),
        false,
    )
}

/// [`without`] at a position where the interpreter backtracks rather
/// than commits: its copy ties with the original instead of outranking
/// it, so tree-sitter keeps both readings and a later token decides, as
/// the interpreter's ordered choice does by retrying. A conflict between
/// the two rules is then stated in the grammar's `conflicts`.
pub fn without_tied(expr: &str, nodes: &[&str]) -> Rule {
    Rule::Without(
        expr.to_string(),
        nodes.iter().map(|s| s.to_string()).collect(),
        true,
    )
}

pub fn recover(r: impl Into<Rule>, boundary: impl Into<Rule>) -> Rule {
    Rule::Recover(Box::new(r.into()), Box::new(boundary.into()))
}

/// A named rule.
pub fn def(name: &str, doc: &str, body: Rule) -> Def {
    Def {
        name: name.to_string(),
        doc: doc.to_string(),
        body: Body::Rule(body),
    }
}

/// The expression rule.
pub fn climb(name: &str, doc: &str, primary: Rule, ops: Vec<Op>) -> Def {
    Def {
        name: name.to_string(),
        doc: doc.to_string(),
        body: Body::Climb(Climb { primary, ops }),
    }
}

pub fn prefix(node: &str, prec: i32, head: impl Into<Rule>, operand: &str) -> Op {
    Op {
        node: node.to_string(),
        doc: String::new(),
        prec,
        assoc: Assoc::None,
        form: Form::Prefix {
            head: head.into(),
            operand: operand.to_string(),
        },
        dynamic: 0,
    }
}

pub fn infix(
    node: &str,
    prec: i32,
    assoc: Assoc,
    left: &str,
    op: impl Into<Rule>,
    right: &str,
) -> Op {
    Op {
        node: node.to_string(),
        doc: String::new(),
        prec,
        assoc,
        form: Form::Infix {
            left: left.to_string(),
            op: op.into(),
            right: right.to_string(),
        },
        dynamic: 0,
    }
}

pub fn postfix(node: &str, prec: i32, target: &str, tail: impl Into<Rule>) -> Op {
    Op {
        node: node.to_string(),
        doc: String::new(),
        prec,
        assoc: Assoc::Left,
        form: Form::Postfix {
            target: target.to_string(),
            tail: tail.into(),
            targets: None,
        },
        dynamic: 0,
    }
}

impl Op {
    pub fn doc(mut self, doc: &str) -> Op {
        self.doc = doc.to_string();
        self
    }

    pub fn assoc(mut self, a: Assoc) -> Op {
        self.assoc = a;
        self
    }

    pub fn dynamic(mut self, n: i32) -> Op {
        self.dynamic = n;
        self
    }

    /// Restrict a postfix operator's target to these node kinds.
    pub fn targets(mut self, kinds: &[&str]) -> Op {
        if let Form::Postfix { targets, .. } = &mut self.form {
            *targets = Some(kinds.iter().map(|s| s.to_string()).collect());
        }
        self
    }
}

/// Whether a rule name is hidden — builds no node of its own.
pub fn is_hidden(name: &str) -> bool {
    name.starts_with('_')
}
