//! The established Cartan grammar as a syntax-only value.
//!
//! The interpreter returns concrete syntax trees and the printers generate
//! editor tooling from this same value. AST normalization and the additional
//! form checks performed after parsing are outside this module.
//!
//! # Newlines
//!
//! A newline is a token, `_newline`, and not an extra: it separates
//! wherever a comma separates — items, Map entries, brace items,
//! argument lists, rows — at every depth. Two rules join
//! lines instead, and both are stated in the tokens rather than in a
//! scanner. **A line that has not finished speaking continues**: after
//! an operator, an opener, `=`, `:=`, `<-` or a keyword an expression
//! follows, the grammar allows newlines, written as `nl()`. **A line
//! that opens with a continuation continues the line above**: an
//! infix operator that may open a line is one token that includes its own
//! leading newlines — `cont("+")` is the pattern `(\n[ \t\r]*)*\+`,
//! read as `"+"` — so the longest match at a line end takes the
//! operator rather than the separator. **A token that can begin an
//! expression never continues the line above**, with one exception: `-`,
//! `not`, `..` and `..=` are no continuation, so `[1\n -2\n 3]` keeps
//! its three items and `[0..4\n ..8]` its two axes, while unary `+`,
//! which may begin an expression, is read as the binary continuation at
//! the start of a line, so `[1\n +2]` is the one item `1 + 2`. A range is
//! complete at its operator, `0..` being the open range, so `..` takes
//! no newline after it either. A word operator that may open
//! a line — `with`, `but`, `and`, `or`, `else` — takes one whitespace character
//! after the word in its token, so `without = 2` on a fresh line is
//! still a binding.
//!
//! # What the interpreter alone states
//!
//! A loop header's expression is parsed without the payload and
//! configuration zones (`without`), so the brace after `every P`,
//! `fold u := u0` and `for i in space` belongs to the header; tree-sitter
//! states the same preference with `prec.dynamic`. The parser's own
//! diagnostics — the one a missing `=` after a port's name is reported
//! as — stand in `expecting` and `refusing`, each labelled with the
//! syntax-local rule ([`crate::diagnostic::Rule`]) for a host to present.
//! Tree-sitter, which has error
//! recovery of its own, prints the rule without them.

use std::sync::OnceLock;

use crate::diagnostic::{Comprehension, Rule as Stated};
use crate::lexer::{LOCKED, NAMESPACE, PARAM, PERIODIC, WHEN};
use treegram::pattern::{self as p, Pat};
use treegram::*;

/// **The node a namespace item builds**, and the hidden rule its body
/// items take: both are named for the keyword
/// ([`crate::NAMESPACE`]), so a word change carries the node
/// with it. the parser reads these rather than a spelling of
/// its own, and the editors' queries name what they print.
pub fn ns_node() -> &'static str {
    static N: OnceLock<String> = OnceLock::new();
    N.get_or_init(|| format!("{NAMESPACE}_definition")).as_str()
}

fn ns_item() -> &'static str {
    static N: OnceLock<String> = OnceLock::new();
    N.get_or_init(|| format!("_{NAMESPACE}_item")).as_str()
}

/// The grammar, built once.
pub fn grammar() -> &'static Grammar {
    static G: OnceLock<Grammar> = OnceLock::new();
    G.get_or_init(build)
}

/// The grammar as `grammar.js`.
pub fn print() -> String {
    treegram::print(grammar())
}

/// The grammar's lexical layer as a TextMate grammar, for the VS Code
/// extension (`editors/vscode/syntaxes/`): keywords, strings, numbers,
/// comments and operators read off the same value, with `true`,
/// `false` and `none` colored as the constants they are. The
/// structural coloring is the language server's semantic tokens.
pub fn print_textmate() -> String {
    treegram::textmate(grammar(), "source.cartan", CONSTANTS)
}

/// The word tokens that are values rather than keywords, which every
/// lexical printer colors as constants.
const CONSTANTS: &[&str] = &["true", "false", "none"];

/// The grammar's lexical layer as a Vim syntax file
/// (`editors/vim/syntax/cartan.vim`), read off the same value.
pub fn print_vim() -> String {
    treegram::vim(grammar(), "cartan", CONSTANTS)
}

/// The Vim file that gives a `.cart` document the `cartan` filetype
/// (`editors/vim/ftdetect/cartan.vim`).
pub fn print_vim_ftdetect() -> String {
    treegram::ftdetect("cartan", "cart")
}

/// The grammar's lexical layer as a Sublime Text syntax definition
/// (`editors/sublime/cartan.sublime-syntax`), read off the same value.
pub fn print_sublime() -> String {
    treegram::sublime(grammar(), "source.cartan", "cart", CONSTANTS)
}

/// The grammar's lexical layer as an Emacs major mode. The host supplies
/// its builtin names; syntax has no callable registry.
pub fn print_emacs(builtin_names: &[&str]) -> String {
    treegram::emacs(
        grammar(),
        "cartan",
        "cart",
        CONSTANTS,
        builtin_names,
        &["cartan", "lsp"],
    )
}

// ---- the precedence table ----------------------------------------------

const CONDITIONAL: i32 = 0;
const RANGE: i32 = 1;
const OR: i32 = 2;
const AND: i32 = 3;
const NOT: i32 = 4;
const COMPARE: i32 = 5;
/// `x ?? y` — the presence default: tighter than the
/// comparisons and the Boolean words, looser than arithmetic.
const DEFAULT: i32 = 6;
const ADD: i32 = 7;
const MUL: i32 = 8;
const NEG: i32 = 9;
const POWER: i32 = 10;
/// `with` and `but`: attachment, below the four zones.
const ATTACH: i32 = 11;
/// `key of value` — the component read,
/// above attachment and every arithmetic operator, so its operand is
/// the postfix zone's own: `0 of f(x)` reads the call's component and
/// `0 of v + 1` adds one to the component.
const OF: i32 = 12;
/// The postfix zones — call, subscript, configuration, payload, the
/// argument mark — chaining left.
const POSTFIX: i32 = 13;

// ---- helpers ------------------------------------------------------------

/// Newlines a line that has not finished speaking may hold.
fn nl() -> Rule {
    repeat(sym("_newline"))
}

/// A newline, or a comma: what separates the items of a bracketed run.
fn sep() -> Rule {
    choice(vec![",".into(), sym("_newline")])
}

/// Items separated by commas or newlines, any number of them, with
/// separators allowed before the first and after the last.
fn items(r: Rule) -> Rule {
    seq(vec![
        repeat(sep()),
        optional(seq(vec![
            r.clone(),
            repeat(seq(vec![repeat1(sep()), r])),
            repeat(sep()),
        ])),
    ])
}

/// Items as [`items`], at least one.
fn items1(r: Rule) -> Rule {
    seq(vec![
        repeat(sep()),
        r.clone(),
        repeat(seq(vec![repeat1(sep()), r])),
        repeat(sep()),
    ])
}

/// Leading newlines, each with the indentation after it.
fn lead(least_one: bool) -> Pat {
    let one = p::seq(vec![Pat::Ch('\n'), p::star(p::class(" \t\r", &[]))]);
    if least_one {
        p::plus(one)
    } else {
        p::star(one)
    }
}

/// An operator that may open a line: one token that includes its
/// leading newlines, read as the operator.
fn cont(op: &str) -> Rule {
    alias(token(Rule::Pat(p::seq(vec![lead(false), p::text(op)]))), op)
}

/// A word operator that may open a line: the keyword on the same line,
/// or one token that includes the newlines, the word, and one whitespace
/// character after it, so that a name opening with the word stays a
/// name.
fn cont_word(op: &str) -> Rule {
    let tok = Rule::Pat(p::seq(vec![
        lead(true),
        p::text(op),
        p::class(" \t\r\n", &[]),
    ]));
    choice(vec![op.into(), alias(token(tok), op)])
}

fn expr() -> Rule {
    sym("_expression")
}

/// A loop header's expression: the brace and the colon after it belong
/// to the header, and a paren after it is the
/// comprehension's bare body going missing, which the body's own
/// sentence reports rather than the call refusal.
fn header_expr() -> Rule {
    without(
        "_expression",
        &["payload_expression", "config_expression", "call_refusal"],
    )
}

/// A site update's key: a header expression, closed by
/// its colon, that holds no update of its own at its top level. A key is
/// greedy — `a but 0..n - 1: v` reads the whole range — so a `but` after
/// a parenthesized override, `cam but (d: 3.0) but (f: 1.0)`, would
/// otherwise open a key that no colon closes; a key that is itself an
/// update is parenthesized. The interpreter tries the key and retries
/// the component update where no colon closes it, so the copy ties with
/// the expression rule and tree-sitter keeps both readings to the colon.
fn site_key() -> Rule {
    without_tied(
        "_expression",
        &[
            "payload_expression",
            "config_expression",
            "call_refusal",
            "site_update_expression",
            "but_expression",
        ],
    )
}

/// The colon a brace's three readings share: a Map entry's, a Map
/// pattern's and a local's stated fiber, together with the
/// configuration zone's, which stands on any expression and so on a
/// brace item too. It is the colon that carries its leading newlines,
/// so a configuration zone may open a line. **The four
/// take one token because tree-sitter's lexer reads one token where
/// two match the same text**, and the reading whose rule spells the
/// other token ends at the colon: that is how `c: Real := 2` lost its
/// reading as a local inside a body. Every colon outside a brace — a
/// parameter's, a binding's, a component's — is the plain token, which
/// is what keeps the configuration zone from competing for those.
fn colon() -> Rule {
    cont(":")
}

/// The punctuation that makes a declaration of the name, or of the name
/// and the parameter list, before it: `=` binds, `:` states a fiber,
/// `:=` declares a local and `<-` writes a port. It is the whole of
/// `bare_expression`'s exclusion, so each token is held to
/// itself: `==` is the comparison and `::` the namespace separator, and
/// neither declares anything.
fn declares() -> Rule {
    choice(vec![
        ":=".into(),
        "<-".into(),
        seq(vec!["=".into(), not_ahead("=")]),
        seq(vec![":".into(), not_ahead(":")]),
    ])
}

/// The text from here to a `)` on this line — the probe that says a
/// head's parens close. `bare_expression`'s lookahead alone reads it
///: a `(` still open where the line ends is a head the
/// writer is still writing, and the declaration forms, reading the same
/// text, state that the text ran out.
fn closes() -> Rule {
    Rule::Pat(p::seq(vec![
        p::star(p::not_class(")\n", &[])),
        Pat::Ch(')'),
    ]))
}

/// One link of the `:` configuration zone: the colon and the atom
/// after it. The `not_ahead` is the interpreter's lookahead, which
/// tree-sitter, being GLR, prints without: the colon of a `:=` is one
/// character of a local's declaration rather than the opening of a
/// configuration zone, so `c: Real := 2` reads as the annotated local
/// it declares. Without it the reading of
/// `{ c: Real := 2 … }` as a Map takes the colon, and `_config_atom`'s
/// sentence is a hard failure that ends the parse before the reading
/// as a body is tried. Tree-sitter settles the same brace with the
/// `_pattern`/`map_entry` conflicts below.
fn config_link() -> Rule {
    seq(vec![
        field("operator", colon()),
        not_ahead("="),
        field("config", sym("_config_atom")),
    ])
}

/// The `:` configuration chain a header takes before its brace.
fn config_chain() -> Rule {
    repeat(config_link())
}

/// A comprehension's body: braced, or bare where the bare form is
/// unambiguous, its absence stated as
/// [`Stated::ComprehensionBody`]. The braced form takes the higher
/// dynamic precedence for tree-sitter; the interpreter tries it first.
fn comprehension_body() -> Rule {
    expecting(
        choice(vec![
            prec_dynamic(2, field("body", sym("body"))),
            prec_dynamic(
                1,
                seq(vec![
                    not_ahead(choice(vec!["(".into(), "[".into()])),
                    field("body", expr()),
                ]),
            ),
        ]),
        Stated::ComprehensionBody.label(),
    )
}

/// The lookahead that tells a parenthesized fiber description from the
/// paren's other readings: a name and `:` opens a product's component
/// or a variant that takes a payload, and a name and `|` opens a
/// variant that takes none.
fn described_ahead() -> Rule {
    choice(vec![
        seq(vec![sym("identifier"), ":".into()]),
        seq(vec![sym("identifier"), "|".into()]),
    ])
}

