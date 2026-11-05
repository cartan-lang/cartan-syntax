//! The interpreter: a document read against a [`Grammar`] directly.
//!
//! It is a scannerless, memoizing, ordered-choice parser — a packrat
//! parser — with one addition, precedence climbing for a [`Climb`].
//! Tokens are matched against the source text where they stand, extras
//! are skipped before every token that is not immediate, a keyword is
//! a literal the word pattern matches whole, and a named rule's match
//! is remembered by position so no rule is parsed twice at one place.
//!
//! A failure is soft or hard. A soft failure is the ordinary one: the
//! rule does not match here, and the enclosing choice tries its next
//! alternative. A hard failure is a diagnostic the grammar stated with
//! [`Rule::Expect`] or [`Rule::Refuse`]: the parse of the enclosing
//! [`Rule::Recover`] region ends with that label, and the region
//! becomes an `ERROR` node. Where a soft failure reaches a recovery
//! region, the diagnostic is the furthest position the parse reached
//! and the tokens it looked for there.
//!
//! **An `expect` yields to a failure that read further.** An ordered
//! choice tries its alternatives in turn, so an `expect` can fire from
//! an alternative the parser reached only because an earlier one failed
//! further along: a stray bracket in a function's body fails the
//! definition softly at the bracket, and the plain-binding alternative
//! tried next then states "expected `=`" back at the item's head. Where
//! the soft failure reached a later position than the `expect`, the
//! region reports the soft failure, which stands at the defect. A
//! `refuse` matched the text it refuses, so it states a fact about the
//! document wherever it fired and stands whatever the parse reached.

use std::collections::{HashMap, HashSet};
use std::hash::{BuildHasherDefault, Hasher};

use crate::pattern::Pat;
use crate::rule::{is_hidden, Assoc, Body, Climb, Form, Grammar, Label, Rule};
use crate::tree::{Diag, Field, Node, Said};

/// **The hash the parser's tables use.** A packrat parse asks its memo
/// a question at every rule and every position, and looks a rule name
/// and a keyword up as often; the keys are a few machine words and a
/// short name, and SipHash — what a `HashMap` reaches for by default,
/// to hold against a hostile key — costs more on such a key than the
/// lookup saves. This is the multiply-rotate hash (Firefox's `FxHash`)
/// over the same keys, which decides nothing but where a bucket sits.
#[derive(Default)]
struct Fx(u64);

/// The odd constant `FxHash` multiplies by.
const FX: u64 = 0x51_7c_c1_b7_27_22_0a_95;

impl Fx {
    #[inline]
    fn mix(&mut self, v: u64) {
        self.0 = (self.0.rotate_left(5) ^ v).wrapping_mul(FX);
    }
}

impl Hasher for Fx {
    #[inline]
    fn finish(&self) -> u64 {
        self.0
    }

    #[inline]
    fn write(&mut self, mut bytes: &[u8]) {
        while let Some((head, rest)) = bytes.split_first_chunk::<8>() {
            self.mix(u64::from_ne_bytes(*head));
            bytes = rest;
        }
        if let Some((head, rest)) = bytes.split_first_chunk::<4>() {
            self.mix(u32::from_ne_bytes(*head) as u64);
            bytes = rest;
        }
        for &b in bytes {
            self.mix(b as u64);
        }
    }

    #[inline]
    fn write_u8(&mut self, n: u8) {
        self.mix(n as u64);
    }

    #[inline]
    fn write_u64(&mut self, n: u64) {
        self.mix(n);
    }

    #[inline]
    fn write_i64(&mut self, n: i64) {
        self.mix(n as u64);
    }

    #[inline]
    fn write_usize(&mut self, n: usize) {
        self.mix(n as u64);
    }
}

type FxMap<K, V> = HashMap<K, V, BuildHasherDefault<Fx>>;
type FxSet<K> = HashSet<K, BuildHasherDefault<Fx>>;

/// What a parse returns: the tree, and the diagnostics in position
/// order. A document with a syntax error still returns a tree, with an
/// `ERROR` node standing over the text the grammar could not read.
#[derive(Clone, Debug)]
pub struct Parse {
    pub root: Node,
    pub diags: Vec<Diag>,
}

