//! A token pattern, held as a small syntax tree rather than a regex
//! string, so that one value prints as the JavaScript regex a
//! `grammar.js` holds and matches directly here with no regex engine
//! behind it. The vocabulary is what a grammar's tokens need: a
//! character, any character, a class, sequence, alternation, and the
//! three repetitions.
//!
//! A pattern matches the way tree-sitter's lexer matches: the longest
//! text it can. [`Pat::longest`] returns that end, or `None` where the
//! pattern matches nothing at all at the position.

/// One entry of a character class.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClassItem {
    /// One character.
    Ch(char),
    /// A range, both ends included.
    Range(char, char),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Pat {
    /// Exactly this character.
    Ch(char),
    /// Any character but a newline, as `.` reads in a JavaScript regex.
    Any,
    /// A class, `[…]`, or its negation, `[^…]`.
    Class {
        negated: bool,
        items: Vec<ClassItem>,
    },
    Seq(Vec<Pat>),
    Alt(Vec<Pat>),
    Star(Box<Pat>),
    Plus(Box<Pat>),
    Opt(Box<Pat>),
}

/// Exactly this text, character by character.
pub fn text(s: &str) -> Pat {
    Pat::Seq(s.chars().map(Pat::Ch).collect())
}

/// A character class. Each entry is a character, or a range written
/// as a two-element string pair — `class(&["_"], &[('a', 'z'), ('A', 'Z')])`.
pub fn class(chars: &str, ranges: &[(char, char)]) -> Pat {
    Pat::Class {
        negated: false,
        items: class_items(chars, ranges),
    }
}

/// Every character but these.
pub fn not_class(chars: &str, ranges: &[(char, char)]) -> Pat {
    Pat::Class {
        negated: true,
        items: class_items(chars, ranges),
    }
}

fn class_items(chars: &str, ranges: &[(char, char)]) -> Vec<ClassItem> {
    let mut items: Vec<ClassItem> = chars.chars().map(ClassItem::Ch).collect();
    items.extend(ranges.iter().map(|(a, b)| ClassItem::Range(*a, *b)));
    items
}

pub fn seq(ps: Vec<Pat>) -> Pat {
    Pat::Seq(ps)
}

pub fn alt(ps: Vec<Pat>) -> Pat {
    Pat::Alt(ps)
}

pub fn star(p: Pat) -> Pat {
    Pat::Star(Box::new(p))
}

pub fn plus(p: Pat) -> Pat {
    Pat::Plus(Box::new(p))
}

pub fn opt(p: Pat) -> Pat {
    Pat::Opt(Box::new(p))
}

/// Whether `c` stands in the class's entries.
fn class_hit(items: &[ClassItem], c: char) -> bool {
    items.iter().any(|it| match it {
        ClassItem::Ch(x) => *x == c,
        ClassItem::Range(a, b) => *a <= c && c <= *b,
    })
}

/// A set of positions the match may have reached, held ascending and
/// without repeats in a `Vec`. The sets a token pattern works in hold
/// one or two positions, and a parse matches its patterns at every
/// position of the text, so the tree a `BTreeSet` allocates for such a
/// set costs more than the match it serves.
type Ends = Vec<usize>;

/// Sets [`Pat::ends`] has finished with, kept for the next match
/// rather than dropped and allocated again. The pool is handed down
/// the recursion, so one `longest` borrows it once.
type Pool = Vec<Ends>;

thread_local! {
    static POOL: std::cell::RefCell<Pool> = const { std::cell::RefCell::new(Vec::new()) };
}

/// An empty set to work in.
fn take(pool: &mut Pool) -> Ends {
    let mut v = pool.pop().unwrap_or_default();
    v.clear();
    v
}

fn give(pool: &mut Pool, v: Ends) {
    pool.push(v);
}

/// Add `x` to an ascending set, and say whether it was absent.
fn insert(v: &mut Ends, x: usize) -> bool {
    match v.binary_search(&x) {
        Ok(_) => false,
        Err(i) => {
            v.insert(i, x);
            true
        }
    }
}

