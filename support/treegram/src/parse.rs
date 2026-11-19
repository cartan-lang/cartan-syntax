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
//!
//! **A parse is given the depth its tree may reach**, and nesting beyond
//! it is a diagnostic rather than a stack overflow. Two checks state it.
//! Every node is built with its height, and a node taller than the depth
//! is refused where it is built, which bounds the tree a parse returns —
//! a long chain of a left-grouping operator included, which the parser
//! reads in a loop. And the parse counts the rules it is reading that
//! start further into the text than the one around them, each a level of
//! the text's nesting, and refuses to read a level past the depth, which
//! bounds the parser's own recursion where no node is built, as in a run
//! of brackets that never closes. A tree within the depth never meets the
//! second check, since every such level holds a node of the tree. The
//! refusal is final: no lookahead or alternative reads past it, and the
//! item it stands in becomes an `ERROR` node.

use std::collections::{HashMap, HashSet};
use std::hash::{BuildHasherDefault, Hasher};

use crate::pattern::Pat;
use crate::rule::{is_hidden, Assoc, Body, Climb, Form, Grammar, Label, Op, Rule};
use crate::tree::{Diag, Field, Node, Opener, Said};

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

/// Parse `src` against the rule `root` of `g`, building no tree deeper
/// than `limit` nodes from the root to a token ([`Node::height`]); text
/// that nests deeper is [`Said::TooDeep`].
pub fn parse(g: &Grammar, src: &str, root: &str, limit: usize) -> Parse {
    let mut e = Engine::new(g, src, limit);
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
    // **The extras after the root's match are the root's**, as an extra
    // the root's own last token skipped would have been: a comment ending
    // the text with no newline after it stands inside the root node rather
    // than beside it, where it would make the root a second node around the
    // first.
    let (trailing, rest) = e.take_extras(end);
    match children.as_mut_slice() {
        [node] if node.named && node.kind == root => {
            if let Some(last) = trailing.last() {
                node.end = node.end.max(last.end);
            }
            node.height = trailing
                .iter()
                .fold(node.height, |height, extra| height.max(extra.height + 1));
            node.children.extend(trailing);
        }
        _ => children.extend(trailing),
    }
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
    let mut root = if children.len() == 1 && children[0].named && children[0].kind == root {
        children.pop().unwrap()
    } else {
        let mut node = wrap(root, children, 0, src.len());
        node.start = 0;
        node.end = src.len();
        node
    };
    if root.height > limit {
        diags.push(e.too_deep_in(&root));
        root.children = vec![error_node(0, src.len())];
        root.height = 2;
    }
    diags.sort_by_key(|d| d.pos);
    Parse { root, diags }
}

/// An ordinary rule's memo key: no exclusion is in force, and the variant
/// says whether it is read inside a token or as an extra.
fn named_key(i: usize, pos: usize, ctx: Ctx) -> MemoKey {
    let variant = if ctx.in_token {
        -2
    } else if ctx.bare {
        -3
    } else {
        -1
    };
    (i, pos, 0, variant)
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
        height: 1,
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

/// (rule, position, exclusion, variant): what a remembered match is
/// keyed by ([`Engine::memo`]).
type MemoKey = (usize, usize, usize, i64);

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
    /// (rule, position, exclusion, variant) → outcome, and whether the
    /// parse was recording the tokens it missed when it reached it. A
    /// parse ends with about one and a half entries per byte of the text
    /// it read, so the table is asked for that room at the outset rather
    /// than doubled and rehashed a dozen times on the way there.
    memo: FxMap<MemoKey, (Soft, bool)>,
    /// Position → the position after the extras there, for the
    /// positions whose extras hold no named node ([`Engine::take_extras`]).
    extras_at: FxMap<usize, usize>,
    /// The furthest position a token was looked for and not found, and
    /// what was looked for there.
    furthest: usize,
    expected: Vec<String>,
    /// The brackets open on the current parse path, innermost last. A
    /// sequence that opens one closes it or drops it on leaving, so a
    /// remembered rule's match leaves this as it found it.
    open: Vec<Opener>,
    /// The innermost bracket whose closer was looked for at the furthest
    /// position ([`Diag::unclosed`]).
    unclosed: Option<Opener>,
    /// The named, unhidden rules being read, innermost last.
    names: Vec<&'g str>,
    /// Whether failures are being recorded (off inside lookahead).
    recording: bool,
    hard: Option<HardDiag>,
    diags: Vec<Diag>,
    /// The alias in force, which names an aliased token in a diagnostic.
    alias: Option<String>,
    /// The depth a tree may reach ([`parse`]).
    limit: usize,
    /// The levels of nesting being read: the rules on the parse path that
    /// start further into the text than the level around them
    /// ([`Engine::nest`]).
    levels: usize,
    /// Where the innermost level being read starts.
    level_at: usize,
    /// The text nests deeper than `limit`: the hard failure stating it is
    /// final, read past by no lookahead, until the recovery region around
    /// it reports it.
    too_deep: bool,
}