/// Parse `src` against the rule `root` of `g`.
pub fn parse(g: &Grammar, src: &str, root: &str) -> Parse {
    let mut e = Engine::new(g, src);
    let ctx = Ctx::default();
    let outcome = e.sym(root, 0, ctx, None, None);
    let mut diags = std::mem::take(&mut e.diags);
    let (mut children, end) = match outcome {
        Ok(Some((nodes, end))) => (nodes, end),
        Ok(None) => {
            diags.push(e.soft_diag());
            (vec![error_node(0, src.len())], src.len())
        }
        Err(Hard) => {
            let hard = e.hard.take().expect("a hard failure states its diagnostic");
            diags.push(e.furthest_of(hard));
            (vec![error_node(0, src.len())], src.len())
        }
    };
    let (trailing, rest) = e.take_extras(end);
    children.extend(trailing);
    if rest < src.len() {
        diags.push(if e.furthest >= rest {
            e.soft_diag()
        } else {
            // the text holds more than the grammar could read, so what
            // it needs is a repair rather than another line
            e.stuck_at(rest, None, false, Said::Expected(Vec::new()))
        });
        children.push(error_node(rest, src.len()));
    }
    let root = if children.len() == 1 && children[0].named && children[0].kind == root {
        children.pop().unwrap()
    } else {
        Node {
            kind: root.to_string(),
            named: true,
            field: None,
            extra: false,
            start: 0,
            end: src.len(),
            children,
        }
    };
    diags.sort_by_key(|d| d.pos);
    Parse { root, diags }
}

fn error_node(start: usize, end: usize) -> Node {
    Node {
        kind: "ERROR".into(),
        named: true,
        field: None,
        extra: false,
        start,
        end,
        children: Vec::new(),
    }
}

/// A hard failure. The diagnostic is on the engine.
struct Hard;

/// The diagnostic a hard failure states, and whether a soft failure
/// that read further outranks it.
struct HardDiag {
    diag: Diag,
    /// True for a [`Rule::Expect`], which states what a rule looked for
    /// and did not find: an ordered choice may have reached that rule
    /// only because an earlier alternative failed further along, and
    /// that alternative's position names the defect. False for a
    /// [`Rule::Refuse`], which matched the text it refuses and so
    /// states a fact about the document wherever it fired.
    outranked: bool,
}

/// A soft outcome: the nodes matched and the position after them, or
/// nothing where the rule does not match.
type Soft = Option<(Vec<Node>, usize)>;
type Outcome = Result<Soft, Hard>;

/// What the rule being matched inherits from its surroundings.
#[derive(Clone, Copy, Default)]
struct Ctx {
    /// Inside a `token(…)`: no extras are skipped and no nodes are built.
    in_token: bool,
    /// Matching an extra: no extras are skipped, and nodes are built,
    /// since a named extra stands in the tree as tree-sitter keeps it.
    bare: bool,
    /// The index of the exclusion set in force, 0 for none.
    exclude: usize,
}

impl Ctx {
    /// Whether a token here takes the extras before it.
    fn skips(&self) -> bool {
        !self.in_token && !self.bare
    }
}

/// The fields matched so far in the sequences enclosing a rule, for a
/// diagnostic's `{field}` to read.
struct Scope<'a> {
    nodes: &'a [Node],
    parent: Option<&'a Scope<'a>>,
}

impl Scope<'_> {
    /// Every field this scope and the scopes around it matched, nearest
    /// first, each name once: what a stated diagnostic reports.
    fn fields(&self) -> Vec<Field> {
        let mut out: Vec<Field> = Vec::new();
        let mut at = Some(self);
        while let Some(s) = at {
            for n in s.nodes {
                if let Some(name) = &n.field {
                    if !out.iter().any(|f| &f.name == name) {
                        out.push(Field {
                            name: name.clone(),
                            start: n.start,
                            end: n.end,
                        });
                    }
                }
            }
            at = s.parent;
        }
        out
    }
}

struct Engine<'g, 's> {
    g: &'g Grammar,
    src: &'s str,
    /// Rule name → index in `g.rules`.
    index: FxMap<&'g str, usize>,
    /// Operator node name → index of the climb that defines it.
    op_nodes: FxMap<&'g str, usize>,
    keywords: FxSet<String>,
    word: Option<&'g Pat>,
    /// The exclusion sets in force somewhere, interned; entry 0 is empty.
    excludes: Vec<Vec<String>>,
    /// (rule, position, exclusion, variant) → outcome. A parse ends
    /// with about one and a half entries per byte of the text it read,
    /// so the table is asked for that room at the outset rather than
    /// doubled and rehashed a dozen times on the way there.
    memo: FxMap<(usize, usize, usize, i64), Soft>,
    /// Position → the position after the extras there, for the
    /// positions whose extras hold no named node ([`Engine::take_extras`]).
    extras_at: FxMap<usize, usize>,
    /// The furthest position a token was looked for and not found, and
    /// what was looked for there.
    furthest: usize,
    expected: Vec<String>,
    /// Whether failures are being recorded (off inside lookahead).
    recording: bool,
    hard: Option<HardDiag>,
    diags: Vec<Diag>,
    /// The alias in force, which names an aliased token in a diagnostic.
    alias: Option<String>,
}

