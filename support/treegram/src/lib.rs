//! A grammar as a Rust value, read two ways.
//!
//! The grammar is a [`Grammar`]: rules built from the same combinators
//! tree-sitter's `grammar.js` uses — `seq`, `choice`, `repeat`,
//! `optional`, `prec`, `token`, `field` — plus one construct
//! tree-sitter states through precedence annotations and this crate
//! states outright, the expression rule with its operator table
//! ([`Climb`]). Two readers take that value. [`print()`] writes it as a
//! `grammar.js` that `tree-sitter generate` accepts, for the editors.
//! [`parse()`] reads a document against it directly, in Rust, and
//! returns a concrete tree with byte spans on every node — no C, no
//! generated parser, nothing that stops a wasm build.
//!
//! Four further printers state the same value's lexical layer for the
//! editors that read no tree-sitter grammar: [`textmate()`] for VS Code,
//! [`vim()`], [`sublime()`] and [`emacs()`]. Each states what a line-based
//! regex highlighter sees — comments, strings, numbers, keywords,
//! operators — and [`lexical`] holds the reading all four share.
//!
//! The two readers run different algorithms — tree-sitter is GLR, the
//! interpreter here is ordered-choice with precedence climbing — so the
//! grammar includes a few annotations one reader uses and the other
//! ignores. Each is named for its reader in [`Rule`], and a project
//! using this crate holds the two to one tree over a corpus.
//!
//! # Newlines
//!
//! Nothing here treats a newline specially. A grammar that reads a
//! newline as a separator states it as a token and keeps it out of
//! `extras`, then writes where a newline may stand — after an
//! operator, before a closer — with `repeat` of that token, and the
//! same statement reaches both readers. A grammar that reads a
//! newline as whitespace lists it in `extras`.

pub mod emacs;
pub mod lexical;
pub mod parse;
pub mod pattern;
pub mod print;
pub mod query;
pub mod rule;
pub mod sublime;
pub mod textmate;
pub mod tree;
pub mod vim;

pub use emacs::emacs;
pub use lexical::Lexical;
pub use parse::{parse, Parse};
pub use pattern::{Dialect, Pat};
pub use print::print;
pub use query::{NodeType, QueryProblem, Roster};
pub use rule::*;
pub use sublime::sublime;
pub use textmate::textmate;
pub use tree::{line_of, Diag, Field, Node, Said};
pub use vim::{ftdetect, vim};
