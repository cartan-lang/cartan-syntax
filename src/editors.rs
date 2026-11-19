//! The editor packages under `editors/`, printed from the grammar value.
//!
//! [`files`] returns every file of the packages this library can print,
//! by its path from the repository root: the Tree-sitter grammar's
//! source, the Zed extension, the VS Code extension and the Vim,
//! Sublime Text and Emacs definitions. The parser `tree-sitter generate`
//! writes from that source is the one thing this library cannot print;
//! the repository's tests generate it beside these files and hold the
//! committed tree to both.
//!
//! The highlight query's keyword, operator and punctuation lists, the
//! bracket query and the indent query are read off the grammar value.
//! The structural highlights, the outline and the injections are written
//! in `src/queries/` and name the grammar's node types, which the tests
//! check against the generated parser.

use treegram::json::Json;
use treegram::Lexical;

use crate::grammar::{self, grammar};

/// The public repository the packages are published from.
pub const REPOSITORY: &str = "https://github.com/cartan-lang/cartan-syntax";

/// The grammar's directory in the repository, which Zed builds the
/// parser in.
pub const GRAMMAR_PATH: &str = "editors/tree-sitter-cartan";

/// Where Zed fetches the grammar from: a Git repository and a revision
/// of it whose `editors/tree-sitter-cartan` holds the parser.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ZedGrammar {
    pub repository: String,
    pub rev: String,
}

const HIGHLIGHTS: &str = include_str!("queries/highlights.scm");
const OUTLINE: &str = include_str!("queries/outline.scm");
const INJECTIONS: &str = include_str!("queries/injections.scm");

/// The Zed language's directory.
const ZED_LANGUAGE: &str = "editors/zed/languages/cartan";

/// Every editor file this library prints, by its path from the
/// repository root, with Zed fetching the grammar from `zed`.
pub fn files(zed: &ZedGrammar) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = vec![
        ("editors/README.md".into(), readme()),
        (format!("{GRAMMAR_PATH}/grammar.js"), grammar::print()),
        (
            format!("{GRAMMAR_PATH}/tree-sitter.json"),
            tree_sitter_json(),
        ),
        (
            format!("{GRAMMAR_PATH}/src/grammar.json"),
            grammar::print_json(),
        ),
        (
            format!("{GRAMMAR_PATH}/src/scanner.c"),
            grammar::print_scanner(),
        ),
        ("editors/zed/extension.toml".into(), zed_manifest(zed)),
        // The Zed registry reads the licence inside the extension's own
        // directory; the repository's licence is its only source.
        (
            "editors/zed/LICENSE".into(),
            include_str!("../LICENSE").into(),
        ),
        (format!("{ZED_LANGUAGE}/config.toml"), zed_config()),
        (format!("{ZED_LANGUAGE}/highlights.scm"), highlights()),
        (format!("{ZED_LANGUAGE}/brackets.scm"), brackets()),
        (format!("{ZED_LANGUAGE}/indents.scm"), indents()),
        (format!("{ZED_LANGUAGE}/injections.scm"), INJECTIONS.into()),
        (format!("{ZED_LANGUAGE}/outline.scm"), OUTLINE.into()),
        ("editors/vscode/package.json".into(), vscode_package()),
        (
            "editors/vscode/language-configuration.json".into(),
            vscode_language_configuration(),
        ),
        (
            "editors/vscode/syntaxes/cartan.tmLanguage.json".into(),
            grammar::print_textmate(),
        ),
        ("editors/vim/syntax/cartan.vim".into(), grammar::print_vim()),
        (
            "editors/vim/ftdetect/cartan.vim".into(),
            grammar::print_vim_ftdetect(),
        ),
        (
            "editors/sublime/cartan.sublime-syntax".into(),
            grammar::print_sublime(),
        ),
        (
            "editors/emacs/cartan-mode.el".into(),
            grammar::print_emacs(&[]),
        ),
    ];
    out.sort();
    out
}

/// The version every editor package carries: the Cartan product release it
/// ships with, which the library's own crate version does not track.
fn version() -> &'static str {
    "0.13.0"
}

fn json(value: Json) -> String {
    let mut out = value.pretty();
    out.push('\n');
    out
}

fn obj(members: Vec<(&str, Json)>) -> Json {
    Json::Obj(
        members
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect(),
    )
}

fn strs(items: &[&str]) -> Json {
    Json::Arr(items.iter().map(|s| Json::str(s)).collect())
}

/// The bracket pairs the grammar keeps balanced.
fn bracket_pairs() -> Vec<(String, String)> {
    grammar().brackets.clone()
}