impl<'g, 's> Engine<'g, 's> {
    fn new(g: &'g Grammar, src: &'s str) -> Self {
        let mut index = FxMap::default();
        let mut op_nodes = FxMap::default();
        for (i, d) in g.rules.iter().enumerate() {
            index.insert(d.name.as_str(), i);
            if let Body::Climb(c) = &d.body {
                for op in &c.ops {
                    op_nodes.insert(op.node.as_str(), i);
                }
            }
        }
        let word = g
            .word
            .as_ref()
            .and_then(|w| g.def(w))
            .and_then(|d| match &d.body {
                Body::Rule(Rule::Pat(p)) => Some(p),
                _ => None,
            });
        Engine {
            g,
            src,
            index,
            op_nodes,
            keywords: g.keywords().into_iter().collect(),
            word,
            excludes: vec![Vec::new()],
            memo: FxMap::with_capacity_and_hasher(src.len(), Default::default()),
            extras_at: FxMap::with_capacity_and_hasher(src.len() / 4, Default::default()),
            furthest: 0,
            expected: Vec::new(),
            recording: true,
            hard: None,
            diags: Vec::new(),
            alias: None,
        }
    }

    // ---- diagnostics ------------------------------------------------

    /// **Whether a miss at `pos` would be recorded.** A token that
    /// fails behind the furthest position reached tells a diagnostic
    /// nothing, and most of the tokens an ordered choice tries do fail
    /// there; naming what was looked for costs a `String` and, for a
    /// pattern, printing the pattern as a regex, so the name is built
    /// only where [`Engine::miss`] would keep it.
    fn misses(&self, pos: usize) -> bool {
        self.recording && pos >= self.furthest
    }

    fn miss(&mut self, pos: usize, what: String) {
        if !self.recording {
            return;
        }
        if pos > self.furthest {
            self.furthest = pos;
            self.expected.clear();
        }
        if pos == self.furthest && !self.expected.contains(&what) {
            self.expected.push(what);
        }
    }

    /// The length of the text at `pos` a diagnostic quotes
    /// ([`Diag::found`]): the next run of word characters, or the next
    /// character, and none at the end of the text.
    fn found(&self, pos: usize) -> usize {
        self.src[pos..]
            .char_indices()
            .take_while(|(i, c)| *i == 0 || c.is_alphanumeric() || *c == '_')
            .map(|(i, c)| i + c.len_utf8())
            .last()
            .unwrap_or(0)
    }

    /// The diagnostic at `pos`: the text found there, and `end` where
    /// the diagnostic is about more text than that.
    fn stuck_at(&self, pos: usize, end: Option<usize>, incomplete: bool, said: Said) -> Diag {
        let n = self.found(pos);
        let found = (n > 0).then(|| self.src[pos..pos + n].to_string());
        Diag {
            pos,
            end: end.unwrap_or(pos + n),
            found,
            incomplete,
            said,
        }
    }

    /// **Whether the parse ran out of text at `pos`**: everything from
    /// there on is whitespace, so the token the parse looked for was
    /// never written rather than written wrong ([`Diag::incomplete`]).
    fn at_end(&self, pos: usize) -> bool {
        self.src[pos.min(self.src.len())..].trim().is_empty()
    }

    /// The diagnostic a soft failure states: what was looked for at the
    /// furthest position reached.
    fn soft_diag(&self) -> Diag {
        let pos = self.furthest;
        // a token looked for past the last character of the text is one
        // the writer has not typed yet
        let incomplete = self.at_end(pos) && !self.expected.is_empty();
        self.stuck_at(pos, None, incomplete, Said::Expected(self.expected.clone()))
    }

    /// The furthest-failure rule applied to a hard diagnostic: where an
    /// alternative read past the position a later alternative's
    /// [`Rule::Expect`] complains at, that alternative's position names
    /// the defect and its soft diagnostic is reported instead. A
    /// [`Rule::Refuse`] matched what it refuses, so it stands.
    fn furthest_of(&self, hard: HardDiag) -> Diag {
        if hard.outranked && self.furthest > hard.diag.pos {
            self.soft_diag()
        } else {
            hard.diag
        }
    }