/// `a` becomes `a` ∪ `b`, both ascending.
fn union(a: &mut Ends, b: &[usize]) {
    if a.is_empty() {
        a.extend_from_slice(b);
        return;
    }
    for &x in b {
        insert(a, x);
    }
}

impl Pat {
    /// The end of the longest match starting at byte `pos` of `s`, or
    /// `None` where nothing matches there. An empty match returns
    /// `Some(pos)`.
    pub fn longest(&self, s: &str, pos: usize) -> Option<usize> {
        // the three atoms answer with no set at all, which is most of
        // what a parse asks for
        match self {
            Pat::Ch(c) => return s[pos..].starts_with(*c).then(|| pos + c.len_utf8()),
            Pat::Any => {
                let c = s[pos..].chars().next().filter(|c| *c != '\n')?;
                return Some(pos + c.len_utf8());
            }
            Pat::Class { negated, items } => {
                let c = s[pos..].chars().next()?;
                return (class_hit(items, c) != *negated).then(|| pos + c.len_utf8());
            }
            _ => {}
        }
        POOL.with(|cell| {
            let pool = &mut *cell.borrow_mut();
            let mut out = take(pool);
            self.ends(s, pos, &mut out, pool);
            let last = out.last().copied();
            give(pool, out);
            last
        })
    }

    /// Every end a match starting at `pos` may have, written into
    /// `out`, which the caller passes empty.
    fn ends(&self, s: &str, pos: usize, out: &mut Ends, pool: &mut Pool) {
        match self {
            Pat::Ch(c) => {
                if s[pos..].starts_with(*c) {
                    out.push(pos + c.len_utf8());
                }
            }
            Pat::Any => {
                if let Some(c) = s[pos..].chars().next() {
                    if c != '\n' {
                        out.push(pos + c.len_utf8());
                    }
                }
            }
            Pat::Class { negated, items } => {
                if let Some(c) = s[pos..].chars().next() {
                    if class_hit(items, c) != *negated {
                        out.push(pos + c.len_utf8());
                    }
                }
            }
            Pat::Seq(ps) => {
                let mut frontier = take(pool);
                frontier.push(pos);
                let mut next = take(pool);
                let mut hit = take(pool);
                for p in ps {
                    next.clear();
                    for &q in &frontier {
                        hit.clear();
                        p.ends(s, q, &mut hit, pool);
                        union(&mut next, &hit);
                    }
                    std::mem::swap(&mut frontier, &mut next);
                    if frontier.is_empty() {
                        break;
                    }
                }
                out.extend_from_slice(&frontier);
                give(pool, frontier);
                give(pool, next);
                give(pool, hit);
            }
            Pat::Alt(ps) => {
                let mut hit = take(pool);
                for p in ps {
                    hit.clear();
                    p.ends(s, pos, &mut hit, pool);
                    union(out, &hit);
                }
                give(pool, hit);
            }
            Pat::Star(p) | Pat::Plus(p) => {
                let mut seen = take(pool);
                let mut frontier = take(pool);
                frontier.push(pos);
                let mut next = take(pool);
                let mut hit = take(pool);
                if matches!(self, Pat::Star(_)) {
                    out.push(pos);
                }
                while !frontier.is_empty() {
                    next.clear();
                    for &q in &frontier {
                        hit.clear();
                        p.ends(s, q, &mut hit, pool);
                        for &e in &hit {
                            // an empty iteration advances nothing
                            if e > q && insert(&mut seen, e) {
                                insert(&mut next, e);
                            }
                        }
                    }
                    union(out, &next);
                    std::mem::swap(&mut frontier, &mut next);
                }
                give(pool, seen);
                give(pool, frontier);
                give(pool, next);
                give(pool, hit);
            }
            Pat::Opt(p) => {
                out.push(pos);
                let mut hit = take(pool);
                p.ends(s, pos, &mut hit, pool);
                union(out, &hit);
                give(pool, hit);
            }
        }
    }

    /// The pattern as the JavaScript regex literal a `grammar.js`
    /// holds, delimiters included.
    pub fn to_js(&self) -> String {
        let mut s = String::from("/");
        self.write_js(&mut s, false);
        s.push('/');
        s
    }

