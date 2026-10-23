//! The grammar's lexical layer as an Emacs major mode, printed from
//! the same [`Grammar`] value [`crate::print()`] reads. The file is a
//! complete `.el` — a syntax table, the font-lock rules, the
//! `auto-mode-alist` entry and an eglot registration — so putting it
//! on the load path and requiring it is the whole installation.
//!
//! Emacs colors comments and strings from the syntax table rather than
//! from font-lock patterns, so the comment and string rules print as
//! syntax-table entries and a `syntax-propertize` rule; what font-lock
//! states is the keyword, constant, builtin and number lists, read off
//! the value in [`crate::lexical`]. Operators print in no list: Emacs
//! colors none by convention, and a face for them exists only in
//! recent versions.

use crate::lexical::{self, Lexical};
use crate::pattern::Dialect;
use crate::rule::Grammar;

/// The Emacs major mode of `g`, as one Elisp file. `lang` is the mode's
/// name — `cartan`, giving `cartan-mode` — `ext` is the file
/// extension with no dot, `constants` names the word tokens that are
/// values rather than keywords, `builtins` names the callables the
/// language confers, and `server` is the language server's command
/// line, which eglot starts.
pub fn emacs(
    g: &Grammar,
    lang: &str,
    ext: &str,
    constants: &[&str],
    builtins: &[&str],
    server: &[&str],
) -> String {
    let lex = Lexical::of(g, constants);
    let mut out = String::new();
    out.push_str(&format!(
        ";;; {lang}-mode.el --- Major mode for the {} language -*- lexical-binding: t; -*-\n\n",
        g.name
    ));
    out.push_str(
        ";; Printed by treegram from the host's shared grammar value.\n\
         ;; An edit here is\n\
         ;; overwritten; edit the grammar and print the file again.\n\n\
         ;;; Commentary:\n\n\
         ;; Comments and strings are colored from the syntax table, the\n\
         ;; triple-quoted string through a `syntax-propertize' rule that\n\
         ;; reads its first quote as a generic string fence.  Font-lock\n\
         ;; states the keywords, the constants, the builtins and the\n\
         ;; numbers.  Structural coloring comes from the language server,\n\
         ;; which eglot starts where eglot is installed.\n\n\
         ;;; Code:\n\n",
    );

    out.push_str(&list(
        &format!("{lang}-keywords"),
        &lex.keywords,
        "The language's keywords.",
    ));
    out.push_str(&list(
        &format!("{lang}-constants"),
        &lex.constants,
        "The word tokens that are values rather than keywords.",
    ));
    let builtins: Vec<String> = builtins.iter().map(|s| s.to_string()).collect();
    out.push_str(&list(
        &format!("{lang}-builtins"),
        &builtins,
        "The callables the language confers, in scope with no import.",
    ));

    out.push_str(&format!(
        "(defvar {lang}-mode-syntax-table\n  \
         (let ((table (make-syntax-table)))\n    \
         (modify-syntax-entry ?# \"<\" table)\n    \
         (modify-syntax-entry ?\\n \">\" table)\n    \
         (modify-syntax-entry ?\\\" \"\\\"\" table)\n    \
         (modify-syntax-entry ?\\\\ \"\\\\\" table)\n    \
         (modify-syntax-entry ?_ \"_\" table)\n    \
         table)\n  \
         \"Syntax table for `{lang}-mode'.\")\n\n"
    ));

    if lex.triple_string {
        out.push_str(&format!(
            "(defconst {lang}--syntax-propertize\n  \
             (syntax-propertize-rules\n   \
             (\"\\\\(\\\"\\\\)\\\\(\\\"\\\\)\\\\(\\\"\\\\)\" (1 \"|\") (2 \".\") (3 \".\")))\n  \
             \"Read a triple quote as one generic string fence.\")\n\n"
        ));
    }

    out.push_str(&format!("(defconst {lang}-font-lock-keywords\n  (list\n"));
    out.push_str(&format!(
        "   (cons (regexp-opt {lang}-keywords 'symbols) 'font-lock-keyword-face)\n"
    ));
    out.push_str(&format!(
        "   (cons (regexp-opt {lang}-constants 'symbols) 'font-lock-constant-face)\n"
    ));
    out.push_str(&format!(
        "   (cons (regexp-opt {lang}-builtins 'symbols) 'font-lock-builtin-face)"
    ));
    if let Some(re) = lexical::regex(g, "number", Dialect::Emacs) {
        out.push_str(&format!(
            "\n   (cons {} 'font-lock-constant-face)",
            lisp_str(&format!("\\_<{re}"))
        ));
    }
    out.push_str(&format!(")\n  \"Font-lock rules for `{lang}-mode'.\")\n\n"));

    out.push_str(&format!(
        ";;;###autoload\n\
         (define-derived-mode {lang}-mode prog-mode \"{}\"\n  \
         \"Major mode for editing {} documents.\"\n  \
         :syntax-table {lang}-mode-syntax-table\n  \
         (setq-local comment-start \"# \")\n  \
         (setq-local comment-start-skip \"#+[ \\t]*\")\n  \
         (setq-local font-lock-defaults '({lang}-font-lock-keywords))\n  \
         (setq-local indent-tabs-mode nil)",
        g.name, g.name
    ));
    if lex.triple_string {
        out.push_str(&format!(
            "\n  (setq-local syntax-propertize-function {lang}--syntax-propertize)"
        ));
    }
    out.push_str(")\n\n");

    out.push_str(&format!(
        ";;;###autoload\n\
         (add-to-list 'auto-mode-alist '(\"\\\\.{ext}\\\\'\" . {lang}-mode))\n\n"
    ));

    let cmd: Vec<String> = server.iter().map(|s| lisp_str(s)).collect();
    out.push_str(&format!(
        "(with-eval-after-load 'eglot\n  \
         (add-to-list 'eglot-server-programs '({lang}-mode {})))\n\n",
        cmd.join(" ")
    ));

    out.push_str(&format!(
        "(provide '{lang}-mode)\n;;; {lang}-mode.el ends here\n"
    ));
    out
}

/// One `defconst` holding a list of names, one per line where there
/// are many.
fn list(name: &str, names: &[String], doc: &str) -> String {
    let mut out = format!("(defconst {name}\n  '(");
    let mut width = 4;
    for (i, n) in names.iter().enumerate() {
        let s = lisp_str(n);
        if i > 0 {
            if width + s.len() + 1 > 68 {
                out.push_str("\n    ");
                width = 4;
            } else {
                out.push(' ');
                width += 1;
            }
        }
        out.push_str(&s);
        width += s.len();
    }
    out.push_str(&format!(")\n  \"{doc}\")\n\n"));
    out
}

/// A string as Elisp spells it.
fn lisp_str(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