    /// A hard failure stating `label` at `pos`, about the text to `end`
    /// where it refused some, with the fields `scope` matched.
    fn fail_hard(
        &mut self,
        pos: usize,
        end: Option<usize>,
        label: Label,
        scope: Option<&Scope>,
    ) -> Outcome {
        if self.hard.is_none() {
            let incomplete = self.at_end(pos);
            let refused = end.is_some();
            let fields = scope.map(Scope::fields).unwrap_or_default();
            let diag = self.stuck_at(
                pos,
                end,
                incomplete,
                Said::Stated {
                    label,
                    refused,
                    fields,
                },
            );
            self.hard = Some(HardDiag {
                diag,
                outranked: !refused,
            });
        }
        Err(Hard)
    }

    // ---- tokens -----------------------------------------------------

    /// The position after the extras standing at `pos`.
    fn skip_extras(&mut self, pos: usize) -> usize {
        self.take_extras(pos).1
    }

    /// The named extras standing at `pos` — the comments — as nodes,
    /// and the position after all the extras there.
    ///
    /// Every token that is not immediate skips the extras before it,
    /// and an ordered choice looks for many tokens at one position, so
    /// the same run of whitespace is matched again at every attempt.
    /// Where it holds no named extra — every position of a document
    /// but the few a comment stands at — the answer is a position and
    /// nothing else, and [`Engine::extras_at`] remembers it.
    fn take_extras(&mut self, mut pos: usize) -> (Vec<Node>, usize) {
        if let Some(&end) = self.extras_at.get(&pos) {
            return (Vec::new(), end);
        }
        let from = pos;
        let recording = self.recording;
        self.recording = false;
        let mut nodes = Vec::new();
        loop {
            let mut moved = false;
            for x in &self.g.extras {
                let ctx = Ctx {
                    in_token: false,
                    bare: true,
                    exclude: 0,
                };
                if let Ok(Some((ns, end))) = self.rule(x, pos, ctx, None, None) {
                    if end > pos {
                        nodes.extend(ns.into_iter().filter(|n| n.named).map(|mut n| {
                            n.extra = true;
                            n
                        }));
                        pos = end;
                        moved = true;
                        break;
                    }
                }
            }
            if !moved {
                break;
            }
        }
        self.recording = recording;
        if nodes.is_empty() {
            self.extras_at.insert(from, pos);
        }
        (nodes, pos)
    }

    /// Whether `s` is a whole word at `pos`: the word pattern matches
    /// exactly its length there.
    fn whole_word(&self, s: &str, pos: usize) -> bool {
        match self.word {
            Some(w) => w.longest(self.src, pos) == Some(pos + s.len()),
            None => true,
        }
    }

    fn literal(
        &mut self,
        s: &str,
        pos: usize,
        immediate: bool,
        ctx: Ctx,
        field: Option<&str>,
    ) -> Soft {
        let (mut nodes, p) = if immediate || !ctx.skips() {
            (Vec::new(), pos)
        } else {
            self.take_extras(pos)
        };
        let hit =
            self.src[p..].starts_with(s) && (!self.keywords.contains(s) || self.whole_word(s, p));
        if !hit {
            if self.misses(p) {
                self.miss(p, format!("`{s}`"));
            }
            return None;
        }
        let end = p + s.len();
        if !ctx.in_token {
            nodes.push(leaf(s, false, p, end, field));
        }
        Some((nodes, end))
    }

    fn pattern(
        &mut self,
        pat: &Pat,
        pos: usize,
        immediate: bool,
        ctx: Ctx,
        field: Option<&str>,
        label: Option<&str>,
    ) -> Soft {
        let (mut nodes, p) = if immediate || !ctx.skips() {
            (Vec::new(), pos)
        } else {
            self.take_extras(pos)
        };
        let end = pat.longest(self.src, p);
        let is_word = self.word.is_some_and(|w| std::ptr::eq(w, pat));
        let end = match end {
            Some(e) if !(is_word && self.keywords.contains(&self.src[p..e])) => e,
            _ => {
                if self.misses(p) {
                    let what = match (label, &self.alias) {
                        (Some(l), _) => l.to_string(),
                        (None, Some(a)) => format!("`{a}`"),
                        (None, None) => pat.to_js(),
                    };
                    self.miss(p, what);
                }
                return None;
            }
        };
        if !ctx.in_token {
            nodes.push(leaf(&self.src[p..end], false, p, end, field));
        }
        Some((nodes, end))
    }

    // ---- rules ------------------------------------------------------

