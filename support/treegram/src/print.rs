//! The grammar as a `grammar.js`, the text `tree-sitter generate`
//! reads, and as the `grammar.json` that text evaluates to, which the
//! generator also reads directly. Every combinator prints as its
//! tree-sitter spelling, a [`Climb`] prints as one `prec`-annotated
//! rule per operator node beside the expression rule's `choice`, and
//! the interpreter's own constructs print as the rule they wrap or as
//! nothing. Both forms come from one presentation of the grammar,
//! so they state the same rules.
//!
//! Where the grammar names a newline token or states lookaheads, the
//! presentation reads them through an external scanner, which
//! [`crate::scanner()`] prints from the same value: the newline token
//! and every lookahead are `externals`, a newline is otherwise an
//! extra, and the newlines a rule allows with `repeat` of the token
//! print as nothing.

use crate::json::Json;
use crate::rule::{Assoc, Body, Climb, Def, Exclusion, Form, Grammar, Op, Rule};

/// The `grammar.js` text of a grammar.
pub fn print(g: &Grammar) -> String {
    let p = Presentation::of(g);
    let mut out = String::new();
    if !g.doc.is_empty() {
        out.push_str("/**\n");
        for line in g.doc.lines() {
            if line.is_empty() {
                out.push_str(" *\n");
            } else {
                out.push_str(" * ");
                out.push_str(line);
                out.push('\n');
            }
        }
        out.push_str(" */\n\n");
    }
    out.push_str("module.exports = grammar({\n");
    out.push_str(&format!("  name: {},\n\n", js_string(&g.name)));
    out.push_str("  extras: ($) => ");
    out.push_str(&render(&Js::list(p.extras.clone()), 2, 80));
    out.push_str(",\n\n");
    if !p.externals.is_empty() {
        let names: Vec<Js> = p.externals.iter().map(|n| Js::sym(n)).collect();
        out.push_str("  externals: ($) => ");
        out.push_str(&render(&Js::list(names), 2, 80));
        out.push_str(",\n\n");
    }
    if let Some(w) = &g.word {
        out.push_str(&format!("  word: ($) => $.{w},\n\n"));
    }
    if !g.conflicts.is_empty() {
        out.push_str("  conflicts: ($) => [\n");
        for c in &g.conflicts {
            let names: Vec<String> = c.iter().map(|n| format!("$.{n}")).collect();
            out.push_str(&format!("    [{}],\n", names.join(", ")));
        }
        out.push_str("  ],\n\n");
    }
    out.push_str("  rules: {\n");
    for (i, (name, doc, rule)) in p.rules.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        for line in doc.lines() {
            out.push_str("    //");
            if !line.is_empty() {
                out.push(' ');
                out.push_str(line);
            }
            out.push('\n');
        }
        let arg = if mentions_symbol(rule) { "($)" } else { "(_)" };
        let flat = format!("    {name}: {arg} => {},", render(rule, 4, usize::MAX));
        if flat.len() <= 80 {
            out.push_str(&flat);
            out.push('\n');
        } else {
            out.push_str(&format!("    {name}: {arg} =>\n      "));
            out.push_str(&render(rule, 6, 80));
            out.push_str(",\n");
        }
    }
    out.push_str("  },\n});\n");
    out
}

/// The `grammar.json` text of a grammar: what the `grammar.js`
/// [`print()`] writes evaluates to under tree-sitter's DSL, and what
/// `tree-sitter generate` reads without running JavaScript.
pub fn print_json(g: &Grammar) -> String {
    let p = Presentation::of(g);
    let names = |list: &[String]| Json::Arr(list.iter().map(|n| Json::str(n)).collect());
    let symbols = |list: &[String]| Json::Arr(list.iter().map(|n| Js::sym(n).to_json()).collect());
    let mut fields = vec![
        (
            "$schema",
            Json::str(
                "https://tree-sitter.github.io/tree-sitter/assets/schemas/grammar.schema.json",
            ),
        ),
        ("name", Json::str(&g.name)),
    ];
    if let Some(w) = &g.word {
        fields.push(("word", Json::str(w)));
    }
    fields.extend([
        (
            "rules",
            Json::Obj(
                p.rules
                    .iter()
                    .map(|(name, _, rule)| (name.clone(), rule.to_json()))
                    .collect(),
            ),
        ),
        (
            "extras",
            Json::Arr(p.extras.iter().map(Js::to_json).collect()),
        ),
        (
            "conflicts",
            Json::Arr(g.conflicts.iter().map(|c| names(c)).collect()),
        ),
        ("precedences", Json::Arr(Vec::new())),
        ("externals", symbols(&p.externals)),
        ("inline", Json::Arr(Vec::new())),
        ("supertypes", Json::Arr(Vec::new())),
        ("reserved", Json::Obj(Vec::new())),
    ]);
    let doc = Json::Obj(
        fields
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect(),
    );
    let mut out = doc.pretty();
    out.push('\n');
    out
}