/// The Tree-sitter CLI's configuration of the grammar directory, whose
/// version `tree-sitter generate` writes into the parser.
fn tree_sitter_json() -> String {
    json(obj(vec![
        (
            "$schema",
            Json::str(
                "https://tree-sitter.github.io/tree-sitter/assets/schemas/config.schema.json",
            ),
        ),
        (
            "grammars",
            Json::Arr(vec![obj(vec![
                ("name", Json::str("cartan")),
                ("camelcase", Json::str("Cartan")),
                ("scope", Json::str("source.cartan")),
                ("path", Json::str(".")),
                ("file-types", strs(&["cart"])),
            ])]),
        ),
        (
            "metadata",
            obj(vec![
                ("version", Json::str(version())),
                ("license", Json::str("MIT")),
                (
                    "description",
                    Json::str("Cartan grammar for Tree-sitter, printed by cartan-syntax"),
                ),
                ("links", obj(vec![("repository", Json::str(REPOSITORY))])),
            ]),
        ),
    ]))
}

fn zed_manifest(zed: &ZedGrammar) -> String {
    format!(
        r#"id = "cartan"
name = "Cartan"
description = "Cartan syntax highlighting, brackets, indentation and outline."
version = "{version}"
schema_version = 1
authors = ["Cartan contributors"]
repository = "{REPOSITORY}"

[grammars.cartan]
repository = "{repository}"
rev = "{rev}"
path = "{GRAMMAR_PATH}"
"#,
        version = version(),
        repository = zed.repository,
        rev = zed.rev,
    )
}