    /// Match `r` at `pos`. `field` is the name the nodes matched take;
    /// `scope` is the enclosing sequences' fields, for diagnostics.
    fn rule(
        &mut self,
        r: &Rule,
        pos: usize,
        ctx: Ctx,
        field: Option<&str>,
        scope: Option<&Scope>,
    ) -> Outcome {
        let r_outer = r;
        Ok(match r {
            Rule::Blank => Some((Vec::new(), pos)),
            Rule::Str(s) => self.literal(s, pos, false, ctx, field),
            Rule::Pat(p) => self.pattern(p, pos, false, ctx, field, None),
            Rule::Sym(name) => return self.sym(name, pos, ctx, field, scope),
            Rule::Without(name, nodes, _) => {
                let ex = self.intern(nodes);
                let inner = Ctx { exclude: ex, ..ctx };
                return self.sym(name, pos, inner, field, scope);
            }
            Rule::Seq(rs) => {
                let mut nodes: Vec<Node> = Vec::new();
                let mut p = pos;
                for r in rs {
                    let inner = Scope {
                        nodes: &nodes,
                        parent: scope,
                    };
                    match self.rule(r, p, ctx, field, Some(&inner))? {
                        Some((ns, e)) => {
                            nodes.extend(ns);
                            p = e;
                        }
                        None => return Ok(None),
                    }
                }
                Some((nodes, p))
            }
            Rule::Choice(rs) => {
                for r in rs {
                    if let Some(hit) = self.rule(r, pos, ctx, field, scope)? {
                        return Ok(Some(hit));
                    }
                }
                None
            }
            Rule::Repeat(r) => self.repeat(r, pos, ctx, field, scope, 0)?,
            Rule::Repeat1(r) => self.repeat(r, pos, ctx, field, scope, 1)?,
            Rule::Optional(r) => Some(
                self.rule(r, pos, ctx, field, scope)?
                    .unwrap_or((Vec::new(), pos)),
            ),
            Rule::Prec(_, r)
            | Rule::PrecLeft(_, r)
            | Rule::PrecRight(_, r)
            | Rule::PrecDynamic(_, r) => return self.rule(r, pos, ctx, field, scope),
            Rule::Token(r) => {
                let (mut nodes, p) = if ctx.skips() {
                    self.take_extras(pos)
                } else {
                    (Vec::new(), pos)
                };
                let inner = Ctx {
                    in_token: true,
                    ..ctx
                };
                match self.rule(r, p, inner, None, scope)? {
                    Some((_, end)) => {
                        if !ctx.in_token {
                            nodes.push(leaf(&self.src[p..end], false, p, end, field));
                        }
                        Some((nodes, end))
                    }
                    None => None,
                }
            }
            Rule::Immediate(r) => match &**r {
                Rule::Str(s) => self.literal(s, pos, true, ctx, field),
                Rule::Pat(p) => self.pattern(p, pos, true, ctx, field, None),
                r => {
                    let inner = Ctx {
                        in_token: true,
                        ..ctx
                    };
                    match self.rule(r, pos, inner, None, scope)? {
                        Some((_, end)) => {
                            let nodes = if ctx.in_token {
                                Vec::new()
                            } else {
                                vec![leaf(&self.src[pos..end], false, pos, end, field)]
                            };
                            Some((nodes, end))
                        }
                        None => None,
                    }
                }
            },
            Rule::Field(name, r) => return self.rule(r, pos, ctx, Some(name), scope),
            Rule::Alias(r, name) => {
                let outer = self.alias.replace(name.clone());
                let hit = self.rule(r, pos, ctx, field, scope);
                self.alias = outer;
                match hit? {
                    Some((mut nodes, end)) => {
                        // the extras skipped before the token keep their names
                        for n in nodes.iter_mut().filter(|n| !n.extra) {
                            n.kind = name.clone();
                        }
                        Some((nodes, end))
                    }
                    None => None,
                }
            }
            Rule::NotAheadSoft(r) => {
                let recording = self.recording;
                let hard = self.hard.take();
                let furthest = self.furthest;
                let expected = std::mem::take(&mut self.expected);
                let diagnostics = self.diags.len();
                self.recording = false;
                let hit = self.rule(r, pos, ctx, None, scope);
                self.recording = recording;
                self.hard = hard;
                self.furthest = furthest;
                self.expected = expected;
                self.diags.truncate(diagnostics);
                match hit {
                    Ok(Some(_)) => None,
                    Ok(None) | Err(Hard) => Some((Vec::new(), pos)),
                }
            }
            Rule::NotAhead(r) | Rule::Ahead(r) => {
                let recording = self.recording;
                self.recording = false;
                let hit = self.rule(r, pos, ctx, None, scope);
                self.recording = recording;
                let wants = matches!(r_outer, Rule::Ahead(_));
                match hit?.is_some() == wants {
                    true => Some((Vec::new(), pos)),
                    false => None,
                }
            }
            Rule::Expect(r, label) => match self.rule(r, pos, ctx, field, scope)? {
                Some(hit) => Some(hit),
                None => {
                    let p = self.skip_extras(pos);
                    return self.fail_hard(p, None, *label, scope);
                }
            },
            Rule::Refuse(r, label) => {
                let recording = self.recording;
                self.recording = false;
                let hit = self.rule(r, pos, ctx, None, scope);
                self.recording = recording;
                match hit? {
                    Some((nodes, end)) => {
                        let p = self.skip_extras(pos);
                        let own = Scope {
                            nodes: &nodes,
                            parent: scope,
                        };
                        return self.fail_hard(p, Some(end.max(p)), *label, Some(&own));
                    }
                    None => Some((Vec::new(), pos)),
                }
            }
            Rule::Recover(r, boundary) => return self.recover(r, boundary, pos, ctx, field, scope),
        })
    }