/// **The two parenthesized fiber descriptions**: a product's
/// components separated by `,` and a sum's variants
/// separated by `|`. The sum stands first, so a `|` run
/// is never read as a product's components and then refused; a product
/// list reaches the second alternative because a sum states at least
/// two variants, which is what the `|` run after the first one
/// requires. `guard` states the interpreter's lookahead, which the
/// three-way paren reading needs and which tree-sitter, being GLR,
/// prints without. `least_one` states whether the product form takes
/// an empty pair of parens, which the declaration's own sentence then
/// refuses.
fn descriptions(guard: Option<Rule>, least_one: bool) -> Vec<Rule> {
    let ahead_colon = || match &guard {
        Some(g) => vec![ahead(g.clone())],
        None => Vec::new(),
    };
    let sum = {
        let mut rs = vec!["(".into()];
        rs.extend(ahead_colon());
        rs.push(field("variant", sym("variant")));
        rs.push(repeat1(seq(vec![
            nl(),
            "|".into(),
            nl(),
            field("variant", sym("variant")),
        ])));
        rs.push(refusing(",", Stated::MixedSeparators.label()));
        rs.push(")".into());
        seq(rs)
    };
    let product = {
        let comps = match least_one {
            true => items1(field("component", sym("component"))),
            false => items(field("component", sym("component"))),
        };
        let mut rs = vec!["(".into()];
        rs.extend(ahead_colon());
        rs.push(comps);
        rs.push(refusing("|", Stated::MixedSeparators.label()));
        rs.push(")".into());
        seq(rs)
    };
    vec![sum, product]
}

/// **An axis open at one end or both**: `..`, `..e`,
/// `..=e` and `s..`, its ends read by `end`. A start and the operator
/// open a range too, and the range takes the end where one follows:
/// the open form stands where none does, so it yields to the range.
fn open_range(range_op: &Rule, end: fn() -> Rule, start: fn(Rule) -> Rule) -> Rule {
    choice(vec![
        seq(vec![
            field("operator", range_op.clone()),
            optional(seq(vec![not_ahead("{"), field("end", end())])),
        ]),
        start(seq(vec![
            field("start", end()),
            field("operator", range_op.clone()),
        ])),
    ])
}

/// The header word `word` spells.
fn comprehension_word(word: &str) -> Comprehension {
    match word {
        "for" => Comprehension::For,
        _ => Comprehension::Each,
    }
}

/// `for` and `each` share one header, clause for clause
///.
fn comprehension(word: &str, doc: &str) -> Def {
    def(
        &format!("{word}_expression"),
        doc,
        seq(vec![
            word.into(),
            nl(),
            field("binders", sym("binder_list")),
            expecting(
                "in",
                Stated::ComprehensionIn(comprehension_word(word)).label(),
            ),
            nl(),
            field(
                if word == "for" { "space" } else { "collection" },
                choice(vec![sym("header_open_range"), header_expr()]),
            ),
            comprehension_body(),
        ]),
    )
}

// ---- the grammar --------------------------------------------------------

