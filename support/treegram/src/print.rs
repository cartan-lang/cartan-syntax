//! The grammar as a `grammar.js`: the text `tree-sitter generate`
//! reads. Every combinator prints as its tree-sitter spelling, a
//! [`Climb`] prints as one `prec`-annotated rule per operator node
//! beside the expression rule's `choice`, and the interpreter's own
//! constructs print as the rule they wrap or as nothing.

use crate::rule::{Assoc, Body, Climb, Def, Form, Grammar, Op, Rule};

/// The `grammar.js` text of a grammar.
pub fn print(g: &Grammar) -> String {
    let copies = copies(g);
    COPIES.with(|c| *c.borrow_mut() = copies.clone());
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
    let extras: Vec<Js> = g.extras.iter().filter_map(js).collect();
    out.push_str("  extras: ($) => ");
    out.push_str(&render(&Js::list(extras), 2, 80));
    out.push_str(",\n\n");
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
    let mut first = true;
    for d in &g.rules {
        for (name, doc, rule) in printed_rules(d, &copies) {
            if !first {
                out.push('\n');
            }
            first = false;
            for line in doc.lines() {
                out.push_str("    // ");
                out.push_str(line);
                out.push('\n');
            }
            let arg = if mentions_symbol(&rule) { "($)" } else { "(_)" };
            let flat = format!("    {name}: {arg} => {},", render(&rule, 4, usize::MAX));
            if flat.len() <= 80 {
                out.push_str(&flat);
                out.push('\n');
            } else {
                out.push_str(&format!("    {name}: {arg} =>\n      "));
                out.push_str(&render(&rule, 6, 80));
                out.push_str(",\n");
            }
        }
    }
    out.push_str("  },\n});\n");
    out
}

/// An exclusion copy of a climb: `Without(name, nodes)` somewhere in
/// the grammar asks for the expression rule `name` with the operator
/// nodes `nodes` left out, which the printer writes as a second set of
/// rules — `name_k` and one `node_k` per remaining operator, each
/// aliased back to the original name at its reference — so
/// tree-sitter parses a loop header with the same reduced grammar the
/// interpreter does.
#[derive(Clone, Debug, PartialEq)]
struct Copy {
    climb: String,
    excluded: Vec<String>,
    /// Whether the copy ties with the original rather than outranking
    /// it ([`crate::rule::without_tied`]).
    tied: bool,
    /// The copy's rule name: the climb's name and an index.
    name: String,
}

/// Every distinct exclusion the grammar asks for, in order of first
/// appearance.
fn copies(g: &Grammar) -> Vec<Copy> {
    let mut out: Vec<Copy> = Vec::new();
    let mut visit = |r: &Rule| {
        withouts(r, &mut |climb, excluded, tied| {
            if !out
                .iter()
                .any(|c| c.climb == climb && c.excluded == excluded && c.tied == tied)
            {
                let name = format!(
                    "{climb}_{}",
                    out.iter().filter(|c| c.climb == climb).count() + 1
                );
                out.push(Copy {
                    climb: climb.to_string(),
                    excluded: excluded.to_vec(),
                    tied,
                    name,
                });
            }
        })
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

fn withouts(r: &Rule, f: &mut impl FnMut(&str, &[String], bool)) {
    match r {
        Rule::Without(n, ns, tied) => f(n, ns, *tied),
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
        | Rule::Ahead(r)
        | Rule::Expect(r, _)
        | Rule::Refuse(r, _)
        | Rule::Recover(r, _) => withouts(r, f),
        Rule::Blank | Rule::Str(_) | Rule::Pat(_) | Rule::Sym(_) => {}
    }
}

/// The rules a definition prints as: one for an ordinary rule, and for
/// a climb the expression rule followed by one rule per operator node,
/// then the same again for each exclusion copy of it.
fn printed_rules(d: &Def, copies: &[Copy]) -> Vec<(String, String, Js)> {
    match &d.body {
        Body::Rule(r) => vec![(d.name.clone(), d.doc.clone(), js(r).unwrap_or_else(blank))],
        Body::Climb(c) => {
            let mut out = climb_rules(&d.name, &d.doc, c, None);
            for copy in copies.iter().filter(|c| c.climb == d.name) {
                let excluded: Vec<String> =
                    copy.excluded.iter().map(|n| format!("`{n}`")).collect();
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
                out.extend(climb_rules(&d.name, &doc, c, Some(copy)));
            }
            out
        }
    }
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
            vec![Js::sym(&format!("{node}_{}", suffix(c))), Js::sym(node)],
        ),
        None => Js::sym(node),
    }
}

