//! The lexical layer of a grammar, read off the value: what a
//! line-based highlighter colors, and the shared reading the four
//! editor printers beside this module take.
//!
//! An editor's coloring is regex matching over one line, so what a
//! printer states is what a lexer can see — comments, strings,
//! numbers, keywords, operators — and the structural coloring comes
//! from a language server's semantic tokens.
//!
//! What prints is read off the value. The keyword set is every literal
//! token the rules state whose text matches the grammar's word
//! pattern, a `cont`-style alias included; the operator set is the
//! rest, less the brackets and separators an editor leaves uncolored.
//! The `comment`, `string` and `number` rules print as the regex their
//! tokens spell, in whichever dialect the printer asks for. One
//! construct is stated by name rather than derived: a rule named
//! `triple_string` prints as a begin/end pair on `"""`, since its text
//! spans lines and a one-line match reads none.

use crate::pattern::{Dialect, Pat};
use crate::rule::{Body, Def, Form, Grammar, Rule};

/// What an editor colors, read off a grammar value: the word tokens
/// split into keywords and the named constants, the operator tokens,
/// and whether the grammar states a triple-quoted string.
pub struct Lexical {
    /// The word tokens that are keywords, sorted.
    pub keywords: Vec<String>,
    /// The operator tokens, longest first so that a longer operator
    /// matches before a prefix of it.
    pub operators: Vec<String>,
    /// The word tokens named as values rather than keywords, sorted.
    pub constants: Vec<String>,
    /// Whether the grammar states a `triple_string` rule.
    pub triple_string: bool,
}

impl Lexical {
    /// Read `g`'s lexical layer. `constants` names the word tokens
    /// that are values rather than keywords — `true`, `none`.
    pub fn of(g: &Grammar, constants: &[&str]) -> Lexical {
        let word = word_pattern(g);
        let mut keywords: Vec<String> = Vec::new();
        let mut operators: Vec<String> = Vec::new();
        for s in tokens(g) {
            let is_word = word
                .as_ref()
                .is_some_and(|w| w.longest(&s, 0) == Some(s.len()))
                && !s.is_empty();
            if constants.contains(&s.as_str()) {
                continue;
            }
            // a token ending in a quotation mark opens or closes a
            // string — the delimiter itself, and a tag standing against
            // it — and the `string` rule's own regex colors what it
            // delimits, so coloring it as an operator would take the
            // opening quote out of that match
            if is_word {
                keywords.push(s);
            } else if !matches!(s.as_str(), "(" | ")" | "[" | "]" | "{" | "}" | ",")
                && !s.ends_with('"')
            {
                operators.push(s);
            }
        }
        keywords.sort();
        keywords.dedup();
        operators.sort_by(|a, b| b.len().cmp(&a.len()).then(a.cmp(b)));
        operators.dedup();
        let mut cs: Vec<String> = constants.iter().map(|s| s.to_string()).collect();
        cs.sort();
        Lexical {
            keywords,
            operators,
            constants: cs,
            triple_string: find(g, "triple_string").is_some(),
        }
    }

    /// The operators as one alternation, longest first, in `d`'s
    /// spelling.
    pub fn operator_regex(&self, d: Dialect) -> String {
        let alts: Vec<String> = self.operators.iter().map(|s| d.literal(s)).collect();
        alts.join(d.alt())
    }
}

/// A named rule's body as one regex in `d`'s spelling, for the
/// token-shaped rules — `comment`, `string`, `number`.
pub fn regex(g: &Grammar, rule: &str, d: Dialect) -> Option<String> {
    let def = find(g, rule)?;
    match &def.body {
        Body::Rule(r) => rule_regex(r, g, 0, d),
        Body::Climb(_) => None,
    }
}

/// The word pattern the keywords are told apart with, where the
/// grammar names one.
fn word_pattern(g: &Grammar) -> Option<Pat> {
    let w = g.word.as_ref()?;
    match &find(g, w)?.body {
        Body::Rule(Rule::Pat(p)) => Some(p.clone()),
        _ => None,
    }
}

fn find<'g>(g: &'g Grammar, name: &str) -> Option<&'g Def> {
    g.rules.iter().find(|d| d.name == name)
}

/// Every literal token the grammar states, aliases included.
fn tokens(g: &Grammar) -> Vec<String> {
    let mut out = Vec::new();
    for d in &g.rules {
        match &d.body {
            Body::Rule(r) => collect(r, &mut out),
            Body::Climb(c) => {
                collect(&c.primary, &mut out);
                for op in &c.ops {
                    match &op.form {
                        Form::Prefix { head, .. } => collect(head, &mut out),
                        Form::Infix { op, .. } => collect(op, &mut out),
                        Form::Postfix { tail, .. } => collect(tail, &mut out),
                    }
                }
            }
        }
    }
    out
}

fn collect(r: &Rule, out: &mut Vec<String>) {
    match r {
        Rule::Str(s) => out.push(s.clone()),
        // a `cont` operator is a pattern aliased to the operator's own
        // text, and the alias is the token the reader sees
        Rule::Alias(inner, name) => {
            out.push(name.clone());
            collect(inner, out);
        }
        Rule::Seq(rs) | Rule::Choice(rs) => rs.iter().for_each(|r| collect(r, out)),
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
        | Rule::Expect(r, _)
        | Rule::Recover(r, _) => collect(r, out),
        Rule::Blank
        | Rule::Pat(_)
        | Rule::Sym(_)
        | Rule::NotAhead(_)
        | Rule::Ahead(_)
        | Rule::Refuse(..)
        | Rule::Without(..) => {}
    }
}

/// A reference to another rule inlines that rule's regex, to a small
/// depth.
fn rule_regex(r: &Rule, g: &Grammar, depth: usize, d: Dialect) -> Option<String> {
    if depth > 8 {
        return None;
    }
    Some(match r {
        Rule::Blank => String::new(),
        Rule::Str(s) => d.literal(s),
        Rule::Pat(p) => format!("{}{}{}", d.open(), p.to_dialect(d), d.close()),
        Rule::Sym(n) => rule_regex(&as_rule(find(g, n)?)?, g, depth + 1, d)?,
        Rule::Seq(rs) => {
            let mut s = String::new();
            for r in rs {
                s.push_str(&rule_regex(r, g, depth, d)?);
            }
            s
        }
        Rule::Choice(rs) => {
            let alts: Option<Vec<String>> = rs.iter().map(|r| rule_regex(r, g, depth, d)).collect();
            d.group(&alts?)
        }
        Rule::Repeat(r) => format!("{}{}{}*", d.open(), rule_regex(r, g, depth, d)?, d.close()),
        Rule::Repeat1(r) => format!("{}{}{}+", d.open(), rule_regex(r, g, depth, d)?, d.close()),
        Rule::Optional(r) => format!("{}{}{}?", d.open(), rule_regex(r, g, depth, d)?, d.close()),
        Rule::Prec(_, r)
        | Rule::PrecLeft(_, r)
        | Rule::PrecRight(_, r)
        | Rule::PrecDynamic(_, r)
        | Rule::Token(r)
        | Rule::Immediate(r)
        | Rule::Field(_, r)
        | Rule::Alias(r, _)
        | Rule::Expect(r, _)
        | Rule::Recover(r, _) => rule_regex(r, g, depth, d)?,
        Rule::NotAhead(_) | Rule::Ahead(_) | Rule::Refuse(..) | Rule::Without(..) => String::new(),
    })
}

fn as_rule(d: &Def) -> Option<Rule> {
    match &d.body {
        Body::Rule(r) => Some(r.clone()),
        Body::Climb(_) => None,
    }
}