fn build() -> Grammar {
    let word = p::seq(vec![
        p::class("_", &[('A', 'Z'), ('a', 'z')]),
        p::star(p::class("_", &[('A', 'Z'), ('a', 'z'), ('0', '9')])),
    ]);
    let digits = p::seq(vec![
        p::class("", &[('0', '9')]),
        p::star(p::class("_", &[('0', '9')])),
    ]);
    let number = token(seq(vec![
        digits.clone().into(),
        optional(Rule::Pat(p::seq(vec![Pat::Ch('.'), digits]))),
        optional(Rule::Pat(p::seq(vec![
            p::class("eE", &[]),
            p::opt(p::class("+-", &[])),
            p::plus(p::class("", &[('0', '9')])),
        ]))),
    ]));
    let string = token(seq(vec![
        "\"".into(),
        repeat(choice(vec![
            Rule::Pat(p::not_class("\"\\\n", &[])),
            Rule::Pat(p::seq(vec![Pat::Ch('\\'), Pat::Any])),
        ])),
        "\"".into(),
    ]));
    let triple_content = p::plus(p::alt(vec![
        p::not_class("\"", &[]),
        p::seq(vec![Pat::Ch('"'), p::not_class("\"", &[])]),
        p::seq(vec![Pat::Ch('"'), Pat::Ch('"'), p::not_class("\"", &[])]),
    ]));

    let compare = choice(vec![
        cont("=="),
        cont("!="),
        cont("<="),
        cont(">="),
        cont("<"),
        cont(">"),
    ]);
    // a range operator begins an expression too (`..e`), so it is no
    // continuation: a line opening with it is a fresh item
    let range_op = choice(vec!["..=".into(), "..".into()]);

    Grammar {
        name: "cartan".into(),
        doc: "tree-sitter-cartan — a grammar for cartan documents (`.cart`).\n\
              \n\
              **This file is generated.** The standalone `cartan-syntax` crate\n\
              prints it from `src/grammar.rs`, the same grammar its concrete\n\
              syntax parser reads directly. Edit that Rust source, run\n\
              `cargo run --example grammar > grammar.js`, then use an installed\n\
              Tree-sitter CLI to generate and test an editor parser. Generation\n\
              alone does not establish Tree-sitter parser conformance.\n\
              \n\
              An item is a binding, a port, a function,\n\
              an agent, a use, or a bare expression standing alone — which is\n\
              the spelling of `main = expr` — and every composite expression reads in four\n\
              zones — `head(constitutive) : config { payload } with control`\n\
              — in that fixed order and any number of turns.\n\
              \n\
              A newline is a token here, and separates wherever a comma does,\n\
              at every depth. A line continues where it has not finished\n\
              speaking — after an operator, an opener or a keyword wanting an\n\
              expression, newlines are allowed — and a line opening with an\n\
              infix operator continues the line above: such an operator is one\n\
              token carrying its leading newlines, aliased to the operator, so\n\
              the longest match at a line end takes it over the separator.\n\
              A token that can begin an expression is not one, so\n\
              `[1\\n -2\\n 3]` keeps its three items and `[0..4\\n ..8]` its two\n\
              axes. A word\n\
              operator opening a line — `with`, `but`, `and`, `or` — takes one\n\
              whitespace character after the word in its token, which is what\n\
              keeps `without = 2` on a fresh line a binding."
            .into(),
        extras: vec![Rule::Pat(p::plus(p::class(" \t\r", &[]))), sym("comment")],
        word: Some("identifier".into()),
        // the four the grammar needs. A braced body is reachable twice
        // wherever `comprehension_body` stands — as the rule's own
        // `body` field and as the expression a bare body is — and the
        // higher dynamic precedence on the braced form settles it.
        conflicts: vec![
            // A texture's body is either a braced body or an expression,
            // and the closing paren can share a newline with the caller.
            vec!["texture_expression".into(), "_expression".into()],
            vec!["_expression".into(), "for_expression".into()],
            vec!["_expression".into(), "each_expression".into()],
            // an open range's end may be a brace in the text, and the
            // header's body is one too: the interpreter reads no brace
            // there, and the parse that leaves the header a body stands
            vec!["header_open_range".into()],
            vec!["range_expression_1".into(), "header_open_range".into()],
            // `: name` may end there or continue into `(args)` or `[i]`,
            // and which it is shows only at the token after the name; the
            // dynamic precedence on the call and the index settles it
            vec!["_config_atom".into(), "_expression".into()],
            // after `but` a site update's key (`site_key`, the second
            // exclusion copy, tied) and a component update's override
            // open alike, and the colon closing the key is what tells
            // them apart. Where both readings stand, the
            // site update's dynamic precedence keeps it, as the
            // interpreter's order does
            vec!["_expression".into(), "_expression_2".into()],
            // a brace item opens the same way as a pattern and as an
            // expression, and the `:=` is what tells a local from a bare
            // item: the three parenthesized patterns are spelled exactly
            // as the values they mirror
            vec!["_expression".into(), "_pattern".into()],
            // a brace opens a Map literal, a body and a Map pattern
            // alike, and a bare name inside opens an item, an entry
            // and a key at once: the `:=` after the closing brace is
            // what tells the pattern from the two values
            vec!["_expression".into(), "map_sub_pattern".into()],
            // a bare run of names opening a brace (T:78) reads as a body's
            // item, a local's bare list pattern and a Map pattern's
            // entries at once, and the `:=` after the run, or the `}`
            // closing a Map pattern, is what tells the three apart
            vec!["_expression".into(), "_pattern".into(), "map_sub_pattern".into()],
            // a top-level item opening `f(x` is a definition's parameters
            // or, through a bare run's first position, a constructor
            // pattern, and the `=` or the `,` after the parens decides
            vec!["parameter".into(), "constructor_pattern".into()],
            // a Map literal's walk and the `each` comprehension share
            // their header, and the walk's dynamic precedence settles a
            // brace holding one toward the Map
            vec!["_expression".into(), "map_walk".into()],
            // a component read's key opens the way an expression opens
            // — a name, a number, a parenthesized run of numbers — and
            // the `of` after it is what tells the two apart
            //
            vec!["_expression".into(), "component_key".into()],
            // a `Mat` entry's key in a product, `((0, 1): v)`, opens the
            // way a component read's key and a grouping open
            vec!["_expression".into(), "component_key".into(), "position_pair".into()],
            // a fiber description as a constructor head opens the way a
            // named-parens pattern and a product value open — a name, a
            // colon, a name — and what follows the closing paren is what
            // tells the three apart
            vec!["fiber_type".into(), "_pattern".into()],
            vec!["fiber_type".into(), "_expression".into()],
            vec!["fiber_type".into(), "_expression".into(), "_pattern".into()],
            // a product's component and a sum's variant are spelled
            // alike — a name, a colon, a fiber — and the separator
            // after the first one is what tells the two apart
            //
            vec!["component".into(), "variant".into()],
            // a variant taking no payload is a bare name, which opens
            // the way a parenthesized expression and a parenthesized
            // pattern open: the `|` after the first one is what tells
            // the sum from the two
            vec!["_expression".into(), "variant".into()],
            vec!["_pattern".into(), "variant".into()],
            vec!["_expression".into(), "_pattern".into(), "variant".into()],
            // a brace item and a Map entry open alike — a name, a colon,
            // a name — and the `:=` after the fiber is what tells the
            // annotated local from the entry
            vec!["_expression".into(), "_pattern".into(), "map_entry".into()],
            vec![
                "_expression".into(),
                "_pattern".into(),
                "map_sub_pattern".into(),
                "map_entry".into(),
            ],
            vec!["parenthesized_expression".into(), "computed_key".into()],
            // inside a head's parens a name and a tight colon opens a
            // parameter and a sum fiber's variant argument alike, and
            // what follows decides. The two meet only where a
            // definition's head and a call's are read at once, which is
            // what an item standing on a bare call asks for
            vec!["variant_argument".into(), "_pattern".into()],
        ],
        brackets: vec![
            ("(".into(), ")".into()),
            ("[".into(), "]".into()),
            ("{".into(), "}".into()),
        ],
        rules: vec![
            def(
                "source_file",
                "items separated by newlines alone; an item that does not parse \
                 becomes an error node to the end of its line, and the next \
                 item is read",
                seq(vec![
                    nl(),
                    optional(seq(vec![
                        sym("_item_line"),
                        repeat(seq(vec![repeat1(sym("_newline")), sym("_item_line")])),
                        nl(),
                    ])),
                ]),
            ),
            def("_newline", "", Rule::Pat(Pat::Ch('\n'))),
            def(
                "comment",
                "",
                token(seq(vec!["#".into(), Rule::Pat(p::star(p::not_class("\n", &[])))])),
            ),
            def(
                "_item_line",
                "",
                recover(
                    seq(vec![
                        sym("_item"),
                        expecting(
                            choice(vec![ahead(sym("_newline")), not_ahead(Rule::Pat(p::not_class("", &[])))]),
                            Stated::ItemEnd.label(),
                        ),
                    ]),
                    sym("_newline"),
                ),
            ),
            def(
                "_item",
                "",
                choice(vec![
                    sym("doc_declaration"),
                    sym("use_declaration"),
                    sym(ns_node()),
                    sym("port_declaration"),
                    sym("fiber_declaration"),
                    sym("struct_declaration"),
                    sym("agent"),
                    // the bare expression stands above the three items
                    // whose heads it shares because each of those states
                    // a sentence at its `=` rather than failing back to a
                    // later alternative; its own lookahead — the head of a
                    // binding, a definition or a pattern refused before
                    // it, a newline or the end of the text required after
                    // it — is what keeps every one of them the item it is
                    //
                    sym("bare_expression"),
                    sym("pattern_binding"),
                    sym("function_definition"),
                    sym("binding"),
                ]),
            ),
            // ---- items -------------------------------------------------
            def(
                "doc_declaration",
                "`\"\"\"prose\"\"\"` standing alone — a doc string. It\n\
                 is a declaration rather than a binding: no name, no node in the\n\
                 graph, no part in evaluation. Standing first it is the document's\n\
                 prose; standing above a binding it is that binding's prose. Which\n\
                 one it is is positional, so a later analysis stage decides it — the grammar\n\
                 accepts the literal as an item anywhere.\n\
                 \n\
                 A triple string is an expression too, so a document's first line\n\
                 could be read as the bare item `main = \"\"\"prose\"\"\"`\n\
                ; the precedence here is the statement that a\n\
                 triple string standing alone as an item is the prose, and the\n\
                 interpreter, reading the alternatives in order, states the same\n\
                 by taking this one first.",
                prec(1, field("text", sym("triple_string"))),
            ),
            def(
                "use_declaration",
                "`use \"fig.cart\" as fig`, `use \"fig.cart\"::{panel}`, `use std::*`,\n\
                 `use std::{slider}`, `use std::slider` and `use \"hydro.cart\" as\n\
                 ideal with (pressure: ideal_gas)` — the one line that brings\n\
                 another document's names here.\n\
                 \n\
                 **The head is a quoted path or a bare namespace name**, and the\n\
                 absence of quotes is the whole of the difference: a path names a\n\
                 file, a bare name a namespace the language knows, `std` among\n\
                 them. **What follows either head is the same**: `as` binds the\n\
                 namespace under a name of the caller's choosing, `::*`, `::{a,\n\
                 b}` and `::a` import names into this document's own scope, and\n\
                 a `with (…)` clause states the argument each of an instantiated\n\
                 namespace's holes takes. Where `as` is absent a file's stem is\n\
                 the namespace's name.\n\
                 \n\
                 That a line states at most one of `as` and an import list, and\n\
                 which combinations a namespace head admits, a later analysis stage states.",
                seq(vec![
                    "use".into(),
                    expecting(
                        choice(vec![
                            field("path", sym("string")),
                            field("namespace", sym("identifier")),
                        ]),
                        Stated::UseHead.label(),
                    ),
                    optional(seq(vec![
                        "::".into(),
                        expecting(
                            field("imports", sym("import_list")),
                            Stated::UseImports.label(),
                        ),
                    ])),
                    optional(seq(vec![
                        "as".into(),
                        expecting(
                            field("alias", sym("identifier")),
                            Stated::UseAlias.label(),
                        ),
                    ])),
                    optional(field("holes", sym("with_clause"))),
                ]),
            ),
            def(
                "import_list",
                "`::*`, `::{lighter, slider}` and `::slider` — what a `use` line\n\
                 takes into this document's own scope. The star\n\
                 imports every public name of the namespace and the braced list\n\
                 the names it writes; a bare name is exactly the one-name list,\n\
                 so `use std::slider` and `use std::{slider}` are one line. An\n\
                 imported name that collides with a binding of this document is\n\
                 refused at the `use` line, by name.",
                choice(vec![
                    "*".into(),
                    seq(vec!["{".into(), items1(field("name", sym("identifier"))), "}".into()]),
                    field("name", sym("identifier")),
                ]),
            ),
            def(
                "with_clause",
                "`with (pressure: ideal_gas)` — the arguments an instantiated\n\
                 namespace's holes take: one binding per hole the\n\
                 head declares, the hole's name and the expression this document\n\
                 hands it. `with` is the control zone's word in expression\n\
                 position, and in a `use` line, where no expression stands, it is\n\
                 this clause and nothing else. That every hole is bound exactly\n\
                 once and that each argument fits the shape its hole declares,\n\
                 a later analysis stage states.",
                seq(vec![
                    "with".into(),
                    expecting(
                        "(",
                        Stated::WithHoles.label(),
                    ),
                    items1(sym("hole_binding")),
                    ")".into(),
                ]),
            ),
            def(
                "hole_binding",
                "",
                seq(vec![
                    field("name", sym("identifier")),
                    expecting(":", Stated::HoleArgument.label()),
                    nl(),
                    field("value", expr()),
                ]),
            ),
            def(
                ns_node(),
                &format!(
                    "`{NAMESPACE} hydro(pressure(rho: Real): Real) {{ … }}` — a unit\n\
                     written inside a document: a body of items whose\n\
                     holes the instantiating `use` line fills. A document holding\n\
                     no `{NAMESPACE}` item is one named by its file stem, so every\n\
                     `use` line in the corpus keeps its reading.\n\
                     \n\
                     **It holds values, functions and fibers alone.** A port, an\n\
                     update block and a `use` line are the document's, not its: a\n\
                     durable port declared inside one instantiated twice would\n\
                     have one declaring line reachable through two names, and no\n\
                     atomic multi-file rename makes those two writes one (A7,\n\
                     A3). One that needs a port takes it as a hole."
                ),
                seq(vec![
                    refusing("module", Stated::ModuleSpelling.label()),
                    NAMESPACE.into(),
                    expecting(
                        field("name", sym("identifier")),
                        Stated::NamespaceName.label(),
                    ),
                    expecting(
                        field("holes", sym("hole_list")),
                        Stated::NamespaceHoles.label(),
                    ),
                    expecting(
                        "{",
                        Stated::NamespaceBrace.label(),
                    ),
                    items(sym(ns_item())),
                    "}".into(),
                ]),
            ),
            def("hole_list", "", seq(vec!["(".into(), items(sym("hole")), ")".into()])),
            def(
                "hole",
                "`gamma: Real` and `pressure(rho: Real): Real` — one hole of a\n\
                 namespace head: the name the body reads, and\n\
                 the shape whatever fills it must take. A plain name states the\n\
                 fiber its value takes, and a parenthesized parameter list states\n\
                 a function's shape, the fiber after the parens being the one the\n\
                 result takes. The parameter list is the declaration's own\n\
                , so a hole's parameters are written the way a\n\
                 function's are; that a hole's parameter declares no default, the\n\
                 lowering states.\n\
                 \n\
                 A hole a value fills may state a default, `gamma: Real = 5.0 /\n\
                 3.0`, and a `use` line leaving that hole unbound takes it. That a\n\
                 function-shaped hole states none — a default there would be a\n\
                 function value, and the function space is first-order — the\n\
                 lowering states.",
                seq(vec![
                    field("name", sym("identifier")),
                    optional(field("parameters", sym("parameter_list"))),
                    expecting(
                        ":",
                        Stated::HoleFiber.label(),
                    ),
                    field("type", sym("type_annotation")),
                    optional(seq(vec!["=".into(), nl(), field("default", expr())])),
                ]),
            ),
            def(
                ns_item(),
                "",
                seq(vec![
                    refusing(
                        choice(vec![
                            "use".into(),
                            PARAM.into(),
                            "state".into(),
                            "every".into(),
                            "while".into(),
                        ]),
                        Stated::NamespaceItem.label(),
                    ),
                    choice(vec![
                        sym("doc_declaration"),
                        sym("fiber_declaration"),
                        sym("pattern_binding"),
                        sym("function_definition"),
                        sym("binding"),
                    ]),
                ]),
            ),
            def(
                "port_declaration",
                "`param name = expr` / `state name = expr` — every port holds an\n\
                 initial (axiom A2). A `param` belongs to the text and a `state`\n\
                 to the run. A port states an optional fiber after `:`,\n\
                 `state x: Vector<Real, 2>|None = vec(0, 0)`, and the initial is\n\
                 held to it.\n\
                 \n\
                 **A parenthetical after the initial states when the port is\n\
                 locked**: `param res: Int = 100 (locked when\n\
                 physics != none)`. It belongs to a `param`, whose three writers\n\
                 a lock reaches; a `state` belongs to the run, and the parser\n\
                 states that where one writes the clause.",
                seq(vec![
                    field("kind", choice(vec![PARAM.into(), "state".into()])),
                    field("name", sym("identifier")),
                    optional(seq(vec![":".into(), field("type", sym("type_annotation"))])),
                    expecting("=", Stated::PortValue.label()),
                    nl(),
                    field("value", expr()),
                    optional(field("lock", sym("lock_clause"))),
                ]),
            ),
            def(
                "lock_clause",
                "`(locked when physics != none)` — the parenthetical a `param`\n\
                 writes after its initial to state the condition under which it\n\
                 is locked. The condition is an ordinary Bool\n\
                 expression of the document, elaborated into the graph like any\n\
                 other and read live, so a lock states itself at the declaration\n\
                 rather than being inferred from what a run did.",
                seq(vec![
                    "(".into(),
                    LOCKED.into(),
                    expecting(
                        WHEN,
                        Stated::LockCondition.label(),
                    ),
                    nl(),
                    expecting(
                        field("condition", expr()),
                        Stated::LockCondition.label(),
                    ),
                    ")".into(),
                ]),
            ),
            def(
                "fiber_declaration",
                "`fiber Cons = (D: Real, S: Vector<Real, 2>, tau: Real)` and\n\
                 `fiber Shape = (A: Real | B: Vector<Real, 3>)` — a fiber\n\
                 declaration: a product's components separated by `,`\n\
                , a sum's variants separated by `|`\n\
                , each a name and a fiber type inside the named\n\
                 parens the value form already uses — a variant written as a\n\
                 bare name taking no payload, so `fiber Mode = (Binary |\n\
                 Single)` is an enumeration. A product states at least one\n\
                 component and a sum at least two variants, and no name twice,\n\
                 which a later analysis stage states. `fiber Cell<k> = (at: [Int; k],\n\
                 v: Vector<Real, k>)` names a width parameter after the name,\n\
                 which the component fibers read.\n\
                 \n\
                 **A declared product states its algebra after `:`**\n\
                : `fiber Conserved: Module = (…)` admits\n\
                 componentwise addition, negation and scaling, `fiber\n\
                 Primitive: Algebra = (…)` the componentwise product of two\n\
                 values besides, and a declaration writing no word is a set\n\
                 every arithmetic operator refuses on. A sum states no word,\n\
                 its structure being the active variant's.",
                seq(vec![
                    "fiber".into(),
                    field("name", sym("identifier")),
                    optional(seq(vec!["<".into(), field("param", sym("identifier")), ">".into()])),
                    optional(seq(vec![":".into(), field("algebra", sym("identifier"))])),
                    expecting("=", Stated::FiberDeclares.label()),
                    refusing("{", Stated::StructFiber.label()),
                    expecting(choice(descriptions(None, false)), Stated::FiberDeclares.label()),
                ]),
            ),
            def(
                "struct_declaration",
                "`struct State = { iter: Int, time: Real, cons: Array<Conserved, 2> }`\n\
                 — a name for a struct's entries, standing wherever a type\n\
                 annotation stands. It is a declaration and no\n\
                 node: a document that states one shape at several binding sites\n\
                 writes the braces once.\n\
                 \n\
                 **It is an alias and no constructor.** Braces build a struct and\n\
                 parens build a fiber, so a struct alias has no call form and\n\
                 `State(…)` refuses, naming the brace literal that builds one.",
                seq(vec![
                    "struct".into(),
                    expecting(field("name", sym("identifier")), Stated::StructName.label()),
                    expecting("=", Stated::StructEquals.label()),
                    expecting(
                        field("entries", sym("struct_type")),
                        Stated::StructEntries.label(),
                    ),
                ]),
            ),
            def(
                "struct_type",
                "`{ iter: Int, cons: Array<Conserved, 2> }` — a struct's entries\n\
                 written where a type stands: one entry per key, in the order the\n\
                 text writes them, each a bare name and the kind it holds. It is\n\
                 the Map literal read backwards, as the brace pattern is\n\
                , and it nests.\n\
                 \n\
                 **Parens describe a fiber's components and braces a struct's\n\
                 entries.** A fiber is plain old data at a site and a struct is a\n\
                 record of whole values, so the bracket tells the two apart and a\n\
                 struct stands in no field; that a struct is refused wherever a\n\
                 fiber is required, a later analysis stage states.",
                seq(vec!["{".into(), items(field("entry", sym("struct_entry"))), "}".into()]),
            ),
            def(
                "struct_entry",
                "`iter: Int` — one entry of a struct: the key, and the kind the\n\
                 entry holds, which may take the absence alternative,\n\
                 `name: Real|None`: the entry is always there and its value may be\n\
                 absent. That a struct states at least one entry, and no key\n\
                 twice, and that the alternative is `None`, a later analysis stage\n\
                 states.",
                seq(vec![
                    field("name", sym("identifier")),
                    expecting(":", Stated::StructEntryType.label()),
                    field("type", sym("fiber_type")),
                    optional(seq(vec![
                        "|".into(),
                        field("alternative", sym("fiber_type")),
                        refusing("|", Stated::AnnotationAlternative.label()),
                    ])),
                ]),
            ),
            def(
                "component",
                "",
                seq(vec![
                    field("name", sym("identifier")),
                    expecting(":", Stated::ComponentType.label()),
                    field("type", sym("fiber_type")),
                ]),
            ),
            def(
                "variant",
                "`A: Real` and `Binary` — one variant of a sum fiber\n\
                : its name, and the fiber its payload takes where\n\
                 it takes one. **A variant written as a bare name takes no\n\
                 payload**, so `(Binary | Single)` is an enumeration and\n\
                 `(Point | Segment: Real)` mixes the two forms.",
                seq(vec![
                    field("name", sym("identifier")),
                    optional(seq(vec![":".into(), field("type", sym("fiber_type"))])),
                ]),
            ),
            def(
                "fiber_type",
                "A fiber type: a name with optional `<…>` arguments, types or\n\
                 widths — `Real`, `Vector<Real, 2>`, `Mat<3, 3>`, a static list\n\
                 kind `[Int; 2]` (`static_list_type`), a type taking the\n\
                 absence alternative, `List<Real|None>`, where a later analysis stage admits\n\
                 an entry that may be absent — or a product's\n\
                 components or a sum's variants stated in the open, either of\n\
                 which stands wherever an alias does.\n\
                 **The name is qualified where a call head's\n\
                 is**, `orbit::Binary` naming a used document's\n\
                 fiber. The lookahead before the components is the\n\
                 interpreter's alone, as `described_head`'s is: a `component`\n\
                 states its own sentence for a missing `:`, and the parenthesized\n\
                 description is one of three readings of a paren, so the sentence\n\
                 waits until the colon says the reading is this one.",
                choice({
                    let guard = choice(vec![described_ahead(), ")".into()]);
                    let mut rs = descriptions(Some(guard), false);
                    // the brace form reaches every place a type stands,
                    // so a struct written where a fiber is required
                    // refuses in one sentence at a later analysis stage rather
                    // than as a parse error at each of them
                    rs.push(field("entries", sym("struct_type")));
                    rs.push(field("list", sym("static_list_type")));
                    rs.push(seq(vec![
                        field("name", choice(vec![sym("qualified_identifier"), sym("identifier")])),
                        optional(seq(vec![
                            "<".into(),
                            items(choice(vec![sym("type_annotation"), sym("number")])),
                            ">".into(),
                        ])),
                    ]));
                    rs
                }),
            ),
            def(
                "static_list_type",
                "`[Int; 2]` and `[Int, Real]` — a static list kind:\n\
                 the repeated form states one entry kind and a width — a number, a\n\
                 declaration's width parameter, or `_` for any — and the concrete\n\
                 form one kind per position, at least one, which is sugar for the\n\
                 repeated form where its entries agree. The brackets are positional\n\
                 as a List literal's are, and the length is a fact of the kind.",
                // one line, as a kind is written: a newline after the
                // `[` would leave a List literal and this form one
                // reading of the same text inside a parenthesized
                // description
                choice(vec![
                    seq(vec![
                        "[".into(),
                        field("item", sym("fiber_type")),
                        ";".into(),
                        field("width", choice(vec![sym("number"), sym("identifier")])),
                        "]".into(),
                    ]),
                    seq(vec![
                        "[".into(),
                        field("item", sym("fiber_type")),
                        repeat(seq(vec![",".into(), field("item", sym("fiber_type"))])),
                        "]".into(),
                    ]),
                ]),
            ),
            def(
                "binding",
                "`name = expr`, and `name: Real = expr` where the binding states\n\
                 the fiber its value takes.",
                seq(vec![
                    field("name", sym("identifier")),
                    optional(seq(vec![":".into(), field("type", sym("type_annotation"))])),
                    expecting("=", Stated::BindingValue.label()),
                    nl(),
                    field("value", expr()),
                ]),
            ),
            def(
                "bare_expression",
                "`plot`, `3.0 * x`, `canvas2d { … } with nav(xr)` standing alone\n\
                 as an item — **the spelling of `main = expr`**.\n\
                 Any expression stands here, over as many\n\
                 continuation lines as it takes, and the item ends where the\n\
                 line does. A later analysis stage supplies the name `main` and the item\n\
                 reaches the elaborator as the binding it spells, so a document\n\
                 writing both refuses where two `main` bindings refuse. A\n\
                 namespace body holds no such item: it declares no endpoint.\n\
                 \n\
                 **The lookahead is what leaves every declaration the form it\n\
                 is**: an identifier followed by `=`, `:`, `:=` or `<-`, and an\n\
                 identifier followed by a parenthesized parameter list and then\n\
                 `=` or `:`, are the binding, the local, the write and the\n\
                 function definition, and a parenthesized or braced pattern\n\
                 followed by `=` is the top-level pattern; this item refuses\n\
                 them all before reading a word. What is left is exactly the\n\
                 bare expression: an identifier that runs on into a call's\n\
                 arguments, a `with`, a brace, a `.`, an operator or the line's\n\
                 end. A head whose parens the writer has not closed on the line\n\
                 stands out too, since the declaration forms read the same text\n\
                 and their own miss reports it as text run out. The lookahead\n\
                 is the interpreter's, which tree-sitter, being GLR, prints\n\
                 without: there the declaration's own `=` is what decides, and\n\
                 the two readings run together until it — `parameter_list`\n\
                 taking the immediate `(` is what carries the definition's\n\
                 reading that far, and `doc_declaration`'s precedence is what\n\
                 keeps a triple string standing alone the prose it is.",
                seq(vec![
                    not_ahead(choice(vec![
                        seq(vec![sym("identifier"), declares()]),
                        // the open head stands before the reading that
                        // parses the parens, so a `(` the writer has
                        // not closed leaves the declaration forms to
                        // read the text — and to state, at the end of
                        // it, that the text ran out
                        seq(vec![sym("identifier"), immediate("("), not_ahead(closes())]),
                        seq(vec![sym("identifier"), sym("parameter_list"), declares()]),
                        seq(vec![
                            choice(vec![sym("named_pattern"), sym("map_pattern")]),
                            declares(),
                        ]),
                    ])),
                    field("value", expr()),
                    choice(vec![
                        ahead(sym("_newline")),
                        not_ahead(Rule::Pat(p::not_class("", &[]))),
                    ]),
                ]),
            ),
            def(
                "pattern_binding",
                "`(D: D) = U`, `[i, j] = s` and `{ rho, vel } = m` — a top-level\n\
                 pattern item: each name the pattern binds is a\n\
                 document name holding that component, exported through a `use`\n\
                 alias as any other document name is, and a field of products\n\
                 binds its component fields. The parenthesized patterns and the\n\
                 braced one open where no other item does. The constructor form\n\
                 `Cons(D, S, tau) = U` is written the way a function definition\n\
                 is, and elaboration tells the two apart by what the head names.\n\
                 A bare run, `a, b = t`, is `[a, b] = t` (T:78); it stands first,\n\
                 so `[a, b], c = t` is one run rather than a bracket missing its\n\
                 `=`, and a top-level comma separates no items.",
                seq(vec![
                    field(
                        "pattern",
                        choice(vec![
                            sym("bare_list_pattern"),
                            sym("named_pattern"),
                            sym("list_pattern"),
                            sym("map_pattern"),
                        ]),
                    ),
                    expecting("=", Stated::PatternValue.label()),
                    nl(),
                    field("value", expr()),
                ]),
            ),
            def(
                "function_definition",
                "`name(params) : { k: default, … } = expr` — a transparent function\n\
                 (A9). The declaration mirrors the call: the Map gives the\n\
                 configuration names and their defaults.\n\
                 \n\
                 The head is spelled exactly as a call is, and an item stands on\n\
                 a bare call too, so the two readings run together\n\
                 until the `=` decides. What carries both that far is\n\
                 `parameter_list` taking the immediate `(` the lexer reads after\n\
                 a head as well as the plain one.",
                seq(vec![
                    field("name", sym("identifier")),
                    field("parameters", sym("parameter_list")),
                    optional(seq(vec![":".into(), nl(), field("config", sym("map"))])),
                    expecting("=", Stated::FunctionBody.label()),
                    nl(),
                    field("body", expr()),
                ]),
            ),
            // the open paren stands in both spellings the lexer knows.
            // A call's paren follows its head immediately, and a
            // definition's head is spelled exactly as a call is, so at
            // `f(` — where an item may be either — the
            // immediate token is the one that stands; taking it here is
            // what keeps the definition's reading alive beside the
            // call's until the `=` decides
            def(
                "parameter_list",
                "",
                seq(vec![
                    choice(vec![immediate("("), "(".into()]),
                    items(sym("parameter")),
                    ")".into(),
                ]),
            ),
            def(
                "parameter",
                "`xr: Interval`, `at: Real|None`, `color: Color = hex(\"#56b6c2\")`,\n\
                 `Cons(d, s, _)` — a parameter binds a pattern and states an\n\
                 optional type and an optional default, independently of each\n\
                 other (wishlist item 71). The annotation and the default stand on\n\
                 the whole slot. That only a trailing run declares defaults, that a\n\
                 default stands on a bare name, and that `None` is the one\n\
                 alternative an annotation takes, a later analysis stage states. A\n\
                 positional tuple stands only with the fiber it reads stated after\n\
                 it, `(rho, _, pre): Primitive`.",
                choice(vec![
                    seq(vec![
                        field("name", sym("site_pattern")),
                        ":".into(),
                        field("type", sym("type_annotation")),
                        optional(seq(vec!["=".into(), nl(), field("default", expr())])),
                    ]),
                    // the colon stands in both spellings the lexer
                    // knows, as the parameter list's paren does: a sum
                    // constructor's variant argument takes the tight
                    // colon (D:20), and at `f(x:` — where an item may
                    // be a definition's head or a bare constructor call
                    // — the immediate token is the one
                    // that stands, so taking it here keeps the
                    // parameter's reading alive until the `=` decides
                    seq(vec![
                        field("name", sym("_pattern")),
                        optional(seq(vec![
                            choice(vec![immediate(":"), ":".into()]),
                            field("type", sym("type_annotation")),
                        ])),
                        optional(seq(vec!["=".into(), nl(), field("default", expr())])),
                    ]),
                ]),
            ),
            def(
                "type_annotation",
                "A type, or a function signature where a type stands. The\n\
                 signature stands first, since a fiber description also opens\n\
                 with a paren and takes `()`. A trailing `->` after a bare type\n\
                 is read so that the missing parentheses are named rather than\n\
                 left to the token after it; where a signature is admitted,\n\
                 source normalization states.",
                choice(vec![
                    sym("signature_type"),
                    seq(vec![
                        sym("fiber_type"),
                        optional(seq(vec![
                            "|".into(),
                            field("alternative", sym("fiber_type")),
                            refusing(
                                "|",
                                Stated::AnnotationAlternative.label(),
                            ),
                        ])),
                        optional(seq(vec![
                            field("arrow", "->"),
                            field("result", sym("type_annotation")),
                        ])),
                    ]),
                ]),
            ),
            def(
                "signature_type",
                "`(Real, Real) -> Real` — the signature of a function a parameter\n\
                 receives: its argument types in parens, any number of them, and\n\
                 its result type. It describes a function named at the call, not\n\
                 a function value.",
                seq(vec![
                    "(".into(),
                    items(field("argument", sym("type_annotation"))),
                    ")".into(),
                    field("arrow", "->"),
                    field("result", sym("type_annotation")),
                ]),
            ),
            def(
                "agent",
                "`every P (while G)? (: config)* { p <- e, … }`, and\n\
                 `while G (: config)* { p <- e, … }` — an agent in its three\n\
                 spellings, of which the third is sugar. `P` is the\n\
                 period, of type `Real|None`, and `none` is the block that\n\
                 fires as fast as the runner allows; **`while G` is the\n\
                 spelling of `every none while G`**, a guard with no period,\n\
                 and `while` with no guard is not a form. The brace outranks\n\
                 the payload zone: an agent\n\
                 header bars a juxtaposed brace, whichever spelling opens it,\n\
                 so the brace is the agent's\n\
                 writes and never `P`'s or `G`'s payload, and the\n\
                 `:` after the header is the agent's configuration,\n\
                 `: { precision: \"f64\" }`.",
                prec_dynamic(
                    1,
                    choice(vec![
                        seq(vec![
                            "every".into(),
                            nl(),
                            field("period", header_expr()),
                            optional(seq(vec![
                                nl(),
                                "while".into(),
                                nl(),
                                field("guard", header_expr()),
                            ])),
                            config_chain(),
                            expecting(
                                field("body", sym("body")),
                                Stated::UpdateBody.label(),
                            ),
                        ]),
                        // the period this spelling leaves out is `none`, and
                        // the parser synthesizes it, so the two spellings
                        // reach the elaborator as one AST
                        seq(vec![
                            "while".into(),
                            nl(),
                            // the brace standing where the guard belongs is
                            // the unguarded block written in the guarded
                            // spelling, and the guard's own rule would read
                            // it as `G` and then complain that the body is
                            // missing from a line that has one. The refusal
                            // prints nowhere in the grammar value, so the
                            // editors' parser is untouched by it
                            refusing(
                                "{",
                                Stated::WhileGuard.label(),
                            ),
                            field("guard", header_expr()),
                            config_chain(),
                            expecting(
                                field("body", sym("body")),
                                Stated::UpdateBody.label(),
                            ),
                        ]),
                    ]),
                ),
            ),
            // ---- expressions -------------------------------------------
            def(
                "texture_expression",
                "`texture(|p| color)` or `texture(|p| { local := value\n color })` — a
                 deferred per-pixel body. `p` is the canvas data-coordinate
                 `Vector<Real, 2>`, and the body must return one rank-zero `Color`.
                 The binder is local to the body; this form does not make a
                 general function value",
                seq(vec![
                    "texture".into(),
                    immediate("("),
                    nl(),
                    "|".into(),
                    nl(),
                    field("parameter", sym("identifier")),
                    nl(),
                    "|".into(),
                    nl(),
                    field(
                        "body",
                        choice(vec![
                            prec_dynamic(2, sym("body")),
                            prec_dynamic(1, expr()),
                        ]),
                    ),
                    nl(),
                    ")".into(),
                ]),
            ),
            climb(
                "_expression",
                "the expression, in one table: the primary forms, then every\n\
                 operator with its precedence — tightest last",
                choice(vec![
                    sym("configurable_expression"),
                    sym("texture_expression"),
                    sym("call_expression"),
                    sym("qualified_identifier"),
                    sym("boolean"),
                    sym("none"),
                    // the tagged literal stands above the bare name, since
                    // `f` against a quote opens one and `f` alone is a name
                    sym("template_string"),
                    sym("identifier"),
                    sym("number"),
                    sym("triple_string"),
                    sym("string"),
                    sym("list"),
                    sym("product"),
                    sym("parenthesized_expression"),
                    sym("map"),
                    sym("body"),
                    sym("tap_expression"),
                    sym("for_expression"),
                    sym("each_expression"),
                    sym("fold_expression"),
                    sym("match_expression"),
                ]),
                vec![
                    prefix(
                        "conditional_expression",
                        CONDITIONAL,
                        seq(vec![
                            "if".into(),
                            nl(),
                            field("condition", expr()),
                            nl(),
                            expecting("then", Stated::Then.label()),
                            nl(),
                            field("consequence", expr()),
                            cont_word("else"),
                            nl(),
                        ]),
                        "alternative",
                    )
                    .assoc(Assoc::Right)
                    .doc("`if c then a else b` — `then` is required and the `else`\nclause is optional, the two alternatives standing here in that\norder. `else` is a continuation word, so it may\nopen a line of its own, and a next line beginning with any\nother word closes the conditional at its consequence."),
                    prefix(
                        "conditional_expression",
                        CONDITIONAL,
                        seq(vec![
                            "if".into(),
                            nl(),
                            field("condition", expr()),
                            nl(),
                            expecting("then", Stated::Then.label()),
                            nl(),
                        ]),
                        "consequence",
                    )
                    .assoc(Assoc::Right)
                    .doc("`if c then a` — a later analysis stage supplies `none` for the\nalternative. Reading the clause above first is\nwhat binds the `else` of `if a then if b then x else y` to\nthe inner `if`."),
                    prefix("unary_expression", NOT, seq(vec![field("operator", "not"), nl()]), "operand")
                        .doc("`not`, above the comparisons and below `and`"),
                    prefix("unary_expression", NEG, seq(vec![field("operator", "-"), nl()]), "operand"),
                    prefix("unary_expression", NEG, seq(vec![field("operator", "+"), nl()]), "operand")
                        .doc("unary `+`, standing wherever unary `-` does, so `+0.5` and `-0.5`\nread alike. A line opening with `+` still continues the line above,\nthe continuation token taking it as the binary operator."),
                    infix("range_expression", RANGE, Assoc::Left, "left", seq(vec![field("operator", range_op.clone()), not_ahead("{")]), "right")
                        .doc("`a..b` / `a..=b` — the loosest operator there is, Rust's own\nprecedence, so `0..n - 1` reads `0..(n - 1)`. It groups left here,\nand a later analysis stage refuses `a..b..c`: a range states two endpoints.\nAn end reads no brace, which is a header's body after `for i in 3..`.\nThe end stands on the operator's line: `0..` is complete as the open\nrange, so a newline after the operator ends it."),
                    postfix("periodic_expression", RANGE, "axis", PERIODIC)
                        .doc("`0..n periodic` — the box axis that wraps. It\nstands on a range and nowhere else, so it reads at the range's own\nprecedence and a later analysis stage refuses it on anything but a range: the\nword states an axis of a box, and a box's axis is a range."),
                    infix("binary_expression", OR, Assoc::Left, "left", seq(vec![field("operator", cont_word("or")), nl()]), "right"),
                    infix("binary_expression", AND, Assoc::Left, "left", seq(vec![field("operator", cont_word("and")), nl()]), "right"),
                    infix("binary_expression", COMPARE, Assoc::Left, "left", seq(vec![field("operator", compare), nl()]), "right")
                        .doc("the comparisons group left here, and a later analysis stage refuses a chain\n— `a < b < c` is two comparisons and says so"),
                    infix("binary_expression", DEFAULT, Assoc::Right, "left", seq(vec![field("operator", cont("??")), nl()]), "right")
                        .doc("`x ?? y` — the presence default: `x` where it is\npresent, `y` where it is absent. It groups right, so defaults chain,\nand stands between the comparisons and arithmetic, so `m ?? 0 > 3` is\n`(m ?? 0) > 3` and `m ?? n + 1` is `m ?? (n + 1)`. The lexer's longest\nmatch takes it over the postfix absence mark `?`."),
                    infix("binary_expression", ADD, Assoc::Left, "left", seq(vec![field("operator", cont("+")), nl()]), "right"),
                    infix("binary_expression", ADD, Assoc::Left, "left", seq(vec![field("operator", cont("<>")), nl()]), "right")
                        .doc("`a <> b` — the join: the string two strings make, the list two\nlists make end to end, the union two Maps make. It stands at `+`'s\nprecedence and groups left, and it is a continuation operator, so a\nline opening with `<>` attaches to the line above it. The lexer's\nlongest match takes it over `<` followed by `>`, which is no legal\nsequence."),
                    infix("binary_expression", ADD, Assoc::Left, "left", seq(vec![field("operator", "-"), nl()]), "right")
                        .doc("`-` carries no leading newlines: a line opening with `-` is a fresh\nsigned item, so `[1\\n -2\\n 3]` keeps its three items"),
                    infix("binary_expression", MUL, Assoc::Left, "left", seq(vec![field("operator", choice(vec![cont("*"), cont("//"), cont("/"), cont("@")])), nl()]), "right")
                        .doc("`@` is the action operator: infix at the\nmultiplicative level, left-associative, so `g @ a @ b` is the scalar\nand `(R1 * R2) @ v` composes then acts"),
                    infix("binary_expression", POWER, Assoc::Right, "left", seq(vec![field("operator", cont("^")), nl()]), "right"),
                    infix("with_expression", ATTACH, Assoc::Left, "target", seq(vec![cont_word("with"), nl()]), "control")
                        .doc("`view with control` — attachment, chainable, never a child. The\ncontrol takes the postfix zones of its own — `with scrub(g) : cfg { … }`\n— and not a further `with`, so the chain stays flat."),
                    infix(
                        "site_update_expression",
                        ATTACH,
                        Assoc::Left,
                        "target",
                        seq(vec![cont_word("but"), nl(), field("key", site_key()), ":".into(), nl()]),
                        "value",
                    )
                    .dynamic(1)
                    .doc("`base but key: value` — the site update: the\nfield on the left with the sites the key states holding the value and\nevery other site kept. The key is an expression — a range or a box,\n`space(x)`, a mask — closed by its colon, so it reads no\nconfiguration zone; the chain groups left at `with`'s precedence, and a\nline opening with `but` continues the one above. It stands before the\ncomponent update, which the interpreter reads when no colon closes a\nkey: `cam but (d: 3.0)` names a component, `a but (n - 1): 0.0` a site."),
                    infix("but_expression", ATTACH, Assoc::Left, "target", seq(vec![cont_word("but"), nl()]), "override")
                        .doc("`base but override` — the product-fiber component update\n: the value on the left with the components the\noverride names replaced, at `with`'s precedence and grouping left."),
                    postfix("absent_expression", POSTFIX, "value", "?")
                        .doc("`f(x?, y)` — the argument mark: a postfix `?` says\nthe slot it stands on licenses absence. It rides the postfix chain,\nso a call result marks too — `points(vec(x, y)?)`."),
                    postfix("index_expression", POSTFIX, "target", field("index", sym("subscript"))),
                    prefix(
                        "component_expression",
                        OF,
                        seq(vec![field("key", sym("component_key")), "of".into(), nl()]),
                        "value",
                    )
                    .assoc(Assoc::Right)
                    .doc("`S of u`, `0 of v`, `a of c`, `(i, j) of m` — the component read\n. It stands above every arithmetic and\ncomparison operator and above unary minus, so `0 of v + 1` adds one\nto the component and `-0 of v` negates it, and its operand is a\npostfix-level expression, so `0 of f(x)` and `0 of v[s]` read as\nwritten. It groups right, so `0 of S of u` reads the product's\ncomponent and then that value's."),
                    postfix(
                        "config_expression",
                        POSTFIX,
                        "target",
                        config_link(),
                    )
                    .targets(&["config_expression"])
                    .dynamic(1)
                    .doc("`e : map` — configuration, ascribed against a registry signature.\nIt chains, and a later key wins. A link of the chain\ntakes a dynamic precedence, so `e : a : b[i]` is two links with `b[i]`\nthe second atom rather than one link whose atom is `(a : b)[i]`."),
                    postfix(
                        "config_expression",
                        POSTFIX,
                        "target",
                        config_link(),
                    ),
                    postfix(
                        "configure_dot_expression",
                        POSTFIX,
                        "target",
                        seq(vec![field("operator", cont(".")), field("config", sym("map"))]),
                    )
                    .doc("`value.{key: expression}` — literal settings closing; links chain left"),
                    postfix("payload_expression", POSTFIX, "target", field("body", sym("body")))
                        .doc("`head(…) { items }` — call sugar for a trailing List argument; a\nbrace whose first item is a `port <-` write is the writes a control\nhead takes, which a later analysis stage decides"),
                    postfix("payload_expression", POSTFIX, "target", field("body", sym("list")))
                        .doc("`head [a, b]` — a payload of one row, exactly `head { [a, b] }`\n. The bracket opens on the head's line, as the\nbrace does, so `head` and a `[` on the next line stay two items.\nThe index takes its `[` immediately after its target, so `xs[i]`\nreads the element and `xs [i]` is this zone."),
                    postfix(
                        "namespace_dot_refusal",
                        POSTFIX,
                        "target",
                        refusing(
                            seq(vec![cont("."), sym("identifier")]),
                            Stated::NamespaceDot.label(),
                        ),
                    )
                    .targets(&["identifier"])
                    .doc("`ns.x` — the retired spelling of a namespace step, refused with\n`ns::x`. The dot is no separator in the language: a namespace is\n`::` and a component is `of`, so the one lexical run a name and a\ndot open is named here rather than left to the elaborator."),
                    postfix(
                        "dot_refusal",
                        POSTFIX,
                        "target",
                        refusing(
                            seq(vec![cont("."), sym("identifier")]),
                            Stated::ComponentDot.label(),
                        ),
                    )
                    .targets(&[
                        "call_expression",
                        "index_expression",
                        "component_expression",
                        "absent_expression",
                        "parenthesized_expression",
                        "dot_refusal",
                    ]),
                    postfix("call_refusal", POSTFIX, "target", refusing(immediate("("), Stated::CallHead.label())),
                ],
            ),
            def(
                "call_expression",
                "only named functions can be called, and a used document's name is\n\
                 reached through its namespace. Primes ride the head —\n\
                 `f'(x)`, the univariate derivative at the prime count's order\n\
                 — and a primed head is a call, so the parens\n\
                 follow it. **The paren follows the head immediately**, as the\n\
                 subscript's bracket follows its target: `f(x)` is a call and\n\
                 `f (x)` is the name `f` beside a parenthesized value, which is\n\
                 what lets a header's collection stand beside a bare body and a\n\
                 Map walk's collection beside its entry. A\n\
                 parenthesized fiber description heads a call too,\n\
                 `(D: Real, E: Real)(1, 0)`, which is the anonymous spelling of a\n\
                 declared product's constructor:\n\
                 in head position the parens can hold nothing else, since neither\n\
                 a product value nor a grouping takes arguments.",
                prec(
                    2,
                    choice(vec![
                        // `Shape(A: 1.0)` — a sum fiber's constructor
                        //: the head applied to a named
                        // argument stating the variant and its payload.
                        // It stands before the positional forms, since
                        // a name and a colon in the parens is what
                        // tells it from them.
                        prec_dynamic(
                            3,
                            seq(vec![
                                field("function", choice(vec![sym("qualified_identifier"), sym("identifier")])),
                                field("variants", sym("variant_arguments")),
                            ]),
                        ),
                        prec_dynamic(
                            3,
                            seq(vec![
                                field("described", sym("described_head")),
                                field("variants", sym("variant_arguments")),
                            ]),
                        ),
                        seq(vec![
                            field("function", choice(vec![sym("qualified_identifier"), sym("identifier")])),
                            field("primes", sym("primes")),
                            expecting(field("arguments", sym("argument_list")), Stated::PrimedCall.label()),
                        ]),
                        seq(vec![
                            field("function", choice(vec![sym("qualified_identifier"), sym("identifier")])),
                            field("arguments", sym("argument_list")),
                        ]),
                        // **the topology word heads its own reader**
                        //: `periodic` is reserved, so
                        // the box axis's word and a name never collide,
                        // and the reader `periodic(p)` reads the word
                        // as the head it is, the call arm taking the
                        // head's own text as it does for a name.
                        seq(vec![
                            field("function", Rule::from(PERIODIC)),
                            field("arguments", sym("argument_list")),
                        ]),
                        // **the lock's word heads its own reader**
                        //: `locked` is reserved, so the
                        // parenthetical's word and a name never collide,
                        // and the reader `locked(p)` reads the word as
                        // the head it is, the call arm taking the head's
                        // own text as it does for a name.
                        seq(vec![
                            field("function", Rule::from(LOCKED)),
                            field("arguments", sym("argument_list")),
                        ]),
                        prec_dynamic(
                            2,
                            seq(vec![
                                field("described", sym("described_head")),
                                field("arguments", sym("argument_list")),
                            ]),
                        ),
                    ]),
                ),
            ),
            def(
                "described_head",
                "`(D: Real, E: Real)` standing where a constructor's name stands —\n\
                 in `(D: Real, E: Real)(1, 0)` and in the pattern\n\
                 `(D: Real, E: Real)(D, E) := u`. The description names the\n\
                 same fiber as its alias, written where no `fiber`\n\
                 item declares one, and each component states the fiber that\n\
                 component takes. The lookahead is the\n\
                 interpreter's alone — a `component` states its own sentence for a\n\
                 missing `:`, and that sentence belongs to the declaration rather\n\
                 than to a paren the parser is still choosing between three\n\
                 readings of; tree-sitter, which is GLR, prints without it. A\n\
                 sum's variants stand here too, `(A: Real | B: Real)(A: 1.0)`\n\
                .",
                choice(descriptions(Some(described_ahead()), true)),
            ),
            def(
                "variant_arguments",
                "`(A: 1.0)` — the argument a sum fiber's constructor takes\n\
                : the variant it names and the payload it holds.\n\
                 The rule takes a run of them so that a call naming none or\n\
                 several is refused at elaboration, against the roster the\n\
                 declaration states, rather than by the parser.",
                seq(vec![immediate("("), items1(sym("variant_argument")), ")".into()]),
            ),
            def(
                "variant_argument",
                "`A: 1.0` — the variant a sum fiber's constructor names and the\n\
                 payload it holds. **A variant argument's colon is\n\
                 tight**: it follows the name immediately, and `name : value`\n\
                 with whitespace before the colon is a configuration zone on the\n\
                 expression `name`, read by the positional\n\
                 argument list. That is what tells the configured payload in\n\
                 `f(block : { pad: 1 } { 1 }, \"t\")` from a constructor in both\n\
                 readers (D:20).",
                seq(vec![
                    field("name", sym("identifier")),
                    immediate(":"),
                    nl(),
                    field("value", expr()),
                ]),
            ),
            def("primes", "", immediate(Rule::Pat(p::plus(Pat::Ch('\''))))),
            def(
                "argument_list",
                "the constitutive zone is positional; configuration rides after\n\
                 ':'. A slot may open with a run of `:` marks — the call-site\n\
                 derivative's slot, its colon count the order;\n\
                 the dynamic precedence keeps `f(a : style)` an ascription on `a`\n\
                 rather than `a` beside a marked second slot. A name inside the\n\
                 parens is a configuration key in the wrong zone, and says so\n\
                . The `(` is immediate, so a call's parens\n\
                 follow its head with no space between them.",
                seq(vec![
                    immediate("("),
                    items(seq(vec![
                        refusing(
                            seq(vec![
                                field("key", sym("identifier")),
                                "=".into(),
                                not_ahead("="),
                            ]),
                            Stated::PositionalArgument.label(),
                        ),
                        choice(vec![
                            prec_dynamic(-1, seq(vec![field("mark", sym("mark")), expr()])),
                            expr(),
                        ]),
                    ])),
                    ")".into(),
                ]),
            ),
            def("mark", "", Rule::Pat(p::plus(Pat::Ch(':')))),
            def(
                "subscript",
                "immediate `[`: `wave[i]` indexes, and a `[` after a space or on a\n\
                 fresh line opens a row. Axis selectors are comma-separated.",
                seq(vec![
                    immediate("["),
                    nl(),
                    sym("selector"),
                    repeat(seq(vec![nl(), ",".into(), nl(), sym("selector")])),
                    nl(),
                    "]".into(),
                ]),
            ),
            def(
                "selector",
                "One axis selector: an ordinary expression — an\n\
                 Int global coordinate or a range restriction — or an open\n\
                 range, where the target's own box bounds the missing end:\n\
                 `a[4..]`, `a[..10]`, `a[..=10]`, `a[..]`.",
                choice(vec![sym("open_range"), expr()]),
            ),
            def(
                "open_range",
                "An axis open at one end or both. In a subscript\n\
                 the target's own box bounds the missing end; as an entry of a\n\
                 `for` header's box literal it states an axis the comprehension\n\
                 does not bound, and the comprehension is a rule. A later analysis stage\n\
                 refuses it anywhere else. `..=` with no end has nothing to\n\
                 reach, which a later analysis stage says.",
                open_range(&range_op, expr, |r| prec(-1, r)),
            ),
            def(
                "header_open_range",
                "The open axis a `for` header's collection states alone,\n\
                 `for i in .. { … }`, `for i in 3.. { … }`: its ends are the\n\
                 header's own expressions, and a missing end reads no brace,\n\
                 which is the header's body.",
                // the header's expression is the climb's exclusion copy,
                // whose precedences treegram lifts to `2p + 1`: the open
                // start ties the range there, and the conflict below
                // leaves the two to the text that follows
                open_range(&range_op, header_expr, |r| prec(2 * RANGE + 1, r)),
            ),
            def(
                "configurable_expression",
                "`:base :over { body }` — ordered settings atoms and a value body",
                seq(vec![
                    repeat1(config_link()),
                    nl(),
                    field("body", sym("body")),
                ]),
            ),
            def(
                "_config_atom",
                "a name, and what a name reaches: an index picks a style out of a\n\
                 list of them and a call asks a function for one.\n\
                 The chain stops before a brace, which belongs to the head, and\n\
                 the call and the index bind to the atom rather than to the\n\
                 ascription that holds it: `el : styles[i]` picks a style, where\n\
                 `(el : styles)[i]` would index a dressed element.",
                expecting(
                    choice(vec![
                        sym("map"),
                        prec_dynamic(1, sym("index_expression")),
                        prec_dynamic(1, sym("call_expression")),
                        sym("qualified_identifier"),
                        sym("identifier"),
                        sym("parenthesized_expression"),
                    ]),
                    Stated::ConfigAtom.label(),
                ),
            ),
            def(
                "fold_expression",
                "`fold u := u0 for x in seq while G : { max_iter } { body }` and\n\
                 `reduce s := s0 …` — the bounded recurrence and the header that\n\
                 answers its destination. One rule,\n\
                 because it is one header: the keyword is the whole of the\n\
                 difference. The dynamic precedence gives the brace to the body,\n\
                 exactly as the agent's header keeps its writes.",
                prec_dynamic(
                    1,
                    seq(vec![
                        field("keyword", choice(vec!["fold".into(), "reduce".into()])),
                        nl(),
                        field("accumulator", sym("identifier")),
                        expecting(":=", Stated::FoldSeed.label()),
                        nl(),
                        field("seed", header_expr()),
                        optional(seq(vec![
                            nl(),
                            "for".into(),
                            nl(),
                            field("element", sym("binder_list")),
                            expecting("in", Stated::FoldIn.label()),
                            nl(),
                            field("driver", choice(vec![sym("header_open_range"), header_expr()])),
                        ])),
                        optional(seq(vec![nl(), "while".into(), nl(), field("guard", header_expr())])),
                        config_chain(),
                        expecting(field("body", sym("body")), Stated::FoldBody.label()),
                    ]),
                ),
            ),
            def(
                "match_expression",
                "`match s { A(x) then x, B(v) then norm(v) }` — the sum fiber's\n\
                 reader, the header family's fourth member beside\n\
                 `if` and `for`. The scrutinee stands in the header's own zone,\n\
                 so the brace after it holds the arms rather than a payload on\n\
                 the scrutinee, and the arms separate as body items do, one per\n\
                 line or by commas. A trailing `else` arm stands where no\n\
                 variant's arm did; that every variant has an arm or an `else`\n\
                 stands, elaboration states against the declared roster.",
                prec_dynamic(
                    1,
                    seq(vec![
                        "match".into(),
                        nl(),
                        field("scrutinee", header_expr()),
                        expecting(
                            "{",
                            Stated::MatchArms.label(),
                        ),
                        items(choice(vec![sym("match_arm"), sym("else_arm")])),
                        "}".into(),
                    ]),
                ),
            ),
            def(
                "match_arm",
                "`A(x) then x`, `B(X: x, Y: y) then x + y` and `Binary then 2` —\n\
                 one arm of a `match`: the variant's\n\
                 name, one sub-pattern for its payload where it takes one,\n\
                 `then`, and the expression the arm takes. Since a variant holds\n\
                 exactly one payload, the parens may hold the items of the\n\
                 payload's named-parens pattern directly, `B(X: x, Y: y)`\n\
                 standing for `B((X: x, Y: y))`; the colons tell that spelling\n\
                 from the positional one, and both are admitted. **A variant\n\
                 that takes no payload is read by its bare name**, and that an\n\
                 arm's parens agree with the roster, elaboration states.",
                seq(vec![
                    field("variant", sym("identifier")),
                    optional(seq(vec![
                        "(".into(),
                        choice(vec![
                            items1(sym("named_sub_pattern")),
                            field("pattern", sym("_pattern")),
                        ]),
                        ")".into(),
                    ])),
                    expecting(
                        "then",
                        Stated::ArmThen.label(),
                    ),
                    nl(),
                    field("body", expr()),
                ]),
            ),
            def(
                "else_arm",
                "`else e` — the arm a `match` takes where no variant's arm stood\n\
                . One of these stands in place of the arms the\n\
                 roster is missing.",
                seq(vec!["else".into(), nl(), field("body", expr())]),
            ),
            comprehension(
                "for",
                "`for [i, j] in space { body }` and `for I in space body` — the\n\
                 comprehension, a member of the header family.\n\
                 The braced body has the higher dynamic precedence,\n\
                 which is how the grammar states the loop-header zone rule: a\n\
                 brace after the space is the comprehension's body, never a\n\
                 payload on the space. The brace is optional, as it is on a\n\
                 function definition — a bare body may not open with `(`, `[` or\n\
                 an operator, since juxtaposition is whitespace-insensitive here\n\
                 and the space would take them.",
            ),
            comprehension(
                "each",
                "`each x in xs { body }` and `each x in xs body` — the List\n\
                 comprehension, `for`'s sibling: it walks a\n\
                 List, a Map, a range or a box of any rank and builds a `List` of\n\
                 the body's results\n\
                 where `for` builds a located field. The header shape is `for`'s\n\
                 clause for clause.",
            ),
            def(
                "binder_list",
                "a header's binder is one pattern: a\n\
                 name takes the element whole, a bracket destructures — the site of\n\
                 a box one name per axis, or a lockstep's collections one pattern\n\
                 apiece, which a later analysis stage tells apart by the collection — and a\n\
                 constructor pattern reads components. `(i, j)` is no pattern here,\n\
                 and says so naming the bracket. Two or more patterns written\n\
                 with commas and no bracket are the bracket's pattern\n\
                 (`bare_list_pattern`): `for i, j in [.., ..]` is `for [i, j] in`.",
                seq(vec![
                    refusing(sym("site_pattern"), Stated::HeaderParen.label()),
                    choice(vec![sym("bare_list_pattern"), sym("_pattern")]),
                ]),
            ),
            def(
                "_pattern",
                "A pattern — what stands wherever a name is bound: a\n\
                 local, a `for` binder, a parameter, a control binder and a\n\
                 top-level item. A name binds the value whole, `_` binds nothing,\n\
                 `[i, j]` reads a static list's positions,\n\
                 `Cons(d, s, tau)` reads a declared product's components in the\n\
                 declared order, `(at: p)` reads components by name\n\
                , and `{ rho, vel }` reads a Map's entries by\n\
                 key. Every sub-pattern position takes any pattern, so the form\n\
                 nests. The named parens begin the way an expression does, and\n\
                 the token after the first name decides; the braced form begins\n\
                 the way a Map literal does, and the `:=` after it decides. A\n\
                 positional tuple, `(rho, _, pre)`, stands only at a parameter or\n\
                 a local stating the fiber it reads.",
                choice(vec![
                    sym("constructor_pattern"),
                    sym("named_pattern"),
                    sym("list_pattern"),
                    sym("map_pattern"),
                    sym("identifier"),
                ]),
            ),
            def(
                "list_pattern",
                "`[i, j]`, `[[i, j], t]` — a static list destructured, one\n\
                 sub-pattern per position: the List literal read\n\
                 backwards. The length is the right side's kind's, so a list of no\n\
                 known length refuses at check time. A position is a pattern again,\n\
                 and a parenthesized one is none: a bracket nests in a bracket.",
                // the List literal's own separators, so the two readings
                // of a bracket share their newlines until the `:=` or the
                // `=` after it tells them apart
                seq(vec![
                    "[".into(),
                    items1(seq(vec![refusing(sym("site_pattern"), Stated::BracketParen.label()), sym("_pattern")])),
                    "]".into(),
                ]),
            ),
            def(
                "bare_list_pattern",
                "`i, j` and `a, [b, c]` — a static list destructured with no\n\
                 bracket, exactly `[i, j]`, wherever a whole binder stands: a\n\
                 comprehension's header, a local before its `:=` and a top-level\n\
                 pattern item before its `=`. It takes two\n\
                 positions or more, so one name stays a name. Inside a brace a\n\
                 run of names followed by `:=` is this pattern rather than an item\n\
                 and a local, `{ a, b := t … }` binding both; a position is a\n\
                 pattern again and nests with a bracket. Tree-sitter reads the\n\
                 brace's other reading, an item and then a local, beside it, and\n\
                 the dynamic precedence keeps this one, as the interpreter's\n\
                 order does.",
                // the first position takes no paren refusal of its own: a
                // brace opening `(i, j)` or `(k):` is another reading, and
                // its own sentence stands there
                prec_dynamic(
                    1,
                    seq(vec![
                        sym("_pattern"),
                        repeat1(seq(vec![
                            ",".into(),
                            nl(),
                            refusing(sym("site_pattern"), Stated::BracketParen.label()),
                            sym("_pattern"),
                        ])),
                    ]),
                ),
            ),
            def(
                "constructor_pattern",
                "`Cons(d, vec(sx, sy), _)` — the components of `vec` or of a\n\
                 declared product fiber, positional in the declared order, one\n\
                 sub-pattern each. It mirrors the constructor literal\n\
                , and the arity is checked against the\n\
                 declaration at elaboration. A fiber description heads it too,\n\
                 `(D: Real, E: Real)(D, E) := u`, which states the fiber the\n\
                 value takes and reads its components. The paren\n\
                 follows the head immediately, as the constructor literal's does.",
                choice(vec![
                    seq(vec![
                        field("head", sym("identifier")),
                        immediate("("),
                        items1(sym("_pattern")),
                        ")".into(),
                    ]),
                    prec_dynamic(
                        2,
                        seq(vec![
                            field("described", sym("described_head")),
                            immediate("("),
                            items1(sym("_pattern")),
                            ")".into(),
                        ]),
                    ),
                ]),
            ),
            def(
                "named_pattern",
                "`(at: p, grab: q)`, `(0: x, 2: z)` — components by key, any subset\n\
                 in any order. It mirrors the value form a readout writes for an\n\
                 unnamed product, it is the form the gesture\n\
                 products take, and an integer key reads a `vec` or a `Complex`\n\
                 by position.",
                seq(vec!["(".into(), items1(sym("named_sub_pattern")), ")".into()]),
            ),
            def(
                "named_sub_pattern",
                "",
                seq(vec![
                    field("name", choice(vec![sym("identifier"), sym("number")])),
                    ":".into(),
                    nl(),
                    field("pattern", sym("_pattern")),
                ]),
            ),
            def(
                "map_pattern",
                "`{ rho, vel }`, `{ rho: r }`, `{ pos: (x, y) }` — a Map's entries\n\
                 by key: the Map literal read backwards, with the binder\n\
                 where the literal puts the value. A bare name is the key and the\n\
                 binder at once, renaming puts a sub-pattern after the colon as a\n\
                 named-parens pattern does, and the sub-pattern nests. A key\n\
                 stands twice refuses, and each key is the subscript that reads\n\
                 it.",
                seq(vec!["{".into(), items1(sym("map_sub_pattern")), "}".into()]),
            ),
            def(
                "map_sub_pattern",
                "",
                choice(vec![
                    seq(vec![
                        field("key", sym("identifier")),
                        colon(),
                        nl(),
                        field("pattern", sym("_pattern")),
                    ]),
                    field("key", sym("identifier")),
                ]),
            ),
            def(
                "site_pattern",
                "`(rho, _, pre): Primitive` — a positional tuple, one sub-pattern\n\
                 per component of the fiber the binding site states after it, in\n\
                 declared order, a nested tuple reading a\n\
                 component's own components, `(d, (sx, _), _): Cons`. A site is a\n\
                 static list and destructures with a bracket, `[i, j]`\n\
                .",
                seq(vec![
                    "(".into(),
                    choice(vec![sym("site_pattern"), sym("_pattern")]),
                    repeat(seq(vec![
                        nl(),
                        ",".into(),
                        nl(),
                        choice(vec![sym("site_pattern"), sym("_pattern")]),
                    ])),
                    ")".into(),
                ]),
            ),
            def(
                "component_key",
                "The key of a component read: a name for a\n\
                 declared product fiber's component and for a `Color`'s channel,\n\
                 an integer for a `vec` or a `Complex`, and a\n\
                 parenthesized pair for a `Mat` or a `Metric`. Nothing else\n\
                 stands before `of`, so the key is read at elaboration rather\n\
                 than computed.",
                choice(vec![
                    sym("identifier"),
                    sym("number"),
                    seq(vec![
                        "(".into(),
                        sym("number"),
                        repeat1(seq(vec![nl(), ",".into(), nl(), sym("number")])),
                        ")".into(),
                    ]),
                ]),
            ),
            def(
                "position_pair",
                "`(0, 1)` — a `Mat`'s or a `Metric`'s entry named by its row and\n\
                 column, where a product's component key stands.",
                seq(vec![
                    "(".into(),
                    sym("number"),
                    repeat1(seq(vec![nl(), ",".into(), nl(), sym("number")])),
                    ")".into(),
                ]),
            ),
            def(
                "product",
                "`(D: 1.0, S: vec(0.3, 0.4), tau: 2.5)` — a product fiber's value\n\
                 stated component by component, and the override\n\
                 a `but` takes, one component included. A component\n\
                 key and its colon tell it from grouping parens, which hold a bare\n\
                 expression. An integer key names\n\
                 a component by position, so `v but (0: e)` updates a `vec`\n\
                .",
                seq(vec!["(".into(), items1(sym("product_component")), ")".into()]),
            ),
            def(
                "product_component",
                "the component wins over reading the key as an expression with a\n\
                 configuration after it. A parenthesized run of\n\
                 positions in key position is a component's position written out,\n\
                 `m but ((0, 1): 1.0)` one entry of a `Mat`; a key is never a site,\n\
                 the sites of a field being updated outside the parens,\n\
                 `u but [0..1, 1..2]: 1.0`.",
                prec(
                    POSTFIX + 1,
                    seq(vec![
                        field("name", choice(vec![sym("identifier"), sym("number"), sym("position_pair")])),
                        ":".into(),
                        nl(),
                        field("value", expr()),
                    ]),
                ),
            ),
            def(
                "parenthesized_expression",
                "`(3, 4)` and `(3,)` refuse: parens group one expression, and a\n\
                 site is the static list of its Ints, `[3, 4]`",
                // the separator runs are the product's, so the two forms
                // that open with `(` share one prefix and tree-sitter
                // defers the choice to the token that decides it
                seq(vec![
                    "(".into(),
                    repeat(sep()),
                    expr(),
                    refusing(",", Stated::ParenComma.label()),
                    repeat(sep()),
                    ")".into(),
                ]),
            ),
            def(
                "list",
                "a List literal, and the box literal a header walks: an entry may\n\
                 be an open range, which only a `for` header's box admits",
                seq(vec![
                    "[".into(),
                    items(seq(vec![
                        choice(vec![sym("open_range"), expr()]),
                        refusing(seq(vec![nl(), ";".into()]), Stated::ListRepeat.label()),
                    ])),
                    "]".into(),
                ]),
            ),
            def(
                "tap_expression",
                "`tap (g)? (: config)* { p <- e, … }` — the gesture-triggered writes,\n\
                 which are the whole of the control. The config zone stands between\n\
                 the head and its writes, as it does on a `fold`. The optional\n\
                 binder is the form the rest of the control roster takes\n\
                : `g` is bound to the gesture, and the body\n\
                 reads it by dot. `scrub(g)`, `hover(g)` and `wheel(g)` reach the\n\
                 same reading through `call_expression` under a juxtaposed writes\n\
                 brace, since those heads are ordinary names where `tap` is a\n\
                 keyword.",
                seq(vec![
                    "tap".into(),
                    optional(seq(vec!["(".into(), field("binder", sym("_pattern")), ")".into()])),
                    config_chain(),
                    expecting(field("body", sym("body")), Stated::TapBody.label()),
                ]),
            ),
            // ---- braces --------------------------------------------------
            def(
                "map",
                "`{ k: v, (e): w, each k in ks { (k): f(k) } }` — a Map literal\n\
                : a bare key is its own name, a parenthesized key is\n\
                 an expression returning a Str, and a walk among the entries\n\
                 yields one entry per element. An empty brace is an empty\n\
                 Map, which the precedence over `body` states.",
                prec(1, seq(vec!["{".into(), items(choice(vec![sym("map_entry"), sym("map_walk")])), "}".into()])),
            ),
            def(
                "map_entry",
                "`k: v` — one entry of a Map literal. A name, a colon and a value\n\
                 is also a body item configuring that name, `{ w: style }`, and\n\
                 the entry's dynamic precedence settles the brace toward the Map\n\
                . It stands on the entry rather than on the brace so\n\
                 that it grows with the entries, one configuration link being\n\
                 dynamic itself.",
                prec_dynamic(
                    2,
                    seq(vec![
                        field("key", choice(vec![sym("identifier"), sym("computed_key")])),
                        colon(),
                        nl(),
                        field("value", expr()),
                    ]),
                ),
            ),
            def(
                "computed_key",
                "`(expr)` — a Map entry's key computed rather than named, since\n\
                 a bare identifier in a Map literal is the key's own name\n\
                . The expression returns a Str.",
                // the separator runs are the parenthesized expression's,
                // so the four forms that open with `(` share one prefix
                seq(vec!["(".into(), repeat(sep()), expr(), repeat(sep()), ")".into()]),
            ),
            def(
                "map_walk",
                "`each k in ks (k): f(k)` — a comprehension among a Map\n\
                 literal's entries: the header is `each`'s, and\n\
                 the body is the one entry the walk writes per element, whose\n\
                 key is parenthesized. **The body is that entry, braced or\n\
                 bare**: `each k in ks (k): v` and `each k in ks { (k): v }`\n\
                 are the one walk, as a List's splice takes both spellings.\n\
                 The braceless form reads because a call's paren follows its\n\
                 head immediately, so the collection ends at the space before\n\
                 the key. The walk is one item of the literal, so its entries\n\
                 land in walk order among the others. The dynamic precedence\n\
                 outranks the braced comprehension body's, so a brace holding a\n\
                 walk reads as the Map it is rather than as a body whose one\n\
                 item is an `each`.",
                prec_dynamic(3, seq(vec![
                    "each".into(),
                    nl(),
                    field("binders", sym("binder_list")),
                    expecting("in", Stated::WalkIn.label()),
                    nl(),
                    field("collection", header_expr()),
                    expecting(
                        field("body", choice(vec![sym("map"), sym("map_walk_entry")])),
                        Stated::WalkBody.label(),
                    ),
                ])),
            ),
            def(
                "map_walk_entry",
                "`(k): v` standing as a Map literal's walk body with no brace\n\
                 around it. It is the entry `map_entry` spells,\n\
                 held to the computed key: a bare name is the key's own name,\n\
                 which every element would then share, and the walk's brace-free\n\
                 body is the one place where a bare name here would also open a\n\
                 comprehension's bare body dressed by a configuration zone. The\n\
                 braced spelling takes `map_entry` through `map`, and the two\n\
                 lower as one entry.",
                seq(vec![
                    field("key", sym("computed_key")),
                    colon(),
                    nl(),
                    field("value", expr()),
                ]),
            ),
            def(
                "body",
                "One brace rule for three jobs, self-describing by its items: `:=`\n\
                 locals and bare expressions make a payload or a body, `<-` writes\n\
                 make the writes an `every`, a `tap` or a `key(name)` head takes.\n\
                 (The node's name follows `ast::Body`, the crates' word for the\n\
                 general brace.) A durable local refuses here, on A7's ground: a\n\
                 body's line stands for many instances, and a durable write\n\
                 rewrites one declaring line.",
                seq(vec![
                    "{".into(),
                    items(seq(vec![
                        refusing(
                            PARAM,
                            Stated::LocalParam.label(),
                        ),
                        refusing(
                            seq(vec![
                                field("name", sym("identifier")),
                                "=".into(),
                                not_ahead("="),
                            ]),
                            Stated::BodyEquals.label(),
                        ),
                        refusing(
                            seq(vec!["in".into(), nl()]),
                            Stated::ClauseComprehension.label(),
                        ),
                        refusing(
                            seq(vec![
                                field("head", sym("identifier")),
                                field("tail", token(repeat1(seq(vec![cont("."), sym("identifier")])))),
                                "<-".into(),
                            ]),
                            Stated::ComponentWrite.label(),
                        ),
                        choice(vec![sym("local"), sym("state_local"), sym("write"), expr()]),
                    ])),
                    "}".into(),
                ]),
            ),
            def(
                "local",
                "`x := e` and `Cons(d, s, _) := u` — a local, whose left side is a\n\
                 pattern. The parenthesized patterns begin the way an\n\
                 expression does, and the `:=` is what tells a local from a bare\n\
                 item. A local states an optional fiber after `:`, `a: Real := 3`,\n\
                 and the value it binds is held to it; the\n\
                 lowering states that the annotation stands on a bare name, since\n\
                 a pattern already states what the value is. The annotation's\n\
                 colon is the one a Map entry and a configuration zone take, so\n\
                 a brace opening `{ c: Real := 2` shifts one token for all\n\
                 three readings and keeps them alive to the `:=` that settles\n\
                 them. A positional tuple stands only with the fiber it reads\n\
                 stated after it, `(rho, _, pre): Primitive := p`\n\
                .",
                seq(vec![
                    choice(vec![
                        seq(vec![
                            field("pattern", sym("site_pattern")),
                            colon(),
                            field("type", sym("type_annotation")),
                        ]),
                        field("pattern", sym("bare_list_pattern")),
                        seq(vec![
                            field("pattern", sym("_pattern")),
                            optional(seq(vec![colon(), field("type", sym("type_annotation"))])),
                        ]),
                    ]),
                    ":=".into(),
                    nl(),
                    field("value", expr()),
                ]),
            ),
            def(
                "state_local",
                "`state name := init` — a local transient port, declared inside a\n\
                 block or a function body. It states an\n\
                 optional fiber after `:` as every other binding site does\n\
                .",
                seq(vec![
                    "state".into(),
                    field("name", sym("identifier")),
                    optional(seq(vec![":".into(), field("type", sym("type_annotation"))])),
                    expecting(":=", Stated::StateLocal.label()),
                    nl(),
                    field("value", expr()),
                ]),
            ),
            def(
                "write",
                "`p <- e` — this port takes that expression's value, one batch",
                seq(vec![field("target", sym("identifier")), "<-".into(), nl(), field("value", expr())]),
            ),
            // ---- tokens --------------------------------------------------
            def(
                "qualified_identifier",
                "`fig::panel` — one qualified identifier reaching into a\n\
                 namespace: a used document's, or one the language\n\
                 knows. `::` is the namespace separator everywhere, and it is the\n\
                 only one: a dot between two names is refused with this spelling,\n\
                 and a component is read by its key. The\n\
                 lowering joins the segments into one name and elaboration reads\n\
                 it by what the head names.",
                seq(vec![
                    field("namespace", sym("identifier")),
                    repeat1(seq(vec!["::".into(), field("name", sym("identifier"))])),
                ]),
            ),
            def(
                "template_string",
                "`f\"e = {e:.3} deg\"` and `f\"\"\"…\"\"\"` — the tagged literal, whose\n\
                 holes hold expressions. The `f` stands against the\n\
                 opening quote and tags it, so an untagged literal keeps every\n\
                 meaning it has, braces included. A chunk is the text between the\n\
                 holes; `{{` and `}}` spell a brace; an escape is read as the\n\
                 quoted literal's is, and the triple-quoted form takes its holes\n\
                 with the raw form's other rules. A hole holds any expression of\n\
                 the language, resolved in the enclosing scope as any\n\
                 subexpression is, and states an optional format after `:`.\n\
                 \n\
                 The literal desugars to the join before any node exists, so the\n\
                 graph holds what the operator spelling holds.",
                choice(vec![
                    seq(vec![
                        alias(token(Rule::Pat(p::text("f\"\"\""))), "f\"\"\""),
                        repeat(choice(vec![
                            sym("raw_chunk"),
                            sym("raw_quote"),
                            sym("brace_escape"),
                            sym("substitution"),
                        ])),
                        immediate("\"\"\""),
                    ]),
                    seq(vec![
                        alias(token(Rule::Pat(p::text("f\""))), "f\""),
                        repeat(choice(vec![sym("chunk"), sym("escape"), sym("substitution")])),
                        immediate("\""),
                    ]),
                ]),
            ),
            def(
                "chunk",
                "the text between a quoted tagged literal's holes: everything but\n\
                 the delimiter, the braces a hole and its escape are spelled\n\
                 with, a backslash and a newline",
                immediate(Rule::Pat(p::plus(p::not_class("\"{}\\\n", &[])))),
            ),
            def(
                "raw_chunk",
                "the same, inside `f\"\"\"…\"\"\"`: a backslash and a newline stand\n\
                 for themselves, as in the untagged raw literal, and so does a\n\
                 quotation mark where three do not close the literal; the text\n\
                 reaches the value as written",
                immediate(Rule::Pat(p::plus(p::alt(vec![
                    p::not_class("\"{}", &[]),
                    p::seq(vec![Pat::Ch('"'), p::not_class("\"{}", &[])]),
                    p::seq(vec![Pat::Ch('"'), Pat::Ch('"'), p::not_class("\"{}", &[])]),
                ])))),
            ),
            def(
                "raw_quote",
                "`\"` or `\"\"` inside `f\"\"\"…\"\"\"` where a brace follows — raw text\n\
                 that a raw chunk, whose quotes take a following character, cannot\n\
                 end with. Three quotation marks close the literal instead.",
                seq(vec![
                    not_ahead(immediate("\"\"\"")),
                    immediate(Rule::Pat(p::alt(vec![p::text("\"\""), p::text("\"")]))),
                ]),
            ),
            def(
                "brace_escape",
                "`{{` and `}}` inside `f\"\"\"…\"\"\"` — a brace written as two, since a\n\
                 single brace opens a hole; the raw form has no backslash escape.",
                immediate(Rule::Pat(p::alt(vec![p::text("{{"), p::text("}}")]))),
            ),
            def(
                "escape",
                "`\\n`, `\\\"`, `{{` and `}}` — one character written as two inside\n\
                 `f\"…\"`. The backslash escapes are the quoted literal's\n\
                 and the doubled braces are the tagged literal's own,\n\
                 since a single brace opens a hole.",
                immediate(Rule::Pat(p::alt(vec![
                    p::seq(vec![Pat::Ch('\\'), Pat::Any]),
                    p::text("{{"),
                    p::text("}}"),
                ]))),
            ),
            def(
                "substitution",
                "`{e}` and `{e:.3}` — one hole of a tagged literal: an expression,\n\
                 and the format its value is written in. The configuration zone\n\
                 is left out of the hole's expression, since the colon inside a\n\
                 hole opens the format: a configured value is bound above the\n\
                 literal and named here.",
                seq(vec![
                    immediate("{"),
                    nl(),
                    expecting(
                        field("expr", without("_expression", &["config_expression"])),
                        Stated::TemplateHole.label(),
                    ),
                    optional(field("spec", sym("format_spec"))),
                    nl(),
                    expecting("}", Stated::TemplateClose.label()),
                ]),
            ),
            def(
                "format_spec",
                "`:.3`, `:sci`, `:eng` and `:.3sci` — the format a hole's value is\n\
                 written in: `fmt`'s two keys and nothing more, the digits after\n\
                 the point and the notation. It stands immediately after the\n\
                 expression, with no space before the colon, which is what tells\n\
                 it from the configuration zone's colon. Which texts are formats,\n\
                 a later analysis stage states, so a spelling that is none is named in a\n\
                 sentence rather than left to the closing brace.",
                immediate(Rule::Pat(p::seq(vec![
                    Pat::Ch(':'),
                    p::plus(p::class("._", &[('A', 'Z'), ('a', 'z'), ('0', '9')])),
                ]))),
            ),
            def("identifier", "", Rule::Pat(word)),
            def(
                "number",
                "the fractional part takes at least one digit, so the dots of\n\
                 `0..10` belong to the range and not to the number; `1.0` is the\n\
                 spelling of one as a Real, and `1.` is not a literal",
                number,
            ),
            def("string", "", string),
            def(
                "triple_string",
                "`\"\"\"…\"\"\"` — the multi-line literal. Three quotes win\n\
                 over two by the lexer's longest match, so `\"\"` keeps its meaning.\n\
                 The contents are raw, and the content token states exactly the\n\
                 scan: everything up to the first `\"\"\"`. Its own node carries the\n\
                 prose, which is what `injections.scm` renders as markdown.",
                seq(vec![
                    "\"\"\"".into(),
                    optional(field("text", sym("triple_string_content"))),
                    immediate("\"\"\""),
                ]),
            ),
            def("triple_string_content", "", immediate(Rule::Pat(triple_content))),
            def("boolean", "", choice(vec!["true".into(), "false".into()])),
            def(
                "none",
                "`none` — the None unit kind's sole value: the\n\
                 literal a hover writes at the pointer's leaving, and the one a\n\
                 durable absent port carries in its text.",
                "none".into(),
            ),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The keywords the grammar extracts are the lexer's table, word
    /// for word.
    #[test]
    fn the_grammar_states_the_lexers_keywords() {
        let mut from_lexer: Vec<String> = crate::lexer::KEYWORDS
            .iter()
            .map(|w| w.to_string())
            .collect();
        from_lexer.sort();
        assert_eq!(grammar().keywords(), from_lexer);
    }
}