    /// `r` as many times as it matches, at least `least` times; an
    /// iteration that consumes nothing ends the run.
    fn repeat(
        &mut self,
        r: &Rule,
        pos: usize,
        ctx: Ctx,
        field: Option<&str>,
        scope: Option<&Scope>,
        least: usize,
    ) -> Outcome {
        let mut nodes: Vec<Node> = Vec::new();
        let mut p = pos;
        let mut n = 0;
        while let Some((ns, e)) = self.rule(r, p, ctx, field, scope)? {
            if e == p {
                break;
            }
            nodes.extend(ns);
            p = e;
            n += 1;
        }
        if n < least {
            return Ok(None);
        }
        Ok(Some((nodes, p)))
    }

    fn intern(&mut self, nodes: &[String]) -> usize {
        match self.excludes.iter().position(|e| e == nodes) {
            Some(i) => i,
            None => {
                self.excludes.push(nodes.to_vec());
                self.excludes.len() - 1
            }
        }
    }

    /// A named rule at `pos`. The enclosing scope reaches in for the
    /// sake of diagnostics alone: a match is remembered by position and
    /// a diagnostic is never remembered, so the memo stays sound.
    fn sym(
        &mut self,
        name: &str,
        pos: usize,
        ctx: Ctx,
        field: Option<&str>,
        scope: Option<&Scope>,
    ) -> Outcome {
        if let Some(&ci) = self.op_nodes.get(name) {
            return self.climb(ci, pos, 0, ctx, Some(name), field);
        }
        let Some(&i) = self.index.get(name) else {
            panic!("treegram: the grammar names no rule `{name}`");
        };
        let def = &self.g.rules[i];
        match &def.body {
            Body::Climb(_) => self.climb(i, pos, 0, ctx, None, field),
            Body::Rule(r) => {
                // an ordinary rule starts a fresh exclusion scope
                let inner = Ctx {
                    in_token: ctx.in_token,
                    bare: ctx.bare,
                    exclude: 0,
                };
                let key = (
                    i,
                    pos,
                    0,
                    if ctx.in_token {
                        -2
                    } else if ctx.bare {
                        -3
                    } else {
                        -1
                    },
                );
                if let Some(hit) = self.memo.get(&key) {
                    return Ok(hit.clone().map(|(ns, e)| (with_field(ns, field), e)));
                }
                let label = match r {
                    Rule::Pat(_) | Rule::Token(_) | Rule::Immediate(_) => Some(def.name.as_str()),
                    _ => None,
                };
                let label_text = label.map(|l| l.trim_start_matches('_').replace('_', " "));
                let label = label_text.as_deref();
                let hit = match r {
                    Rule::Pat(p) => self.pattern(p, pos, false, inner, None, label),
                    Rule::Immediate(inner_rule) if matches!(**inner_rule, Rule::Pat(_)) => {
                        let Rule::Pat(p) = &**inner_rule else {
                            unreachable!()
                        };
                        self.pattern(p, pos, true, inner, None, label)
                    }
                    r => self.rule(r, pos, inner, None, scope)?,
                };
                let hit = hit.map(|(nodes, end)| {
                    let nodes = if ctx.in_token || is_hidden(&def.name) {
                        nodes
                    } else {
                        vec![wrap(&def.name, nodes, pos, end, self.src)]
                    };
                    (nodes, end)
                });
                self.memo.insert(key, hit.clone());
                Ok(hit.map(|(ns, e)| (with_field(ns, field), e)))
            }
        }
    }