fn zed_config() -> String {
    let mut out = String::from(
        r##"name = "Cartan"
grammar = "cartan"
path_suffixes = ["cart"]
code_fence_block_name = "cart"
line_comments = ["# "]
tab_size = 4
brackets = [
"##,
    );
    for (open, close) in bracket_pairs() {
        out.push_str(&format!(
            "  {{ start = \"{open}\", end = \"{close}\", close = true, newline = true }},\n"
        ));
    }
    out.push_str(
        "  { start = \"\\\"\", end = \"\\\"\", close = true, newline = false, not_in = [\"string\", \"comment\"] },\n]\n",
    );
    out
}

/// A query list of anonymous node types, one per line under its capture.
fn listed(names: &[String], capture: &str) -> String {
    let mut out = String::from("[\n");
    for n in names {
        out.push_str(&format!("  {}\n", query_string(n)));
    }
    out.push_str(&format!("] @{capture}\n"));
    out
}

fn query_string(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// The highlights: the keyword, operator and punctuation lists read off
/// the grammar value, then the structural captures of `src/queries/`.
fn highlights() -> String {
    let g = grammar();
    let roster = g.roster();
    let lex = Lexical::of(g, grammar::CONSTANTS);
    let anonymous = |s: &String| roster.anonymous.contains(s);
    let keywords: Vec<String> = lex
        .keywords
        .iter()
        .filter(|k| anonymous(k))
        .cloned()
        .collect();
    let delimiters: Vec<String> = [",", ";"]
        .iter()
        .map(|s| s.to_string())
        .filter(anonymous)
        .collect();
    let mut operators: Vec<String> = lex
        .operators
        .iter()
        .filter(|o| anonymous(o) && !delimiters.contains(o))
        .cloned()
        .collect();
    operators.sort();
    let brackets: Vec<String> = bracket_pairs()
        .into_iter()
        .flat_map(|(open, close)| [open, close])
        .collect();
    let mut out =
        String::from("; Printed by cartan-syntax from the grammar value. Do not edit.\n\n");
    out.push_str(&listed(&keywords, "keyword"));
    out.push('\n');
    out.push_str(&listed(&operators, "operator"));
    out.push('\n');
    out.push_str(&listed(&brackets, "punctuation.bracket"));
    out.push('\n');
    out.push_str(&listed(&delimiters, "punctuation.delimiter"));
    out.push('\n');
    out.push_str(HIGHLIGHTS);
    out
}

/// Each bracket pair the grammar keeps balanced, as Zed's matching pair.
fn brackets() -> String {
    let mut out =
        String::from("; Printed by cartan-syntax from the grammar value. Do not edit.\n\n");
    for (open, close) in bracket_pairs() {
        out.push_str(&format!(
            "({} @open {} @close)\n",
            query_string(&open),
            query_string(&close)
        ));
    }
    out
}

/// Each bracket pair indents what it holds and ends at its closer.
fn indents() -> String {
    let mut out =
        String::from("; Printed by cartan-syntax from the grammar value. Do not edit.\n\n");
    for (open, close) in bracket_pairs() {
        out.push_str(&format!(
            "(_ {} {} @end) @indent\n",
            query_string(&open),
            query_string(&close)
        ));
    }
    out
}

fn vscode_package() -> String {
    json(obj(vec![
        ("name", Json::str("cartan")),
        ("displayName", Json::str("Cartan")),
        (
            "description",
            Json::str("Cartan syntax highlighting, printed by cartan-syntax."),
        ),
        ("version", Json::str(version())),
        ("publisher", Json::str("cartan-lang")),
        ("license", Json::str("MIT")),
        (
            "repository",
            obj(vec![
                ("type", Json::str("git")),
                ("url", Json::str(REPOSITORY)),
            ]),
        ),
        ("engines", obj(vec![("vscode", Json::str("^1.75.0"))])),
        ("categories", strs(&["Programming Languages"])),
        (
            "contributes",
            obj(vec![
                (
                    "languages",
                    Json::Arr(vec![obj(vec![
                        ("id", Json::str("cartan")),
                        ("aliases", strs(&["Cartan", "cartan"])),
                        ("extensions", strs(&[".cart"])),
                        ("configuration", Json::str("./language-configuration.json")),
                    ])]),
                ),
                (
                    "grammars",
                    Json::Arr(vec![obj(vec![
                        ("language", Json::str("cartan")),
                        ("scopeName", Json::str("source.cartan")),
                        ("path", Json::str("./syntaxes/cartan.tmLanguage.json")),
                    ])]),
                ),
            ]),
        ),
    ]))
}

fn vscode_language_configuration() -> String {
    let pairs = bracket_pairs();
    let pair =
        |(open, close): &(String, String)| Json::Arr(vec![Json::str(open), Json::str(close)]);
    let quote = ("\"".to_string(), "\"".to_string());
    let mut closing: Vec<Json> = pairs
        .iter()
        .map(|(open, close)| obj(vec![("open", Json::str(open)), ("close", Json::str(close))]))
        .collect();
    closing.push(obj(vec![
        ("open", Json::str("\"")),
        ("close", Json::str("\"")),
        ("notIn", strs(&["string", "comment"])),
    ]));
    let mut surrounding: Vec<Json> = pairs.iter().map(pair).collect();
    surrounding.push(pair(&quote));
    json(obj(vec![
        ("comments", obj(vec![("lineComment", Json::str("#"))])),
        ("brackets", Json::Arr(pairs.iter().map(pair).collect())),
        ("autoClosingPairs", Json::Arr(closing)),
        ("surroundingPairs", Json::Arr(surrounding)),
    ]))
}

fn readme() -> String {
    format!(
        r#"# Editor support for Cartan

Every file under `editors/` is generated from the grammar value in
`src/grammar.rs`; do not edit them. The parser under
`tree-sitter-cartan/src/` is written by the `tree-sitter-generate` crate in
the repository's tests, and `src/scanner.c` by treegram. A test fails when a
committed file differs from the printed one. To print them again, from the
repository root:

```sh
CARTAN_SYNTAX_BLESS=1 cargo test --test editors
```

The same command writes a local Zed development copy to `out/zed/`, which
Git ignores. Its grammar comes from this checkout through a `file://` URL at
the commit `HEAD` names, so commit a changed parser before writing it.

## Zed

The committed `zed/extension.toml` fetches the grammar from
{REPOSITORY} at the revision its `rev` names, in
`{GRAMMAR_PATH}`. The test keeps the `rev` already committed, since a file
cannot name the commit that holds it. When the parser changes, commit it,
then pin the revision and commit again:

```sh
CARTAN_SYNTAX_ZED_REV=$(git rev-parse HEAD) CARTAN_SYNTAX_BLESS=1 cargo test --test editors
```

To use the extension locally, write the development copy as above, then in
Zed run `zed: install dev extension` and choose `out/zed`. Files ending in
`.cart` and Markdown code fences tagged `cart` are highlighted. No language
server is registered.

## VS Code

Copy or link `vscode/` into the extensions directory as an unpacked
extension, for example to `~/.vscode/extensions/cartan-lang.cartan-{version}`,
and restart VS Code.

## Vim

Add `vim/` to the runtime path, for example with
`set runtimepath+=/path/to/cartan-syntax/editors/vim` in `.vimrc`.

## Sublime Text

Copy `sublime/cartan.sublime-syntax` into the `Packages/User` directory.

## Emacs

Add `emacs/` to `load-path` and `(require 'cartan-mode)`.
"#,
        version = version(),
    )
}