/// The index a copy's rule names end in.
fn suffix(c: &Copy) -> &str {
    c.name.rsplit('_').next().unwrap_or("")
}

/// Every `$.name` leaf of a climb's own text renamed to the copy's, so
/// an operator's head — `if c then a else` — reads its expressions in
/// the same reduced grammar.
fn rename(j: Js, name: &str, copy: &Copy) -> Js {
    match j {
        Js::Leaf(s) if s == format!("$.{name}") => Js::sym(&copy.name),
        Js::Leaf(s) => Js::Leaf(s),
        Js::Call(n, args) => Js::Call(n, args.into_iter().map(|a| rename(a, name, copy)).collect()),
        Js::List(items) => Js::List(items.into_iter().map(|a| rename(a, name, copy)).collect()),
    }
}

fn climb_rules(name: &str, doc: &str, c: &Climb, copy: Option<&Copy>) -> Vec<(String, String, Js)> {
    let mut alts: Vec<Js> = match &c.primary {
        Rule::Choice(rs) => rs.iter().filter_map(js).collect(),
        r => js(r).into_iter().collect(),
    };
    let excluded = |node: &str| copy.is_some_and(|c| c.excluded.iter().any(|n| n == node));
    let mut nodes: Vec<&str> = Vec::new();
    for op in &c.ops {
        // an operator whose text is the interpreter's alone — a
        // refusal — has nothing for tree-sitter to read
        if seq_items(op_text(op)).is_empty() || excluded(&op.node) {
            continue;
        }
        if !nodes.contains(&op.node.as_str()) {
            nodes.push(&op.node);
            alts.push(node_ref(&op.node, copy));
        }
    }
    let own = |n: &str| match copy {
        Some(c) => format!("{n}_{}", suffix(c)),
        None => n.to_string(),
    };
    // a copy outranks the original wherever the two could both take a
    // word — a header's expression followed by a bare body — since the
    // interpreter's climb is greedy in the same place; a tied copy
    // stands where the interpreter retries instead, and ranks equal
    let choice = match copy {
        Some(c) if !c.tied => Js::call("prec", vec![Js::num(1), Js::call("choice", alts)]),
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
            if seq_items(op_text(op)).is_empty() {
                continue;
            }
            let expr = climb_sym(name, copy);
            let body = match &op.form {
                Form::Prefix { head, operand } => {
                    let mut items = seq_items(head);
                    items.push(Js::call("field", vec![Js::str(operand), expr]));
                    Js::call("seq", items)
                }
                Form::Infix { left, op: o, right } => {
                    let mut items = vec![Js::call("field", vec![Js::str(left), expr.clone()])];
                    items.extend(seq_items(o));
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
                    items.extend(seq_items(tail));
                    Js::call("seq", items)
                }
            };
            let body = match copy {
                Some(c) => rename(body, name, c),
                None => body,
            };
            let prec = match op.assoc {
                Assoc::None => "prec",
                Assoc::Left => "prec.left",
                Assoc::Right => "prec.right",
            };
            // in a copy, every precedence is lifted to `2p + 1`: the
            // operators keep their order among themselves, and each stands
            // above its original wherever the two could take the same text
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

/// The text an operator matches beside its operands.
fn op_text(op: &Op) -> &Rule {
    match &op.form {
        Form::Prefix { head, .. } => head,
        Form::Infix { op, .. } => op,
        Form::Postfix { tail, .. } => tail,
    }
}

/// A rule's items as the members of a `seq`, flattened one level.
fn seq_items(r: &Rule) -> Vec<Js> {
    match r {
        Rule::Seq(rs) => rs.iter().filter_map(js).collect(),
        r => js(r).into_iter().collect(),
    }
}

fn blank() -> Js {
    Js::call("blank", vec![])
}

/// A rule as JavaScript, or nothing where the rule is the
/// interpreter's alone.
fn js(r: &Rule) -> Option<Js> {
    Some(match r {
        Rule::Blank => blank(),
        Rule::Str(s) => Js::str(s),
        Rule::Pat(p) => Js::Leaf(p.to_js()),
        Rule::Sym(n) => Js::sym(n),
        Rule::Without(n, ns, tied) => Js::sym(&copy_name(n, ns, *tied)),
        // a sequence inside a sequence, and a choice inside a choice,
        // print flat: the nesting says nothing tree-sitter reads
        Rule::Seq(rs) => {
            let mut items: Vec<Js> = Vec::new();
            for r in rs {
                match r {
                    Rule::Seq(inner) => items.extend(inner.iter().filter_map(js)),
                    r => items.extend(js(r)),
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
                    Rule::Choice(inner) => items.extend(inner.iter().filter_map(js)),
                    r => items.extend(js(r)),
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
        Rule::Repeat(r) => Js::call("repeat", vec![js(r)?]),
        Rule::Repeat1(r) => Js::call("repeat1", vec![js(r)?]),
        Rule::Optional(r) => Js::call("optional", vec![js(r)?]),
        Rule::Prec(n, r) => Js::call("prec", vec![Js::num(*n), js(r)?]),
        Rule::PrecLeft(n, r) => Js::call("prec.left", vec![Js::num(*n), js(r)?]),
        Rule::PrecRight(n, r) => Js::call("prec.right", vec![Js::num(*n), js(r)?]),
        Rule::PrecDynamic(n, r) => Js::call("prec.dynamic", vec![Js::num(*n), js(r)?]),
        Rule::Token(r) => Js::call("token", vec![js(r)?]),
        Rule::Immediate(r) => Js::call("token.immediate", vec![js(r)?]),
        Rule::Field(f, r) => Js::call("field", vec![Js::str(f), js(r)?]),
        Rule::Alias(r, name) => Js::call("alias", vec![js(r)?, Js::str(name)]),
        Rule::NotAhead(_) | Rule::Ahead(_) | Rule::Refuse(..) => return None,
        Rule::Expect(r, _) | Rule::Recover(r, _) => js(r)?,
    })
}

/// The name of the exclusion copy a `Without` refers to. The copies are
/// numbered in order of first appearance, which [`copies`] and this
/// agree on by walking the grammar the same way; the walk here is
/// repeated per reference and the grammar is small.
fn copy_name(climb: &str, excluded: &[String], tied: bool) -> String {
    COPIES.with(|c| {
        let list = c.borrow();
        list.iter()
            .find(|x| x.climb == climb && x.excluded == excluded && x.tied == tied)
            .map(|x| x.name.clone())
            .unwrap_or_else(|| climb.to_string())
    })
}

thread_local! {
    /// The copies of the grammar being printed, for [`copy_name`].
    static COPIES: std::cell::RefCell<Vec<Copy>> = const { std::cell::RefCell::new(Vec::new()) };
}

fn mentions_symbol(j: &Js) -> bool {
    match j {
        Js::Leaf(s) => s.starts_with("$."),
        Js::Call(_, args) | Js::List(args) => args.iter().any(mentions_symbol),
    }
}

/// A JavaScript expression, laid out by [`render`].
#[derive(Clone, Debug)]
enum Js {
    Leaf(String),
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
        Js::Leaf(format!("$.{name}"))
    }
    fn str(s: &str) -> Js {
        Js::Leaf(js_string(s))
    }
    fn num(n: i32) -> Js {
        Js::Leaf(n.to_string())
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
        Js::Leaf(_) => return flat,
        Js::Call(name, args) => (format!("{name}("), ")", args),
        Js::List(items) => ("[".to_string(), "]", items),
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
        Js::Leaf(s) => s.clone(),
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