/// The external token the scanner returns in tree-sitter's error
/// recovery, where every external token is valid at once: it stands in
/// no rule, and the scanner returns nothing while it is valid.
pub(crate) const ERROR_SENTINEL: &str = "_error_sentinel";

/// The external tokens of a grammar, in the order the scanner numbers
/// them: the newline token, each lookahead, and the error sentinel. None
/// where the grammar states neither a newline token nor a lookahead.
pub(crate) fn externals(g: &Grammar) -> Vec<String> {
    let mut out: Vec<String> = g.newline.iter().cloned().collect();
    out.extend(g.lookaheads.iter().map(|l| l.name.clone()));
    if !out.is_empty() {
        out.push(ERROR_SENTINEL.to_string());
    }
    out
}

/// What tree-sitter reads of a grammar: its extras, its external
/// tokens, and every rule in order — name, the prose above it in
/// `grammar.js`, and its body — with the exclusion copies the grammar
/// asks for beside the climbs they copy.
struct Presentation<'g> {
    g: &'g Grammar,
    copies: Vec<Copy>,
    extras: Vec<Js>,
    externals: Vec<String>,
    rules: Vec<(String, String, Js)>,
}

impl<'g> Presentation<'g> {
    fn of(g: &'g Grammar) -> Presentation<'g> {
        let mut p = Presentation {
            g,
            copies: copies(g),
            extras: Vec::new(),
            externals: externals(g),
            rules: Vec::new(),
        };
        p.extras = g.extras.iter().filter_map(|r| p.js(r)).collect();
        if let Some(newline) = &g.newline {
            // the newline the scanner does not take is whitespace
            let def = g
                .def(newline)
                .unwrap_or_else(|| panic!("the newline token `{newline}` is no rule"));
            let Body::Rule(r) = &def.body else {
                panic!("the newline token `{newline}` is an expression rule");
            };
            p.extras.extend(p.js(r));
        }
        p.rules = g
            .rules
            .iter()
            .filter(|d| g.newline.as_ref() != Some(&d.name))
            .flat_map(|d| p.printed_rules(d))
            .collect();
        p
    }

    /// The rules a definition prints as: one for an ordinary rule, and
    /// for a climb the expression rule followed by one rule per
    /// operator node, then the same again for each exclusion copy of it.
    fn printed_rules(&self, d: &Def) -> Vec<(String, String, Js)> {
        match &d.body {
            Body::Rule(r) => vec![(
                d.name.clone(),
                d.doc.clone(),
                self.js(r).unwrap_or_else(blank),
            )],
            Body::Climb(c) => {
                let mut out = self.climb_rules(&d.name, &d.doc, c, None);
                for copy in self.copies.iter().filter(|c| c.ex.expr == d.name) {
                    let excluded: Vec<String> =
                        copy.ex.excluded.iter().map(|n| format!("`{n}`")).collect();
                    let doc = format!(
                        "`{}` without {}:\n\
                         the rule a reference through `without` in the grammar's source\n\
                         reads — a loop header's expression, whose brace and colon belong\n\
                         to the header. Every node is aliased to its name in `{}`, so\n\
                         the tree is the same tree; the precedences are lifted to\n\
                         `2p + 1` so the header's reading wins wherever the two could\n\
                         take the same text.",
                        d.name,
                        excluded.join(" and "),
                        d.name
                    );
                    out.extend(self.climb_rules(&d.name, &doc, c, Some(copy)));
                }
                out
            }
        }
    }

    fn climb_rules(
        &self,
        name: &str,
        doc: &str,
        c: &Climb,
        copy: Option<&Copy>,
    ) -> Vec<(String, String, Js)> {
        let mut alts: Vec<Js> = match &c.primary {
            Rule::Choice(rs) => rs.iter().filter_map(|r| self.js(r)).collect(),
            r => self.js(r).into_iter().collect(),
        };
        let excluded = |node: &str| copy.is_some_and(|c| c.ex.excluded.iter().any(|n| n == node));
        let mut nodes: Vec<&str> = Vec::new();
        for op in &c.ops {
            // an operator whose text is the interpreter's alone — a
            // refusal — has nothing for tree-sitter to read
            if self.seq_items(op_text(op)).is_empty() || excluded(&op.node) {
                continue;
            }
            if !nodes.contains(&op.node.as_str()) {
                nodes.push(&op.node);
                alts.push(node_ref(&op.node, copy));
            }
        }
        let own = |n: &str| match copy {
            Some(c) => format!("{n}_{}", c.ex.label),
            None => n.to_string(),
        };
        // a copy outranks the original wherever the two could both take a
        // word — a header's expression followed by a bare body — since the
        // interpreter's climb is greedy in the same place; a tied copy
        // stands where the interpreter retries instead, and ranks equal
        let choice = match copy {
            Some(c) if !c.ex.tied => Js::call("prec", vec![Js::num(1), Js::call("choice", alts)]),
            _ => Js::call("choice", alts),
        };
        let mut out = vec![(own(name), doc.to_string(), choice)];
        for node in nodes {
            let ops: Vec<_> = c.ops.iter().filter(|o| o.node == node).collect();
            let docs: Vec<&str> = ops
                .iter()
                .map(|o| o.doc.as_str())
                .filter(|d| !d.is_empty())
                .collect();
            let mut forms: Vec<Js> = Vec::new();
            for op in &ops {
                if self.seq_items(op_text(op)).is_empty() {
                    continue;
                }
                let expr = climb_sym(name, copy);
                let body = match &op.form {
                    Form::Prefix {
                        head,
                        operand,
                        root,
                    } => {
                        let mut items = self.seq_items(head);
                        let operand_rule = match root {
                            Some(node) => node_ref(node, copy),
                            None => expr,
                        };
                        items.push(Js::call("field", vec![Js::str(operand), operand_rule]));
                        Js::call("seq", items)
                    }
                    Form::Infix { left, op: o, right } => {
                        let mut items = vec![Js::call("field", vec![Js::str(left), expr.clone()])];
                        items.extend(self.seq_items(o));
                        items.push(Js::call("field", vec![Js::str(right), expr]));
                        Js::call("seq", items)
                    }
                    Form::Postfix {
                        target,
                        tail,
                        targets,
                    } => {
                        let t = match targets {
                            Some(ks) => Js::call(
                                "choice",
                                ks.iter()
                                    .filter(|k| !excluded(k))
                                    .map(|k| {
                                        if c.ops.iter().any(|o| &o.node == k) {
                                            node_ref(k, copy)
                                        } else {
                                            Js::sym(k)
                                        }
                                    })
                                    .collect(),
                            ),
                            None => expr,
                        };
                        let mut items = vec![Js::call("field", vec![Js::str(target), t])];
                        items.extend(self.seq_items(tail));
                        Js::call("seq", items)
                    }
                };
                let body = match copy {
                    Some(c) => rename(body, name, c),
                    None => body,
                };
                // the interpreter reads a prefix operator's operand at the
                // operator's own precedence, so an operator of that
                // precedence after the operand extends the operand rather
                // than the prefix form: tree-sitter's right associativity,
                // whatever the operator states
                let prec = match (&op.form, op.assoc) {
                    (Form::Prefix { .. }, _) | (_, Assoc::Right) => "prec.right",
                    (_, Assoc::None) => "prec",
                    (_, Assoc::Left) => "prec.left",
                };
                // in a copy, every precedence is lifted to `2p + 1`: the
                // operators keep their order among themselves, and each
                // stands above its original wherever the two could take
                // the same text
                let p = match copy {
                    Some(_) => 2 * op.prec + 1,
                    None => op.prec,
                };
                let mut form = Js::call(prec, vec![Js::num(p), body]);
                if op.dynamic != 0 {
                    form = Js::call("prec.dynamic", vec![Js::num(op.dynamic), form]);
                }
                forms.push(form);
            }
            if forms.is_empty() {
                continue;
            }
            let rule = if forms.len() == 1 {
                forms.pop().unwrap()
            } else {
                Js::call("choice", forms)
            };
            let doc = match copy {
                Some(_) => String::new(),
                None => docs.join("\n"),
            };
            out.push((own(node), doc, rule));
        }
        out
    }

    /// A rule's items as the members of a `seq`, flattened one level.
    fn seq_items(&self, r: &Rule) -> Vec<Js> {
        match r {
            Rule::Seq(rs) => rs.iter().filter_map(|r| self.js(r)).collect(),
            r => self.js(r).into_iter().collect(),
        }
    }

    /// Whether a rule is `repeat` of the newline token the scanner reads,
    /// which prints as nothing: a newline the parser cannot take as a
    /// separator is whitespace.
    fn optional_newlines(&self, r: &Rule) -> bool {
        matches!((r, &self.g.newline), (Rule::Repeat(inner), Some(n)) if **inner == Rule::Sym(n.clone()))
    }

    /// A rule as JavaScript, or nothing where the rule is the
    /// interpreter's alone.
    fn js(&self, r: &Rule) -> Option<Js> {
        if self.optional_newlines(r) {
            return None;
        }
        Some(match r {
            Rule::Blank => blank(),
            Rule::Str(s) => Js::str(s),
            Rule::Pat(p) => Js::Pat(p.to_regex()),
            Rule::Sym(n) => Js::sym(n),
            Rule::Lookahead(n) => Js::sym(n),
            Rule::Without(ex) => Js::sym(&copy_name(ex)),
            // a sequence inside a sequence, and a choice inside a choice,
            // print flat: the nesting says nothing tree-sitter reads
            Rule::Seq(rs) => {
                let mut items: Vec<Js> = Vec::new();
                for r in rs {
                    match r {
                        Rule::Seq(inner) => items.extend(inner.iter().filter_map(|r| self.js(r))),
                        r => items.extend(self.js(r)),
                    }
                }
                match items.len() {
                    0 => blank(),
                    1 => items.into_iter().next().unwrap(),
                    _ => Js::call("seq", items),
                }
            }
            Rule::Choice(rs) => {
                let mut items: Vec<Js> = Vec::new();
                for r in rs {
                    match r {
                        Rule::Choice(inner) => {
                            items.extend(inner.iter().filter_map(|r| self.js(r)))
                        }
                        r => items.extend(self.js(r)),
                    }
                }
                // a choice whose alternatives are all the interpreter's — a
                // lookahead check — prints as nothing, since an empty
                // `choice()` would match nothing at all
                match items.len() {
                    0 => return None,
                    1 => items.into_iter().next().unwrap(),
                    _ => Js::call("choice", items),
                }
            }
            Rule::Repeat(r) => Js::call("repeat", vec![self.js(r)?]),
            Rule::Repeat1(r) => Js::call("repeat1", vec![self.js(r)?]),
            Rule::Optional(r) => Js::call("optional", vec![self.js(r)?]),
            Rule::Prec(n, r) => Js::call("prec", vec![Js::num(*n), self.js(r)?]),
            Rule::PrecLeft(n, r) => Js::call("prec.left", vec![Js::num(*n), self.js(r)?]),
            Rule::PrecRight(n, r) => Js::call("prec.right", vec![Js::num(*n), self.js(r)?]),
            Rule::PrecDynamic(n, r) => Js::call("prec.dynamic", vec![Js::num(*n), self.js(r)?]),
            Rule::Token(r) => match (&**r, &self.g.newline) {
                // a token carrying its leading newlines: the scanner
                // tells the line it opens from a fresh one and leaves the
                // newlines to the extras, so the token is its text alone,
                // and one that must open a line is the text it aliases
                (Rule::Pat(p), Some(_)) => match crate::scanner::without_lead(p) {
                    Some(Some(rest)) => Js::call("token", vec![Js::Pat(rest.to_regex())]),
                    Some(None) => return None,
                    None => Js::call("token", vec![self.js(r)?]),
                },
                _ => Js::call("token", vec![self.js(r)?]),
            },
            Rule::Immediate(r) => Js::call("token.immediate", vec![self.js(r)?]),
            Rule::Field(f, r) => Js::call("field", vec![Js::str(f), self.js(r)?]),
            Rule::Alias(r, name) => Js::call("alias", vec![self.js(r)?, Js::str(name)]),
            Rule::NotAhead(_) | Rule::NotAheadSoft(_) | Rule::Ahead(_) | Rule::Refuse(..) => {
                return None
            }
            Rule::Expect(r, _) | Rule::Recover(r, _) => self.js(r)?,
        })
    }
}

/// An exclusion copy of a climb: a `Without` somewhere in the grammar
/// asks for an expression rule with some operator nodes left out, which
/// the printer writes as a second set of rules — `{expr}_{label}` and
/// one `{node}_{label}` per remaining operator, each aliased back to the
/// original name at its reference — so tree-sitter parses a loop header
/// with the same reduced grammar the interpreter does.
#[derive(Clone, Debug, PartialEq)]
struct Copy {
    ex: Exclusion,
    /// The copy's rule name: the climb's name and the label.
    name: String,
}

/// Every distinct exclusion the grammar asks for, in order of first
/// appearance. Two exclusions with one label must state one rule.
fn copies(g: &Grammar) -> Vec<Copy> {
    let mut out: Vec<Copy> = Vec::new();
    let mut visit = |r: &Rule| {
        withouts(
            r,
            &mut |ex| match out.iter().find(|c| c.ex.label == ex.label) {
                Some(c) => assert!(
                    c.ex == *ex,
                    "the exclusion label `{}` names two different exclusions",
                    ex.label
                ),
                None => out.push(Copy {
                    ex: ex.clone(),
                    name: copy_name(ex),
                }),
            },
        )
    };
    for d in &g.rules {
        match &d.body {
            Body::Rule(r) => visit(r),
            Body::Climb(c) => {
                visit(&c.primary);
                for op in &c.ops {
                    visit(op_text(op));
                }
            }
        }
    }
    out
}

fn withouts(r: &Rule, f: &mut impl FnMut(&Exclusion)) {
    match r {
        Rule::Without(ex) => f(ex),
        Rule::Seq(rs) | Rule::Choice(rs) => rs.iter().for_each(|r| withouts(r, f)),
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
        | Rule::NotAhead(r)
        | Rule::NotAheadSoft(r)
        | Rule::Ahead(r)
        | Rule::Expect(r, _)
        | Rule::Refuse(r, _)
        | Rule::Recover(r, _) => withouts(r, f),
        Rule::Blank | Rule::Str(_) | Rule::Pat(_) | Rule::Sym(_) | Rule::Lookahead(_) => {}
    }
}

/// The name of the exclusion copy a `Without` refers to.
fn copy_name(ex: &Exclusion) -> String {
    format!("{}_{}", ex.expr, ex.label)
}

/// A symbol reference inside a climb's own rules: the copy's name where
/// a copy is being printed.
fn climb_sym(name: &str, copy: Option<&Copy>) -> Js {
    match copy {
        Some(c) => Js::sym(&c.name),
        None => Js::sym(name),
    }
}

/// A reference to an operator node from inside a climb: in a copy, the
/// copy's node aliased to the original.
fn node_ref(node: &str, copy: Option<&Copy>) -> Js {
    match copy {
        Some(c) => Js::call(
            "alias",
            vec![Js::sym(&format!("{node}_{}", c.ex.label)), Js::sym(node)],
        ),
        None => Js::sym(node),
    }
}

/// Every `$.name` leaf of a climb's own text renamed to the copy's, so
/// an operator's head that holds expressions of its own reads them in
/// the same reduced grammar.
fn rename(j: Js, name: &str, copy: &Copy) -> Js {
    match j {
        Js::Sym(s) if s == name => Js::sym(&copy.name),
        Js::Call(n, args) => Js::Call(n, args.into_iter().map(|a| rename(a, name, copy)).collect()),
        Js::List(items) => Js::List(items.into_iter().map(|a| rename(a, name, copy)).collect()),
        leaf => leaf,
    }
}

/// The text an operator matches beside its operands.
pub(crate) fn op_text(op: &Op) -> &Rule {
    match &op.form {
        Form::Prefix { head, .. } => head,
        Form::Infix { op, .. } => op,
        Form::Postfix { tail, .. } => tail,
    }
}

fn blank() -> Js {
    Js::call("blank", vec![])
}

fn mentions_symbol(j: &Js) -> bool {
    match j {
        Js::Sym(_) => true,
        Js::Str(_) | Js::Num(_) | Js::Pat(_) => false,
        Js::Call(_, args) | Js::List(args) => args.iter().any(mentions_symbol),
    }
}

/// A tree-sitter DSL expression: laid out as JavaScript by [`render`],
/// and as the `grammar.json` rule it evaluates to by [`Js::to_json`].
#[derive(Clone, Debug)]
enum Js {
    /// `$.name`.
    Sym(String),
    /// A string literal.
    Str(String),
    Num(i32),
    /// A regex, without its delimiters.
    Pat(String),
    /// A DSL function applied to its arguments: `seq`, `prec.left`, ….
    Call(String, Vec<Js>),
    List(Vec<Js>),
}

impl Js {
    fn call(name: &str, args: Vec<Js>) -> Js {
        Js::Call(name.to_string(), args)
    }
    fn list(items: Vec<Js>) -> Js {
        Js::List(items)
    }
    fn sym(name: &str) -> Js {
        Js::Sym(name.to_string())
    }
    fn str(s: &str) -> Js {
        Js::Str(s.to_string())
    }
    fn num(n: i32) -> Js {
        Js::Num(n)
    }

