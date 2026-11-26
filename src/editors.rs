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
    out.extend(
        vscode_files()
            .into_iter()
            .map(|(path, text, _)| (format!("{VSCODE}/{path}"), text)),
    );
    out.sort();
    out
}

/// The VS Code extension's directory.
const VSCODE: &str = "editors/vscode";

/// The Tree-sitter release the VS Code package builds its parser with and
/// runs it on: the release of the crates that generate and test the
/// parser here, so the WebAssembly parser and its runtime share their ABI.
pub const TREE_SITTER: &str = "0.27.2";

/// The WebAssembly parser the packaging step builds in the VS Code
/// extension's directory from the committed parser source.
const VSCODE_PARSER: &str = "tree-sitter-cartan.wasm";

/// The files of the Tree-sitter runtime the package holds, from the
/// installed dependency: the packager takes only the files its manifest
/// lists, dependencies included.
const VSCODE_RUNTIME: [&str; 4] = [
    "node_modules/web-tree-sitter/package.json",
    "node_modules/web-tree-sitter/LICENSE",
    "node_modules/web-tree-sitter/web-tree-sitter.cjs",
    "node_modules/web-tree-sitter/web-tree-sitter.wasm",
];

/// The files printed into the VS Code extension's directory besides its
/// manifest, by path from it, each with whether the package holds it.
fn vscode_files() -> Vec<(&'static str, String, bool)> {
    vec![
        // The registries read the licence inside the extension's own
        // directory; the repository's licence is its only source.
        ("LICENSE", include_str!("../LICENSE").into(), true),
        (
            "language-configuration.json",
            vscode_language_configuration(),
            true,
        ),
        (
            "syntaxes/cartan.tmLanguage.json",
            grammar::print_textmate(),
            true,
        ),
        // The extension reads the query Zed reads, against the same parser.
        ("queries/highlights.scm", highlights(), true),
        (
            "extension.js",
            include_str!("vscode/extension.js").into(),
            true,
        ),
        // Run before packaging and not shipped.
        ("check.js", include_str!("vscode/check.js").into(), false),
    ]
}

/// The version every editor package carries: the Cartan product release it
/// ships with, which the library's own crate version does not track.
fn version() -> &'static str {
    "0.15.0"
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
            Json::str("Cartan syntax highlighting from its Tree-sitter grammar, printed by cartan-syntax."),
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
        ("engines", obj(vec![("vscode", Json::str("^1.90.0"))])),
        ("main", Json::str("./extension.js")),
        (
            "files",
            Json::Arr(
                vscode_files()
                    .into_iter()
                    .filter(|(_, _, packed)| *packed)
                    .map(|(path, _, _)| Json::str(path))
                    .chain([VSCODE_PARSER].into_iter().chain(VSCODE_RUNTIME).map(Json::str))
                    .collect(),
            ),
        ),
        (
            "scripts",
            obj(vec![(
                "vscode:prepublish",
                Json::str(&format!(
                    "npx --yes tree-sitter-cli@{TREE_SITTER} build --wasm ../{grammar} \
                     -o {VSCODE_PARSER} && node check.js",
                    grammar = GRAMMAR_PATH.trim_start_matches("editors/"),
                )),
            )]),
        ),
        (
            "dependencies",
            obj(vec![("web-tree-sitter", Json::str(TREE_SITTER))]),
        ),
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

The files generated under `editors/` come from the grammar value in
`src/grammar.rs`. The repository's tests use `tree-sitter-generate` to write
the parser under `tree-sitter-cartan/src/`, and treegram writes `src/scanner.c`.
Regenerate these files from the repository root.

```sh
CARTAN_SYNTAX_BLESS=1 cargo test --test editors
```

The generator checks or updates its declared files and leaves files created
by editors untouched, including Zed's downloaded checkout and compiled grammar.
Remove obsolete generated files explicitly.

The same command writes a local Zed development copy to `out/zed/`, which
Git ignores. Its grammar comes from this checkout through a `file://` URL at
the commit `HEAD` names, so commit a changed parser before writing it.

## Zed

The committed `zed/extension.toml` fetches the grammar from
{REPOSITORY} at the revision its `rev` names, in
`{GRAMMAR_PATH}`. Regeneration keeps the existing `rev` by default. When
the parser changes, commit it, then generate the revision pin separately.

```sh
CARTAN_SYNTAX_ZED_REV=$(git rev-parse HEAD) CARTAN_SYNTAX_BLESS=1 cargo test --test editors
```

Run the pinned-parser check before committing the updated manifest.

```sh
cargo test --offline --test editors zed_queries_read_against_the_pinned_parser -- --ignored --exact
```

This check compiles the parser from the manifest's revision and loads the
shipped Zed queries against it. It reads only local Git objects and fails if
the revision is unavailable. The ordinary editor tests use the current grammar
and can also run from a source archive.

To use the extension locally, write the development copy as above, then in
Zed run `zed: install dev extension` and choose `out/zed`. Files ending in
`.cart` and Markdown code fences tagged `cart` are highlighted. No language
server is registered.

## VS Code

The extension colours lexical tokens with a TextMate grammar printed from
the grammar value, and colours names by their role in the syntax tree with
semantic tokens: `vscode/extension.js` runs the Tree-sitter parser, built
to WebAssembly, with the highlight query Zed uses. Package it with Node; no
account is needed.

```sh
cd editors/vscode
npm install --no-package-lock
npx @vscode/vsce package
```

Packaging first runs `vscode:prepublish`: the Tree-sitter CLI builds
`tree-sitter-cartan.wasm` from `tree-sitter-cartan/`, fetching its compiler
on first use, and `check.js` requires the expected semantic tokens from a
short fixture. The package is `cartan-{version}.vsix`. Install it with
**Extensions: Install from VSIX** in the Command Palette, or
`code --install-extension cartan-{version}.vsix`. VS Code does not update
an extension installed this way. The dependencies, the built parser and the
package are ignored by Git.

Before publishing a package, run the check inside it, so that a file the
package lacks fails rather than being found in this directory: unzip the
`.vsix` into an empty folder, copy `check.js` into its `extension/` folder,
and run `node check.js` there.

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