    /// The pattern as a bare regex, no delimiters — what a TextMate
    /// grammar's `match` key holds. The spelling is `to_js`'s, which
    /// Oniguruma reads the same way.
    pub fn to_regex(&self) -> String {
        self.to_dialect(Dialect::Oniguruma)
    }

    /// The pattern as a bare regex in one editor's spelling.
    pub fn to_dialect(&self, d: Dialect) -> String {
        let mut s = String::new();
        self.write(&mut s, false, d);
        s
    }

    fn write_js(&self, out: &mut String, in_seq: bool) {
        self.write(out, in_seq, Dialect::Oniguruma)
    }

    fn write(&self, out: &mut String, in_seq: bool, d: Dialect) {
        match self {
            Pat::Ch(c) => out.push_str(&escape_for(*c, false, d)),
            Pat::Any => out.push('.'),
            Pat::Class { negated, items } => {
                out.push('[');
                if *negated {
                    out.push('^');
                }
                for it in items {
                    match it {
                        ClassItem::Ch(c) => out.push_str(&escape_for(*c, true, d)),
                        ClassItem::Range(a, b) => {
                            out.push_str(&escape_for(*a, true, d));
                            out.push('-');
                            out.push_str(&escape_for(*b, true, d));
                        }
                    }
                }
                out.push(']');
            }
            Pat::Seq(ps) => {
                for p in ps {
                    p.write(out, true, d);
                }
            }
            Pat::Alt(ps) => {
                if in_seq {
                    out.push_str(d.pat_open());
                }
                for (i, p) in ps.iter().enumerate() {
                    if i > 0 {
                        out.push_str(d.alt());
                    }
                    p.write(out, true, d);
                }
                if in_seq {
                    out.push_str(d.close());
                }
            }
            Pat::Star(p) | Pat::Plus(p) | Pat::Opt(p) => {
                let atom = matches!(**p, Pat::Ch(_) | Pat::Any | Pat::Class { .. });
                if !atom {
                    out.push_str(d.pat_open());
                }
                p.write(out, true, d);
                if !atom {
                    out.push_str(d.close());
                }
                out.push(match self {
                    Pat::Star(_) => '*',
                    Pat::Plus(_) => '+',
                    _ => '?',
                });
            }
        }
    }
}

/// Which editor's regex spelling a pattern prints in. The three differ
/// in how a group opens, how an alternation separates, and which
/// characters state themselves with a backslash; the operators `*`,
/// `+` and `?` and the class brackets read the same in all three.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Dialect {
    /// What a `grammar.js` regex literal holds, and what a TextMate or
    /// a Sublime Text `match` key holds — Oniguruma reads JavaScript's
    /// spelling.
    Oniguruma,
    /// Vim's very-magic regex, the form a leading `\v` selects, where a
    /// group opens `%(` and every character but a letter, a digit and
    /// `_` states itself with a backslash.
    Vim,
    /// The regex an Emacs string holds, where a group opens `\(?:`,
    /// alternation is `\|`, and the parentheses and the bar are
    /// ordinary characters. Elisp's own string escaping doubles the
    /// backslashes afterwards.
    Emacs,
}

impl Dialect {
    /// How a non-capturing group opens.
    pub fn open(self) -> &'static str {
        match self {
            Dialect::Oniguruma => "(?:",
            Dialect::Vim => "%(",
            Dialect::Emacs => "\\(?:",
        }
    }

    /// How a group opens inside a printed [`Pat`]. Oniguruma takes a
    /// capturing group there, which is what a `grammar.js` regex
    /// literal and the committed TextMate grammar hold.
    pub fn pat_open(self) -> &'static str {
        match self {
            Dialect::Oniguruma => "(",
            d => d.open(),
        }
    }

    /// How a group closes.
    pub fn close(self) -> &'static str {
        match self {
            Dialect::Oniguruma | Dialect::Vim => ")",
            Dialect::Emacs => "\\)",
        }
    }

    /// How two alternatives separate.
    pub fn alt(self) -> &'static str {
        match self {
            Dialect::Oniguruma | Dialect::Vim => "|",
            Dialect::Emacs => "\\|",
        }
    }

    /// A group holding the alternatives, whatever the context.
    pub fn group(self, alts: &[String]) -> String {
        format!("{}{}{}", self.open(), alts.join(self.alt()), self.close())
    }

    /// This text as the regex that matches it exactly.
    pub fn literal(self, s: &str) -> String {
        s.chars().map(|c| escape_for(c, false, self)).collect()
    }
}

