//! The grammar's lexical layer as a Sublime Text syntax definition,
//! printed from the same [`Grammar`] value [`crate::print()`] reads.
//! Sublime reads a `.sublime-syntax` file — YAML, a list of contexts,
//! each a list of matches — from any package directory, so dropping
//! the file into `Packages/<Name>/` colors a document with nothing
//! else installed.
//!
//! Sublime matches with Oniguruma, so the regexes are the ones the
//! TextMate printer states. What is read off the value is in
//! [`crate::lexical`]; the triple-quoted string is the one construct
//! stated by name rather than derived, and it prints as a context of
//! its own, since its text spans lines and a `match` reads one.

use crate::lexical::{self, Lexical};
use crate::pattern::Dialect;
use crate::rule::Grammar;

/// The Sublime Text syntax of `g`. `scope` is the root scope —
/// `source.cartan` — whose last segment suffixes every inner scope;
/// `ext` is the file extension, with no dot; `constants` names the
/// word tokens that are values rather than keywords.
pub fn sublime(g: &Grammar, scope: &str, ext: &str, constants: &[&str]) -> String {
    let lang = scope.rsplit('.').next().unwrap_or(scope);
    let lex = Lexical::of(g, constants);
    let d = Dialect::Oniguruma;

    let mut out = String::from("%YAML 1.2\n---\n");
    out.push_str(&format!(
        "# Sublime Text syntax for the {} language.\n\
         #\n\
         # Printed by treegram from the host's shared grammar value.\n\
         # An edit here is\n\
         # overwritten; edit the grammar and print the file again.\n\
         #\n\
         # This file states lexical coloring. A language server is\n\
         # a separate host integration.\n",
        g.name
    ));
    out.push_str(&format!("name: {}\n", g.name));
    out.push_str(&format!("file_extensions:\n  - {ext}\n"));
    out.push_str(&format!("scope: {scope}\n"));
    out.push_str("contexts:\n  main:\n");

    if lex.triple_string {
        out.push_str(&format!(
            "    - match: '\"\"\"'\n      scope: string.quoted.triple.{lang}\n      \
             push: triple_string\n"
        ));
    }
    for (rule, name) in [
        ("string", format!("string.quoted.double.{lang}")),
        ("comment", format!("comment.line.number-sign.{lang}")),
    ] {
        if let Some(re) = lexical::regex(g, rule, d) {
            out.push_str(&entry(&re, &name));
        }
    }
    if !lex.constants.is_empty() {
        out.push_str(&entry(
            &format!("\\b(?:{})\\b", lex.constants.join("|")),
            &format!("constant.language.{lang}"),
        ));
    }
    if let Some(re) = lexical::regex(g, "number", d) {
        out.push_str(&entry(
            &format!("\\b{re}"),
            &format!("constant.numeric.{lang}"),
        ));
    }
    if !lex.keywords.is_empty() {
        out.push_str(&entry(
            &format!("\\b(?:{})\\b", lex.keywords.join("|")),
            &format!("keyword.control.{lang}"),
        ));
    }
    if !lex.operators.is_empty() {
        out.push_str(&entry(
            &lex.operator_regex(d),
            &format!("keyword.operator.{lang}"),
        ));
    }
    if lex.triple_string {
        out.push_str(&format!(
            "  triple_string:\n    - meta_scope: string.quoted.triple.{lang}\n    \
             - match: '\"\"\"'\n      pop: true\n"
        ));
    }
    out
}

/// One entry of the `main` context: a pattern and the scope it colors.
fn entry(regex: &str, scope: &str) -> String {
    format!("    - match: {}\n      scope: {scope}\n", yaml_str(regex))
}

/// A string as a single-quoted YAML scalar, where a quote of its own
/// doubles.
fn yaml_str(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}