    /// The `grammar.json` rule tree-sitter's DSL evaluates this
    /// expression to.
    fn to_json(&self) -> Json {
        let rule = |kind: &str, fields: Vec<(&str, Json)>| {
            let mut all = vec![("type".to_string(), Json::str(kind))];
            all.extend(fields.into_iter().map(|(k, v)| (k.to_string(), v)));
            Json::Obj(all)
        };
        let members = |args: &[Js]| Json::Arr(args.iter().map(Js::to_json).collect());
        match self {
            Js::Sym(name) => rule("SYMBOL", vec![("name", Json::str(name))]),
            Js::Str(s) => rule("STRING", vec![("value", Json::str(s))]),
            Js::Pat(p) => rule("PATTERN", vec![("value", Json::str(p))]),
            Js::Num(n) => Json::Num(*n),
            Js::List(items) => members(items),
            Js::Call(f, args) => match (f.as_str(), args.as_slice()) {
                ("blank", []) => rule("BLANK", vec![]),
                ("seq", args) => rule("SEQ", vec![("members", members(args))]),
                ("choice", args) => rule("CHOICE", vec![("members", members(args))]),
                ("optional", [r]) => rule(
                    "CHOICE",
                    vec![(
                        "members",
                        Json::Arr(vec![r.to_json(), rule("BLANK", vec![])]),
                    )],
                ),
                ("repeat", [r]) => rule("REPEAT", vec![("content", r.to_json())]),
                ("repeat1", [r]) => rule("REPEAT1", vec![("content", r.to_json())]),
                ("token", [r]) => rule("TOKEN", vec![("content", r.to_json())]),
                ("token.immediate", [r]) => rule("IMMEDIATE_TOKEN", vec![("content", r.to_json())]),
                ("field", [Js::Str(name), r]) => rule(
                    "FIELD",
                    vec![("name", Json::str(name)), ("content", r.to_json())],
                ),
                ("alias", [r, target]) => {
                    let (named, value) = match target {
                        Js::Sym(name) => (true, name),
                        Js::Str(name) => (false, name),
                        other => panic!("an alias names a symbol or a string, not {other:?}"),
                    };
                    rule(
                        "ALIAS",
                        vec![
                            ("content", r.to_json()),
                            ("named", Json::Bool(named)),
                            ("value", Json::str(value)),
                        ],
                    )
                }
                (prec, [n, r]) if prec.starts_with("prec") => {
                    let kind = match prec {
                        "prec" => "PREC",
                        "prec.left" => "PREC_LEFT",
                        "prec.right" => "PREC_RIGHT",
                        "prec.dynamic" => "PREC_DYNAMIC",
                        other => panic!("no precedence `{other}`"),
                    };
                    rule(kind, vec![("value", n.to_json()), ("content", r.to_json())])
                }
                (f, args) => panic!("no DSL function `{f}` of {} arguments", args.len()),
            },
        }
    }
}

fn js_string(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// The expression at `indent`, on one line where it fits in `width`
/// columns and one argument per line otherwise.
fn render(j: &Js, indent: usize, width: usize) -> String {
    let flat = flat(j);
    if indent + flat.len() <= width {
        return flat;
    }
    let (open, close, args) = match j {
        Js::Call(name, args) => (format!("{name}("), ")", args),
        Js::List(items) => ("[".to_string(), "]", items),
        _ => return flat,
    };
    let pad = " ".repeat(indent + 2);
    let mut out = open;
    out.push('\n');
    for a in args {
        out.push_str(&pad);
        out.push_str(&render(a, indent + 2, width));
        out.push_str(",\n");
    }
    out.push_str(&" ".repeat(indent));
    out.push_str(close);
    out
}

fn flat(j: &Js) -> String {
    match j {
        Js::Sym(name) => format!("$.{name}"),
        Js::Str(s) => js_string(s),
        Js::Num(n) => n.to_string(),
        Js::Pat(p) => format!("/{p}/"),
        Js::Call(name, args) => {
            let items: Vec<String> = args.iter().map(flat).collect();
            format!("{name}({})", items.join(", "))
        }
        Js::List(items) => {
            let items: Vec<String> = items.iter().map(flat).collect();
            format!("[{}]", items.join(", "))
        }
    }
}