/// A character as `d` spells it, inside a class or out.
fn escape_for(c: char, in_class: bool, d: Dialect) -> String {
    // Oniguruma and Vim spell a control character with a backslash;
    // an Emacs regex holds the character itself, and the Elisp string
    // it is written into spells it
    if matches!(c, '\n' | '\r' | '\t') && d != Dialect::Emacs {
        return match c {
            '\n' => "\\n".into(),
            '\r' => "\\r".into(),
            _ => "\\t".into(),
        };
    }
    match d {
        Dialect::Oniguruma => match c {
            '\\' => "\\\\".into(),
            '/' => "\\/".into(),
            ']' | '^' | '-' if in_class => format!("\\{c}"),
            '.' | '*' | '+' | '?' | '(' | ')' | '[' | '|' | '{' | '}' | '$' | '^' if !in_class => {
                format!("\\{c}")
            }
            c => c.to_string(),
        },
        // in very magic every character but a letter, a digit and `_`
        // has a meaning of its own, and a backslash takes it away; a
        // class reads the four characters that delimit it and nothing
        // else
        Dialect::Vim => match c {
            ']' | '^' | '-' | '\\' if in_class => format!("\\{c}"),
            c if in_class => c.to_string(),
            c if c.is_ascii_alphanumeric() || c == '_' => c.to_string(),
            c if c.is_ascii() => format!("\\{c}"),
            c => c.to_string(),
        },
        // Emacs reads every character of a class literally, a
        // backslash included, so a class prints its characters in the
        // order the pattern states them and nothing is escaped there
        Dialect::Emacs => match c {
            c if in_class => c.to_string(),
            '.' | '*' | '+' | '?' | '[' | ']' | '^' | '$' | '\\' => format!("\\{c}"),
            c => c.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn word() -> Pat {
        seq(vec![
            class("_", &[('A', 'Z'), ('a', 'z')]),
            star(class("_", &[('A', 'Z'), ('a', 'z'), ('0', '9')])),
        ])
    }

    #[test]
    fn a_pattern_matches_the_longest_text_it_can() {
        assert_eq!(word().longest("abc_1 = 2", 0), Some(5));
        assert_eq!(word().longest("1abc", 0), None);
        let num = seq(vec![
            plus(class("", &[('0', '9')])),
            opt(seq(vec![Pat::Ch('.'), plus(class("", &[('0', '9')]))])),
        ]);
        assert_eq!(num.longest("12.5x", 0), Some(4));
        // the fractional part needs a digit, so `0..10` keeps its dots
        assert_eq!(num.longest("0..10", 0), Some(1));
        let triple = plus(alt(vec![
            not_class("\"", &[]),
            seq(vec![Pat::Ch('"'), not_class("\"", &[])]),
            seq(vec![Pat::Ch('"'), Pat::Ch('"'), not_class("\"", &[])]),
        ]));
        assert_eq!(triple.longest("say \"so\" now\"\"\"", 0), Some(12));
    }

    #[test]
    fn a_pattern_prints_as_the_regex_a_grammar_js_holds() {
        assert_eq!(word().to_js(), "/[_A-Za-z][_A-Za-z0-9]*/");
        let s = seq(vec![Pat::Ch('#'), star(not_class("\n", &[]))]);
        assert_eq!(s.to_js(), "/#[^\\n]*/");
        assert_eq!(text("..=").to_js(), "/\\.\\.=/");
        assert_eq!(alt(vec![Pat::Ch('a'), Pat::Ch('b')]).to_js(), "/a|b/");
        assert_eq!(
            seq(vec![Pat::Ch('x'), alt(vec![Pat::Ch('a'), Pat::Ch('b')])]).to_js(),
            "/x(a|b)/"
        );
    }
}