    /// Precedence climbing over the climb at `ci`, allowing operators
    /// of precedence `min` and above. `rooted` names the operator node
    /// the answer must be, for a reference from outside the climb.
    fn climb(
        &mut self,
        ci: usize,
        pos: usize,
        min: i32,
        ctx: Ctx,
        rooted: Option<&str>,
        field: Option<&str>,
    ) -> Outcome {
        let variant = match rooted {
            None => min as i64,
            Some(r) => -1000 - self.op_index(ci, r) as i64,
        };
        let key = (ci, pos, ctx.exclude, variant);
        if let Some(hit) = self.memo.get(&key) {
            return Ok(hit.clone().map(|(ns, e)| (with_field(ns, field), e)));
        }
        let hit = self.climb_uncached(ci, pos, min, ctx, rooted)?;
        self.memo.insert(key, hit.clone());
        Ok(hit.map(|(ns, e)| (with_field(ns, field), e)))
    }

    fn op_index(&self, ci: usize, node: &str) -> usize {
        let Body::Climb(c) = &self.g.rules[ci].body else {
            unreachable!()
        };
        c.ops
            .iter()
            .position(|o| o.node == node)
            .expect("an operator node")
    }

    fn excluded(&self, ctx: Ctx, node: &str) -> bool {
        self.excludes[ctx.exclude].iter().any(|n| n == node)
    }

    fn climb_uncached(
        &mut self,
        ci: usize,
        pos: usize,
        min: i32,
        ctx: Ctx,
        rooted: Option<&str>,
    ) -> Outcome {
        let Body::Climb(c) = &self.g.rules[ci].body else {
            unreachable!()
        };
        let c: &'g Climb = c;
        let name = self.g.rules[ci].name.as_str();
        // the left operand: a prefix operator, or a primary
        let mut left: Option<(Vec<Node>, usize)> = None;
        for op in &c.ops {
            let Form::Prefix { head, operand } = &op.form else {
                continue;
            };
            if self.excluded(ctx, &op.node) {
                continue;
            }
            let Some((mut nodes, p)) = self.rule(head, pos, ctx, None, None)? else {
                continue;
            };
            // an operator that consumed nothing — a refusal that did
            // not fire — is no operator here, as for a postfix one
            if p == pos {
                continue;
            }
            let scope = Scope {
                nodes: &nodes,
                parent: None,
            };
            let _ = &scope;
            let Some((ons, end)) = self.climb(ci, p, op.prec, ctx, None, Some(operand))? else {
                continue;
            };
            nodes.extend(ons);
            left = Some((vec![wrap(&op.node, nodes, pos, end, self.src)], end));
            break;
        }
        let (mut nodes, mut p) = match left {
            Some(l) => l,
            None => match self.rule(&c.primary, pos, ctx, None, None)? {
                Some(l) => l,
                None => return Ok(None),
            },
        };
        let mut best: Option<(Vec<Node>, usize)> = None;
        loop {
            if let Some(r) = rooted {
                if nodes.len() == 1 && nodes[0].kind == r {
                    best = Some((nodes.clone(), p));
                }
            }
            let mut applied = false;
            for op in &c.ops {
                if op.prec < min || self.excluded(ctx, &op.node) {
                    continue;
                }
                match &op.form {
                    Form::Prefix { .. } => {}
                    Form::Infix { left, op: o, right } => {
                        let Some((ons, p1)) = self.rule(o, p, ctx, None, None)? else {
                            continue;
                        };
                        let rmin = if op.assoc == Assoc::Right {
                            op.prec
                        } else {
                            op.prec + 1
                        };
                        let Some((rns, p2)) = self.climb(ci, p1, rmin, ctx, None, Some(right))?
                        else {
                            continue;
                        };
                        if p2 == p {
                            continue;
                        }
                        let mut children = with_field(std::mem::take(&mut nodes), Some(left));
                        children.extend(ons);
                        children.extend(rns);
                        nodes = vec![wrap(&op.node, children, pos, p2, self.src)];
                        p = p2;
                        applied = true;
                        break;
                    }
                    Form::Postfix {
                        target,
                        tail,
                        targets,
                    } => {
                        if let Some(ks) = targets {
                            if !(nodes.len() == 1 && ks.contains(&nodes[0].kind)) {
                                continue;
                            }
                        }
                        let scope = Scope {
                            nodes: &nodes,
                            parent: None,
                        };
                        let Some((tns, p1)) = self.rule(tail, p, ctx, None, Some(&scope))? else {
                            continue;
                        };
                        // an operator that consumed nothing — a refusal
                        // that did not fire — is no operator here
                        if p1 == p {
                            continue;
                        }
                        let mut children = with_field(std::mem::take(&mut nodes), Some(target));
                        children.extend(tns);
                        nodes = vec![wrap(&op.node, children, pos, p1, self.src)];
                        p = p1;
                        applied = true;
                        break;
                    }
                }
            }
            if !applied {
                break;
            }
        }
        let _ = name;
        Ok(match rooted {
            Some(_) => best,
            None => Some((nodes, p)),
        })
    }