impl<'g, 's> Engine<'g, 's> {
    fn new(g: &'g Grammar, src: &'s str, limit: usize) -> Self {
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
            open: Vec::new(),
            unclosed: None,
            names: Vec::new(),
            recording: true,
            hard: None,
            diags: Vec::new(),
            alias: None,
            limit,
            levels: 0,
            level_at: 0,
            too_deep: false,
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
            self.unclosed = None;
        }
        if pos != self.furthest {
            return;
        }
        if let Some(open) = self.open.last() {
            let closes =
                what.strip_prefix('`').and_then(|w| w.strip_suffix('`')) == Some(&open.close);
            if closes && self.unclosed.as_ref().is_none_or(|u| u.start < open.start) {
                self.unclosed = Some(open.clone());
            }
        }
        if !self.expected.contains(&what) {
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
            unclosed: None,
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
        let mut diag = self.stuck_at(pos, None, incomplete, Said::Expected(self.expected.clone()));
        diag.unclosed = self.unclosed.clone();
        diag
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

    /// Text nesting deeper than [`Engine::limit`], stated at `pos` about
    /// the text to `end`.
    fn deep_diag(&self, pos: usize, end: Option<usize>) -> Diag {
        self.stuck_at(pos, end, false, Said::TooDeep { limit: self.limit })
    }

    /// The diagnostic for `node`, a node taller than [`Engine::limit`]: it
    /// stands at the first token below that depth.
    fn too_deep_in(&self, node: &Node) -> Diag {
        let token = below(node, self.limit);
        self.deep_diag(token.start, Some(token.end))
    }

    /// A refusal of depth: a hard failure that is final
    /// ([`Engine::too_deep`]).
    fn fail_deep(&mut self, diag: Diag) -> Hard {
        self.hard = Some(HardDiag {
            diag,
            outranked: false,
        });
        self.too_deep = true;
        Hard
    }

    /// A named node over `children`, refused where it would be taller than
    /// [`Engine::limit`].
    fn node(
        &mut self,
        kind: &str,
        children: Vec<Node>,
        pos: usize,
        end: usize,
    ) -> Result<Node, Hard> {
        let node = wrap(kind, children, pos, end);
        if node.height <= self.limit {
            return Ok(node);
        }
        let diag = self.too_deep_in(&node);
        Err(self.fail_deep(diag))
    }

    /// Enter a rule starting at `pos`: a level of nesting where it starts
    /// further into the text than the innermost level being read, refused
    /// past [`Engine::limit`]. The level to restore on leaving is returned.
    fn nest(&mut self, pos: usize) -> Result<Option<usize>, Hard> {
        if self.levels > 0 && pos <= self.level_at {
            return Ok(None);
        }
        if self.levels == self.limit {
            let p = self.skip_extras(pos);
            let diag = self.deep_diag(p, None);
            return Err(self.fail_deep(diag));
        }
        self.levels += 1;
        Ok(Some(std::mem::replace(&mut self.level_at, pos)))
    }

    /// Leave a rule [`Engine::nest`] entered.
    fn unnest(&mut self, level: Option<usize>) {
        if let Some(at) = level {
            self.levels -= 1;
            self.level_at = at;
        }
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
    ///
    /// **Every level of a document's nesting passes through here many
    /// times**, so this frame is kept to a dispatch: each case that holds
    /// locals of its own is a method of its own, whose frame stands on the
    /// stack only while that case is being read, and the wrappers that
    /// change nothing but the field name are peeled in a loop rather than
    /// recursed through.
    fn rule<'r>(
        &mut self,
        mut r: &'r Rule,
        pos: usize,
        ctx: Ctx,
        mut field: Option<&'r str>,
        scope: Option<&Scope>,
    ) -> Outcome {
        loop {
            match r {
                Rule::Prec(_, inner)
                | Rule::PrecLeft(_, inner)
                | Rule::PrecRight(_, inner)
                | Rule::PrecDynamic(_, inner) => r = inner,
                Rule::Field(name, inner) => {
                    field = Some(name);
                    r = inner;
                }
                _ => break,
            }
        }
        match r {
            Rule::Sym(name) => self.sym(name, pos, ctx, field, scope),
            Rule::Seq(rs) => self.seq(rs, pos, ctx, field, scope),
            Rule::Choice(rs) => self.choice(rs, pos, ctx, field, scope),
            Rule::Optional(r) => self.optional(r, pos, ctx, field, scope),
            Rule::Repeat(r) => self.repeat(r, pos, ctx, field, scope, 0),
            Rule::Repeat1(r) => self.repeat(r, pos, ctx, field, scope, 1),
            r => self.rule_rest(r, pos, ctx, field, scope),
        }
    }

    /// The cases [`Engine::rule`] leaves to a dispatch of their own, which
    /// stands on the stack only while one of them is read: the tokens, the
    /// lookaheads, the stated diagnostics and the rest.
    fn rule_rest(
        &mut self,
        r: &Rule,
        pos: usize,
        ctx: Ctx,
        field: Option<&str>,
        scope: Option<&Scope>,
    ) -> Outcome {
        match r {
            Rule::Blank | Rule::Lookahead(_) => Ok(Some((Vec::new(), pos))),
            Rule::Str(s) => Ok(self.literal(s, pos, false, ctx, field)),
            Rule::Pat(p) => Ok(self.pattern(p, pos, false, ctx, field, None)),
            Rule::Without(ex) => self.without(&ex.expr, &ex.excluded, pos, ctx, field, scope),
            Rule::Token(r) => self.token(r, pos, ctx, field, scope),
            Rule::Immediate(r) => self.immediate(r, pos, ctx, field, scope),
            Rule::Alias(r, name) => self.alias(r, name, pos, ctx, field, scope),
            Rule::NotAheadSoft(r) => self.not_ahead_soft(r, pos, ctx, scope),
            Rule::NotAhead(r) => self.ahead(r, false, pos, ctx, scope),
            Rule::Ahead(r) => self.ahead(r, true, pos, ctx, scope),
            Rule::Expect(r, label) => self.expect(r, *label, pos, ctx, field, scope),
            Rule::Refuse(r, label) => self.refuse(r, *label, pos, ctx, scope),
            Rule::Recover(r, boundary) => self.recover(r, boundary, pos, ctx, field, scope),
            Rule::Sym(_)
            | Rule::Seq(_)
            | Rule::Choice(_)
            | Rule::Optional(_)
            | Rule::Repeat(_)
            | Rule::Repeat1(_)
            | Rule::Prec(..)
            | Rule::PrecLeft(..)
            | Rule::PrecRight(..)
            | Rule::PrecDynamic(..)
            | Rule::Field(..) => unreachable!("read by Engine::rule"),
        }
    }

    fn without(
        &mut self,
        name: &str,
        nodes: &[String],
        pos: usize,
        ctx: Ctx,
        field: Option<&str>,
        scope: Option<&Scope>,
    ) -> Outcome {
        let ex = self.intern(nodes);
        let inner = Ctx { exclude: ex, ..ctx };
        self.sym(name, pos, inner, field, scope)
    }

    /// The first alternative that matches.
    fn choice(
        &mut self,
        rs: &[Rule],
        pos: usize,
        ctx: Ctx,
        field: Option<&str>,
        scope: Option<&Scope>,
    ) -> Outcome {
        for r in rs {
            match self.rule(r, pos, ctx, field, scope) {
                Ok(None) => {}
                decided => return decided,
            }
        }
        Ok(None)
    }

    fn optional(
        &mut self,
        r: &Rule,
        pos: usize,
        ctx: Ctx,
        field: Option<&str>,
        scope: Option<&Scope>,
    ) -> Outcome {
        match self.rule(r, pos, ctx, field, scope) {
            Ok(None) => Ok(Some((Vec::new(), pos))),
            decided => decided,
        }
    }

    fn token(
        &mut self,
        r: &Rule,
        pos: usize,
        ctx: Ctx,
        field: Option<&str>,
        scope: Option<&Scope>,
    ) -> Outcome {
        let (mut nodes, p) = if ctx.skips() {
            self.take_extras(pos)
        } else {
            (Vec::new(), pos)
        };
        let inner = Ctx {
            in_token: true,
            ..ctx
        };
        Ok(match self.rule(r, p, inner, None, scope)? {
            Some((_, end)) => {
                if !ctx.in_token {
                    nodes.push(leaf(&self.src[p..end], false, p, end, field));
                }
                Some((nodes, end))
            }
            None => None,
        })
    }

    fn immediate(
        &mut self,
        r: &Rule,
        pos: usize,
        ctx: Ctx,
        field: Option<&str>,
        scope: Option<&Scope>,
    ) -> Outcome {
        Ok(match r {
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
        })
    }

    fn alias(
        &mut self,
        r: &Rule,
        name: &str,
        pos: usize,
        ctx: Ctx,
        field: Option<&str>,
        scope: Option<&Scope>,
    ) -> Outcome {
        let outer = self.alias.replace(name.to_string());
        let hit = self.rule(r, pos, ctx, field, scope);
        self.alias = outer;
        Ok(match hit? {
            Some((mut nodes, end)) => {
                // the extras skipped before the token keep their names
                for n in nodes.iter_mut().filter(|n| !n.extra) {
                    n.kind = name.to_string();
                }
                Some((nodes, end))
            }
            None => None,
        })
    }

    fn not_ahead_soft(&mut self, r: &Rule, pos: usize, ctx: Ctx, scope: Option<&Scope>) -> Outcome {
        let recording = self.recording;
        let hard = self.hard.take();
        let furthest = self.furthest;
        let expected = std::mem::take(&mut self.expected);
        let unclosed = self.unclosed.take();
        let diagnostics = self.diags.len();
        self.recording = false;
        let hit = self.rule(r, pos, ctx, None, scope);
        self.recording = recording;
        if self.too_deep {
            // final: the probe's own refusal stands, and no reading of the
            // text past it is tried
            return Err(Hard);
        }
        self.hard = hard;
        self.furthest = furthest;
        self.expected = expected;
        self.unclosed = unclosed;
        self.diags.truncate(diagnostics);
        Ok(match hit {
            Ok(Some(_)) => None,
            Ok(None) | Err(Hard) => Some((Vec::new(), pos)),
        })
    }

    /// Lookahead: succeeds, consuming nothing, where `r` matching here is
    /// what `wants` says.
    fn ahead(
        &mut self,
        r: &Rule,
        wants: bool,
        pos: usize,
        ctx: Ctx,
        scope: Option<&Scope>,
    ) -> Outcome {
        let recording = self.recording;
        self.recording = false;
        let hit = self.rule(r, pos, ctx, None, scope);
        self.recording = recording;
        Ok(match hit?.is_some() == wants {
            true => Some((Vec::new(), pos)),
            false => None,
        })
    }

    fn expect(
        &mut self,
        r: &Rule,
        label: Label,
        pos: usize,
        ctx: Ctx,
        field: Option<&str>,
        scope: Option<&Scope>,
    ) -> Outcome {
        match self.rule(r, pos, ctx, field, scope)? {
            Some(hit) => Ok(Some(hit)),
            None => {
                let p = self.skip_extras(pos);
                self.fail_hard(p, None, label, scope)
            }
        }
    }

    fn refuse(
        &mut self,
        r: &Rule,
        label: Label,
        pos: usize,
        ctx: Ctx,
        scope: Option<&Scope>,
    ) -> Outcome {
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
                self.fail_hard(p, Some(end.max(p)), label, Some(&own))
            }
            None => Ok(Some((Vec::new(), pos))),
        }
    }

    /// A sequence's members in order, tracking the grammar's brackets
    /// it opens and closes ([`Engine::open`]) and dropping, on leaving,
    /// any it left open.
    fn seq(
        &mut self,
        rs: &[Rule],
        pos: usize,
        ctx: Ctx,
        field: Option<&str>,
        scope: Option<&Scope>,
    ) -> Outcome {
        let depth = self.open.len();
        let mut nodes: Vec<Node> = Vec::new();
        let mut p = pos;
        for r in rs {
            let inner = Scope {
                nodes: &nodes,
                parent: scope,
            };
            let outcome = self.rule(r, p, ctx, field, Some(&inner));
            match self.member(r, outcome, &mut nodes) {
                Some(Ok(e)) => p = e,
                missed => {
                    self.open.truncate(depth);
                    return missed.transpose().map(|_| None);
                }
            }
        }
        self.open.truncate(depth);
        Ok(Some((nodes, p)))
    }

    /// A sequence's member `r` matched as `outcome`: its nodes appended to
    /// `nodes` and the bracket it opens or closes tracked, and the position
    /// after it; `None` where it did not match.
    fn member(
        &mut self,
        r: &Rule,
        outcome: Outcome,
        nodes: &mut Vec<Node>,
    ) -> Option<Result<usize, Hard>> {
        match outcome {
            Ok(Some((ns, e))) => {
                nodes.extend(ns);
                if let Some(s) = bracket_literal(r) {
                    self.track_bracket(s, e);
                }
                Some(Ok(e))
            }
            Ok(None) => None,
            Err(hard) => Some(Err(hard)),
        }
    }

    /// Opens or closes a bracket the sequence just matched, ending at `end`.
    fn track_bracket(&mut self, s: &str, end: usize) {
        if self.open.last().is_some_and(|open| open.close == s) {
            self.open.pop();
        } else if let Some((open, close)) = self.g.brackets.iter().find(|(open, _)| open == s) {
            self.open.push(Opener {
                start: end - open.len(),
                end,
                text: open.clone(),
                close: close.clone(),
                rule: self.names.last().map(|name| name.to_string()),
            });
        }
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
        let g = self.g;
        match &g.rules[i].body {
            Body::Climb(_) => self.climb(i, pos, 0, ctx, None, field),
            Body::Rule(r) => self.named(i, r, pos, ctx, field, scope),
        }
    }

    /// The ordinary rule `i`, whose body is `r`, at `pos`: its match
    /// wrapped in its node unless it is hidden, and remembered.
    fn named(
        &mut self,
        i: usize,
        r: &'g Rule,
        pos: usize,
        ctx: Ctx,
        field: Option<&str>,
        scope: Option<&Scope>,
    ) -> Outcome {
        let key = named_key(i, pos, ctx);
        if let Some(hit) = self.recall(&key, field) {
            return Ok(hit);
        }
        // an ordinary rule starts a fresh exclusion scope
        let inner = Ctx {
            in_token: ctx.in_token,
            bare: ctx.bare,
            exclude: 0,
        };
        match self.named_body(i, r, pos, inner, scope) {
            Ok(hit) => self.named_node(key, hit, ctx, field),
            failed => failed,
        }
    }

    /// A named rule's match wrapped in its node unless it is hidden or read
    /// inside a token, remembered at `key` and returned with `field` set.
    fn named_node(&mut self, key: MemoKey, hit: Soft, ctx: Ctx, field: Option<&str>) -> Outcome {
        let name = self.g.rules[key.0].name.as_str();
        let hit = match hit {
            Some((nodes, end)) if !ctx.in_token && !is_hidden(name) => {
                Some((vec![self.node(name, nodes, key.1, end)?], end))
            }
            hit => hit,
        };
        Ok(self.store(key, hit, field))
    }

    /// A pattern token, which a diagnostic names by the rule `i`'s name.
    fn named_pattern(&mut self, i: usize, p: &Pat, pos: usize, immediate: bool, ctx: Ctx) -> Soft {
        let label = self.g.rules[i]
            .name
            .trim_start_matches('_')
            .replace('_', " ");
        self.pattern(p, pos, immediate, ctx, None, Some(&label))
    }

    /// A rule's body: one pattern token, which a diagnostic names by the
    /// rule's own name, or a rule read with the rule standing among
    /// [`Engine::names`] unless it is hidden.
    fn named_body(
        &mut self,
        i: usize,
        r: &Rule,
        pos: usize,
        ctx: Ctx,
        scope: Option<&Scope>,
    ) -> Outcome {
        match r {
            Rule::Pat(p) => return Ok(self.named_pattern(i, p, pos, false, ctx)),
            Rule::Immediate(inner) => {
                if let Rule::Pat(p) = &**inner {
                    return Ok(self.named_pattern(i, p, pos, true, ctx));
                }
            }
            _ => {}
        }
        // a token's rule reads no level of nesting
        let level = match r {
            Rule::Token(_) | Rule::Immediate(_) => None,
            _ => self.nest(pos)?,
        };
        let name = self.g.rules[i].name.as_str();
        let named = !is_hidden(name);
        if named {
            self.names.push(name);
        }
        let outcome = self.rule(r, pos, ctx, None, scope);
        if named {
            self.names.pop();
        }
        self.unnest(level);
        outcome
    }

    /// The remembered match at `key`, with `field` set. A miss a probe
    /// remembered recorded none of the tokens it looked for, so a parse
    /// that records reads the rule again rather than lose how far the
    /// miss reached.
    fn recall(&self, key: &MemoKey, field: Option<&str>) -> Option<Soft> {
        let (hit, recorded) = self.memo.get(key)?;
        if hit.is_none() && !recorded && self.recording {
            return None;
        }
        Some(hit.clone().map(|(ns, e)| (with_field(ns, field), e)))
    }

    /// Remember `hit` at `key`, and return it with `field` set.
    fn store(&mut self, key: MemoKey, hit: Soft, field: Option<&str>) -> Soft {
        self.memo.insert(key, (hit.clone(), self.recording));
        hit.map(|(ns, e)| (with_field(ns, field), e))
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
        let key = (ci, pos, ctx.exclude, self.climb_variant(ci, min, rooted));
        if let Some(hit) = self.recall(&key, field) {
            return Ok(hit);
        }
        let level = self.nest(pos)?;
        let outcome = self.climb_uncached(ci, pos, min, ctx, rooted);
        self.unnest(level);
        match outcome {
            Ok(hit) => Ok(self.store(key, hit, field)),
            failed => failed,
        }
    }

    /// A climb's memo variant: the least precedence allowed, or the
    /// operator node a reference from outside the climb asks for.
    fn climb_variant(&self, ci: usize, min: i32, rooted: Option<&str>) -> i64 {
        match rooted {
            None => min as i64,
            Some(r) => -1000 - self.op_index(ci, r) as i64,
        }
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

    /// A left operand — a prefix operator's or a primary — and then the
    /// infix and postfix operators that apply after it, left to right.
    fn climb_uncached(
        &mut self,
        ci: usize,
        pos: usize,
        min: i32,
        ctx: Ctx,
        rooted: Option<&str>,
    ) -> Outcome {
        let g = self.g;
        let Body::Climb(c) = &g.rules[ci].body else {
            unreachable!()
        };
        let left = match self.prefix(c, ci, pos, ctx)? {
            Some(left) => left,
            None => match self.rule(&c.primary, pos, ctx, None, None)? {
                Some(left) => left,
                None => return Ok(None),
            },
        };
        let (mut nodes, mut p) = left;
        let mut best: Option<(Vec<Node>, usize)> = None;
        loop {
            if let Some(r) = rooted {
                if nodes.len() == 1 && nodes[0].kind == r {
                    best = Some((nodes.clone(), p));
                }
            }
            match self.operator(c, ci, pos, p, min, ctx, &mut nodes)? {
                Some(end) => p = end,
                None => break,
            }
        }
        Ok(match rooted {
            Some(_) => best,
            None => Some((nodes, p)),
        })
    }

    /// The first prefix operator that applies at `pos`, with its operand.
    fn prefix(&mut self, c: &'g Climb, ci: usize, pos: usize, ctx: Ctx) -> Outcome {
        for op in &c.ops {
            let Form::Prefix {
                head,
                operand,
                root,
            } = &op.form
            else {
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
            let min = if root.is_some() { 0 } else { op.prec };
            let Some((ons, end)) = self.climb(ci, p, min, ctx, root.as_deref(), Some(operand))?
            else {
                continue;
            };
            nodes.extend(ons);
            return Ok(Some((vec![self.node(&op.node, nodes, pos, end)?], end)));
        }
        Ok(None)
    }

    /// One turn of the operator loop: the first infix or postfix operator
    /// of precedence `min` or above that applies at `p`, after the left
    /// operand `nodes` that starts at `start`, wraps it in place, and the
    /// position after it is returned. `None` where no operator applies.
    #[allow(clippy::too_many_arguments)]
    fn operator(
        &mut self,
        c: &'g Climb,
        ci: usize,
        start: usize,
        p: usize,
        min: i32,
        ctx: Ctx,
        nodes: &mut Vec<Node>,
    ) -> Result<Option<usize>, Hard> {
        for op in &c.ops {
            if op.prec < min || self.excluded(ctx, &op.node) {
                continue;
            }
            let applied = match &op.form {
                Form::Prefix { .. } => None,
                Form::Infix { .. } => self.infix(ci, op, start, p, ctx, nodes)?,
                Form::Postfix { .. } => self.postfix(op, start, p, ctx, nodes)?,
            };
            if applied.is_some() {
                return Ok(applied);
            }
        }
        Ok(None)
    }

    /// The infix operator `op` at `p` and its right operand.
    fn infix(
        &mut self,
        ci: usize,
        op: &Op,
        start: usize,
        p: usize,
        ctx: Ctx,
        nodes: &mut Vec<Node>,
    ) -> Result<Option<usize>, Hard> {
        let Form::Infix { left, op: o, right } = &op.form else {
            unreachable!()
        };
        let Some((ons, p1)) = self.rule(o, p, ctx, None, None)? else {
            return Ok(None);
        };
        let rmin = if op.assoc == Assoc::Right {
            op.prec
        } else {
            op.prec + 1
        };
        let Some((rns, p2)) = self.climb(ci, p1, rmin, ctx, None, Some(right))? else {
            return Ok(None);
        };
        if p2 == p {
            return Ok(None);
        }
        let mut children = with_field(std::mem::take(nodes), Some(left));
        children.extend(ons);
        children.extend(rns);
        *nodes = vec![self.node(&op.node, children, start, p2)?];
        Ok(Some(p2))
    }

    /// The postfix operator `op` at `p`, where it stands on `nodes`.
    fn postfix(
        &mut self,
        op: &Op,
        start: usize,
        p: usize,
        ctx: Ctx,
        nodes: &mut Vec<Node>,
    ) -> Result<Option<usize>, Hard> {
        let Form::Postfix {
            target,
            tail,
            targets,
        } = &op.form
        else {
            unreachable!()
        };
        if let Some(ks) = targets {
            if !(nodes.len() == 1 && ks.contains(&nodes[0].kind)) {
                return Ok(None);
            }
        }
        let scope = Scope {
            nodes,
            parent: None,
        };
        let Some((tns, p1)) = self.rule(tail, p, ctx, None, Some(&scope))? else {
            return Ok(None);
        };
        // an operator that consumed nothing — a refusal that did not
        // fire — is no operator here
        if p1 == p {
            return Ok(None);
        }
        let mut children = with_field(std::mem::take(nodes), Some(target));
        children.extend(tns);
        *nodes = vec![self.node(&op.node, children, start, p1)?];
        Ok(Some(p1))
    }

    /// A recovery region: the rule, or an `ERROR` node over the failed
    /// item ([`Engine::boundary`]).
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
                // a refusal of depth is final only up to the region it
                // stands in, which reports it; the next item is read anew
                self.too_deep = false;
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
        let end = self.boundary(start, diag.pos, boundary, ctx);
        self.diags.push(diag);
        // the failure is reported; what was looked for inside the
        // region is no longer what the parse is looking for
        self.furthest = end;
        self.expected.clear();
        self.unclosed = None;
        Ok(Some((vec![error_node(start, end)], end)))
    }

    /// **Where a failed item that starts at `start` ends**: at the first
    /// match of `boundary` — a line's end — that stands after the
    /// diagnostic at `stuck`, consuming at least one token, where either
    /// every bracket the item opened is closed, or the next line that
    /// holds a token starts no further in than the item and does not
    /// open with a closing bracket; or at the end of the text.
    ///
    /// The text the parse read before its diagnostic is the item's, the
    /// lines it continued onto included, so the region reaches at least
    /// that far. Brackets are counted, and the literals read whole, by the
    /// tokens that stand alone ([`Engine::lexical`]): a literal's contents
    /// are no brackets and no line ends, so a triple-quoted string left
    /// open runs to the end of the text, and a quoted one, which a line
    /// ends, ends there. A bracket left open would otherwise take the rest
    /// of the text, so a line that starts as far out as the item ends the
    /// item there; a closing bracket at the start of a line continues it.
    fn boundary(&mut self, start: usize, stuck: usize, boundary: &Rule, ctx: Ctx) -> usize {
        let recording = self.recording;
        self.recording = false;
        let column = start - self.src[..start].rfind('\n').map_or(0, |at| at + 1);
        let lexical = self.lexical();
        let mut depth = 0i32;
        let mut p = start;
        let mut first = true;
        while p < self.src.len() {
            p = self.skip_extras(p);
            if p >= self.src.len() {
                break;
            }
            if !first && p >= stuck {
                if let Ok(Some((_, next))) = self.rule(boundary, p, ctx, None, None) {
                    if depth <= 0 || self.outdented(next, column, boundary, ctx) {
                        break;
                    }
                }
            }
            first = false;
            p = self.lexical_token(p, &lexical, &mut depth);
        }
        self.recording = recording;
        p.min(self.src.len())
    }

    /// The position after the token at `p`, counting the brackets it
    /// opens or closes in `depth`: a bracket, else the longest of the
    /// standalone tokens `lexical` names, else one character.
    fn lexical_token(&mut self, p: usize, lexical: &[usize], depth: &mut i32) -> usize {
        for (open, close) in &self.g.brackets {
            if self.src[p..].starts_with(open.as_str()) {
                *depth += 1;
                return p + open.len();
            }
            if self.src[p..].starts_with(close.as_str()) {
                *depth -= 1;
                return p + close.len();
            }
        }
        let token_ctx = Ctx {
            in_token: true,
            bare: false,
            exclude: 0,
        };
        let mut best = p;
        for &i in lexical {
            let name = self.g.rules[i].name.as_str();
            if let Ok(Some((_, e))) = self.sym(name, p, token_ctx, None, None) {
                best = best.max(e);
            }
        }
        if best == p {
            best = p + self.src[p..].chars().next().map_or(1, char::len_utf8);
        }
        best
    }

    /// Whether the next line holding a token after the line end that ends
    /// at `next` starts no further in than `column` and opens with no
    /// closing bracket. Blank lines and lines of comments alone are passed
    /// over; the end of the text is no such line.
    fn outdented(&mut self, mut next: usize, column: usize, boundary: &Rule, ctx: Ctx) -> bool {
        loop {
            let at = self.skip_extras(next);
            if at >= self.src.len() {
                return false;
            }
            match self.rule(boundary, at, ctx, None, None) {
                Ok(Some((_, e))) => next = e,
                _ => {
                    let closes = self
                        .g
                        .brackets
                        .iter()
                        .any(|(_, close)| self.src[at..].starts_with(close.as_str()));
                    return at - next <= column && !closes;
                }
            }
        }
    }

    /// **The rules whose match is one token standing on its own**, which
    /// the recovery scan reads the text by: a rule built of literal and
    /// pattern tokens and of other such rules alone, a quoted literal
    /// among them, and not itself immediate. An immediate token only
    /// matches against what precedes it, as the text inside a literal
    /// does, and read anywhere else it reads too far.
    fn lexical(&self) -> Vec<usize> {
        fn tokens(g: &Grammar, r: &Rule, seen: &mut Vec<String>) -> bool {
            match r {
                Rule::Blank | Rule::Str(_) | Rule::Pat(_) => true,
                Rule::Seq(rs) | Rule::Choice(rs) => rs.iter().all(|r| tokens(g, r, seen)),
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
                | Rule::Alias(r, _) => tokens(g, r, seen),
                Rule::Sym(name) => {
                    if seen.contains(name) {
                        return false;
                    }
                    seen.push(name.clone());
                    matches!(g.def(name).map(|d| &d.body), Some(Body::Rule(r)) if tokens(g, r, seen))
                }
                _ => false,
            }
        }
        (0..self.g.rules.len())
            .filter(|&i| match &self.g.rules[i].body {
                Body::Rule(Rule::Immediate(_)) | Body::Climb(_) => false,
                Body::Rule(r) => tokens(self.g, r, &mut Vec::new()),
            })
            .collect()
    }
}

/// The literal a rule matches when it is one token, through the
/// wrappers that leave the token's text as written.
fn bracket_literal(r: &Rule) -> Option<&str> {
    match r {
        Rule::Str(s) => Some(s),
        Rule::Immediate(r) | Rule::Field(_, r) | Rule::Expect(r, _) => bracket_literal(r),
        _ => None,
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
        height: 1,
    }
}

/// A named node over its children.
fn wrap(kind: &str, children: Vec<Node>, pos: usize, end: usize) -> Node {
    let start = children.first().map_or(pos, |c| c.start);
    let end = children.last().map_or(end, |c| c.end);
    let height = 1 + children.iter().map(|c| c.height).max().unwrap_or(0);
    Node {
        kind: kind.to_string(),
        named: true,
        field: None,
        extra: false,
        start,
        end,
        children,
        height,
    }
}

/// The first token, in the text's order, `steps` nodes below `node` on a
/// longest path down from it.
fn below(mut node: &Node, steps: usize) -> &Node {
    for _ in 0..steps {
        match node.children.iter().find(|c| c.height + 1 == node.height) {
            Some(child) => node = child,
            None => break,
        }
    }
    node
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
