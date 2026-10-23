//! The grammar's lexical layer as a Vim syntax file, printed from the
//! same [`Grammar`] value [`crate::print()`] reads. Vim loads
//! `syntax/<lang>.vim` for a buffer whose filetype is `<lang>`, and
//! `ftdetect/<lang>.vim` sets that filetype from the file name, so the
//! two files together color a document with no plugin manager.
//!
//! What is read off the value is in [`crate::lexical`]. Two things are
//! stated here rather than derived: the triple-quoted string prints as
//! a `syn region`, since its text spans lines and a `syn match` reads
//! one, and the highlight links name Vim's standard groups.
//!
//! Order is meaning in a Vim syntax file: where two items match at one
//! position the item defined last wins, so the operators come before
//! the comment, whose `#` is an operator too, and the single-quoted
//! string comes before the triple-quoted region.

use crate::lexical::{self, Lexical};
use crate::pattern::Dialect;
use crate::rule::Grammar;

/// The Vim syntax file of `g`. `lang` is the filetype — `cartan` —
/// which prefixes every syntax group; `constants` names the word
/// tokens that are values rather than keywords.
pub fn vim(g: &Grammar, lang: &str, constants: &[&str]) -> String {
    let lex = Lexical::of(g, constants);
    let d = Dialect::Vim;

    let mut out = String::new();
    out.push_str(&format!(
        "\" Vim syntax file for the {} language.\n",
        g.name
    ));
    out.push_str("\"\n");
    out.push_str(
        "\" Printed by treegram from the host's shared grammar value.\n\
         \" An edit here is\n\
         \" overwritten; edit the grammar and print the file again.\n\
         \"\n\
         \" This file states lexical coloring. A language server is\n\
         \" a separate host integration.\n\n",
    );
    out.push_str("if exists(\"b:current_syntax\")\n  finish\nendif\n\n");

    if !lex.keywords.is_empty() {
        out.push_str(&format!(
            "syn keyword {lang}Keyword {}\n",
            lex.keywords.join(" ")
        ));
    }
    if !lex.constants.is_empty() {
        out.push_str(&format!(
            "syn keyword {lang}Constant {}\n",
            lex.constants.join(" ")
        ));
    }
    out.push('\n');
    if !lex.operators.is_empty() {
        out.push_str(&match_line(
            &format!("{lang}Operator"),
            &lex.operator_regex(d),
            "",
        ));
    }
    if let Some(re) = lexical::regex(g, "number", d) {
        out.push_str(&match_line(&format!("{lang}Number"), &format!("<{re}"), ""));
    }
    if let Some(re) = lexical::regex(g, "string", d) {
        out.push_str(&match_line(&format!("{lang}String"), &re, ""));
    }
    if let Some(re) = lexical::regex(g, "comment", d) {
        out.push_str(&match_line(
            &format!("{lang}Comment"),
            &re,
            " contains=@Spell",
        ));
    }
    if lex.triple_string {
        out.push_str(&format!(
            "syn region {lang}TripleString start=+\"\"\"+ end=+\"\"\"+ contains=@Spell\n"
        ));
    }
    out.push('\n');

    for (group, standard) in [
        ("Keyword", "Keyword"),
        ("Constant", "Constant"),
        ("Operator", "Operator"),
        ("Number", "Number"),
        ("String", "String"),
        ("TripleString", "String"),
        ("Comment", "Comment"),
    ] {
        out.push_str(&format!("hi def link {lang}{group} {standard}\n"));
    }
    out.push_str(&format!("\nlet b:current_syntax = \"{lang}\"\n"));
    out
}

/// The `ftdetect` file that gives a document of this language its
/// filetype. `ext` is the file extension, with no dot.
pub fn ftdetect(lang: &str, ext: &str) -> String {
    format!(
        "\" Give a .{ext} document the {lang} filetype, so that\n\
         \" syntax/{lang}.vim colors it.\n\
         \"\n\
         \" Printed by `cartan grammar --vim-ftdetect`.\n\n\
         autocmd BufRead,BufNewFile *.{ext} set filetype={lang}\n"
    )
}

/// One `syn match` line. A Vim pattern is delimited by whatever
/// character follows the group name, so the delimiter is the first
/// candidate the pattern itself does not hold.
fn match_line(group: &str, regex: &str, tail: &str) -> String {
    let delim = "+/@!;,%&"
        .chars()
        .find(|c| !regex.contains(*c))
        .unwrap_or('+');
    format!("syn match {group} {delim}\\v{regex}{delim}{tail}\n")
}