    /// A recovery region: the rule, or an `ERROR` node to the boundary.
    fn recover(
        &mut self,
        r: &Rule,
        boundary: &Rule,
        pos: usize,
        ctx: Ctx,
        field: Option<&str>,
        scope: Option<&Scope>,
    ) -> Outcome {
        let start = self.skip_extras(pos);
        let outcome = self.rule(r, pos, ctx, field, scope);
        let diag = match outcome {
            Ok(Some(hit)) => return Ok(Some(hit)),
            Ok(None) => self.soft_diag(),
            Err(Hard) => {
                let hard = self
                    .hard
                    .take()
                    .expect("a hard failure states its diagnostic");
                self.furthest_of(hard)
            }
        };
        if start >= self.src.len() {
            return Ok(None);
        }
        let end = self.boundary(start, boundary, ctx);
        self.diags.push(diag);
        // the failure is reported; what was looked for inside the
        // region is no longer what the parse is looking for
        self.furthest = end;
        self.expected.clear();
        Ok(Some((vec![error_node(start, end)], end)))
    }

    /// The position of the next boundary at bracket depth zero after
    /// `start`, consuming at least one token, or the end of input.
    fn boundary(&mut self, start: usize, boundary: &Rule, ctx: Ctx) -> usize {
        let recording = self.recording;
        self.recording = false;
        let token_ctx = Ctx {
            in_token: true,
            bare: false,
            exclude: 0,
        };
        let mut depth = 0i32;
        let mut p = start;
        let mut first = true;
        while p < self.src.len() {
            p = self.skip_extras(p);
            if p >= self.src.len() {
                break;
            }
            if depth <= 0 && !first {
                if let Ok(Some(_)) = self.rule(boundary, p, ctx, None, None) {
                    break;
                }
            }
            first = false;
            // the longest token the lexicon knows here
            let mut best = p;
            for (open, close) in &self.g.brackets {
                if self.src[p..].starts_with(open.as_str()) {
                    depth += 1;
                    best = best.max(p + open.len());
                } else if self.src[p..].starts_with(close.as_str()) {
                    depth -= 1;
                    best = best.max(p + close.len());
                }
            }
            if best == p {
                for d in &self.g.rules {
                    let Body::Rule(r) = &d.body else { continue };
                    if !matches!(r, Rule::Pat(_) | Rule::Token(_) | Rule::Immediate(_)) {
                        continue;
                    }
                    if let Ok(Some((_, e))) = self.rule(r, p, token_ctx, None, None) {
                        best = best.max(e);
                    }
                }
            }
            if best == p {
                best = p + self.src[p..].chars().next().map_or(1, char::len_utf8);
            }
            p = best;
        }
        self.recording = recording;
        p.min(self.src.len())
    }
}

fn leaf(kind: &str, named: bool, start: usize, end: usize, field: Option<&str>) -> Node {
    Node {
        kind: kind.to_string(),
        named,
        field: field.map(str::to_string),
        extra: false,
        start,
        end,
        children: Vec::new(),
    }
}

/// A named node over its children.
fn wrap(kind: &str, children: Vec<Node>, pos: usize, end: usize, src: &str) -> Node {
    let start = children.first().map_or(pos, |c| c.start);
    let end = children.last().map_or(end, |c| c.end);
    let _ = src;
    Node {
        kind: kind.to_string(),
        named: true,
        field: None,
        extra: false,
        start,
        end,
        children,
    }
}

/// The nodes with a field name set where they have none.
fn with_field(mut nodes: Vec<Node>, field: Option<&str>) -> Vec<Node> {
    if let Some(f) = field {
        for n in &mut nodes {
            if n.field.is_none() {
                n.field = Some(f.to_string());
            }
        }
    }
    nodes
}
