---
id: component
title: Syntax component interface
kind: reference
status: accepted
implementation: partial
topics: [syntax, interface, tooling]
---

# Syntax component interface

This component owns the source grammar, grammar parsing, source locations and
syntax-only tooling. It has no evaluator, runtime values, semantic type checker,
name resolver, graph constructor or typed computational intermediate representation.

## Parsing

`parse_document(&str)` reads the source-file grammar. `parse_expression(&str)`
reads an expression without changing its source offsets. Both return a
`SyntaxTree` with:

- `source`: owned UTF-8 source text.
- `root`: a concrete syntax tree node and its children.
- `diagnostics`: recoverable grammar diagnostics.

A concrete syntax tree (CST) retains written structure, including punctuation,
comments and grammar-specific nodes. Each node has a kind, optional field label,
named/extra flags, children and a half-open byte span. `SyntaxTree::text` returns
the source slice at a node's offsets when those offsets are within the text.
The helper does not establish that an arbitrary supplied node originated in
this particular tree.

`has_errors()` checks diagnostics and error nodes. A tree can be returned for
malformed or incomplete input so an editor can inspect the remaining structure.

## Diagnostics

`Diagnostic` preserves a byte `Span`, found text, whether input is incomplete,
`DiagnosticKind` and `unclosed`. A kind is an expected-token set, an unexpected token,
a stated grammar rule with its refusal flag and captured fields, or the nesting
limit below. A captured
field has a name and span; it is not a rendered error sentence. `unclosed` is
the innermost bracket open on the parse path at the furthest failure when its
closer was among the expected tokens: the opener's span, the opening and closing
text and the grammar node kind that opened it, such as `list`. The parser keeps
these brackets per sequence, so a remembered rule's match never leaks one, and
acceptance is unaffected. A rule's own stated diagnostic carries none.

Nesting has a limit. No tree is deeper than `MAX_DEPTH` nodes on its longest
path from the root to a token, the root and the token included
(`Node::height`); a binding `x = ((1))` takes four besides its parentheses.
Text nesting deeper is `DiagnosticKind::TooDeep { limit }` on every entry
point, before parsing can exhaust the stack. A node taller than the limit is
refused where it is built, the diagnostic spanning the first token below the
limit in text order; and the parser refuses to read a level of nesting past the
limit where it builds no node, as in a run of brackets that never closes, the
diagnostic standing where it went past. Each operand of a chain of a
left-grouping operator nests one level, as it does in the tree, so a long sum
reaches the limit too. The refusal is final for the item that holds it, which
becomes an `ERROR` node. The limit bounds the stack that parsing, normalization
and every recursive walk of a tree take, and is chosen to be safe on a 2 MB
thread in a debug build; the [depth tests](../tests/depth.rs) parse at it and
beyond it on such a thread. `highlight` never descends, so it colors text of
any depth.

The owner requires diagnostics and errors to remain typed variants with payloads,
source spans and underlying causes until one top-level formatter in a central
location per standalone application. Lower layers must not generate diagnostic
sentences, collapse causes into strings or classify errors by wording. The public
`Rule` enumeration identifies grammar-specific failures; consumers inspect that
identity rather than diagnostic prose. Wording assertions belong only to formatter
tests. Other tests check structured identity, payload, location, cause and status.
External process streams remain opaque output, not a source for string classification.

The Rust CST/AST APIs already retain structured diagnostics. Presentation lives
in two support packages that the parser library does not depend on:

- `cartan-diagnostics` (`support/diagnostics`) is source-independent. A `Report`
  holds a stable `code`, severity, message, primary and related `Label`s at a
  `SourceLocation` (an index into the caller's source snapshot table plus a byte
  span), notes and JSON `fields`. `render` prints ANSI-free text with source
  excerpts; columns count Unicode scalar values and tabs expand to four-column
  stops, while invalid spans print their byte range rather than a guessed caret.
  `to_json` gives a stable envelope with exact byte spans and source names,
  marking non-UTF-8 names lossy. It has no Syntax, Graph or CLI dependency.
- `cartan-syntax-diagnostics` (`support/syntax-diagnostics`) provides
  `syntax(&ast::Error, source) -> Report`, the one mapping from Syntax grammar
  and form errors to `syntax.*` codes, wording and fields. It reads no files;
  `source` is the caller's table index.

Wording follows one policy, owned by that adapter. A headline reads
`expected X, found Y` where X can be stated reliably, and otherwise
`unexpected Y`. Y is a quoted found token, `end of line` or `end of file`.
For a grammar error X summarizes only the tokens the parser tried at that
position: closing tokens are named exactly, "an expression" appears only where a
number literal could begin, "a name or pattern" only at a binder, continuation
operators are dropped, and any unrecognized token gives up the summary rather
than guessing. An `unclosed` bracket adds a related label at its opener, such as
"List opened here", and an `unclosed` field. Where the text ends or a different
closing bracket stands, the code becomes `syntax.unclosed_delimiter` and the
headline reads "expected `]` to close this List, found end of file". Elsewhere
the token found may instead lack a separator, so the headline stays. Stated and
form rules state the problem; at most one note gives a
repair the rule proves. The end-of-file note is omitted when the headline already
says `end of file`. Codes, spans, the complete `expected` list, `found`,
`incomplete` and captures remain unchanged in report fields; prose is never a
machine interface. Context the error does not carry, such as the enclosing
binding, is never invented.

Downstream tools and this CLI use the same adapter and renderer, so the same
malformed bytes under the same source name print the same diagnostic. Callers
own color and streams. The CLI carries file-read, parse and stdout failures as
`CliError` variants; I/O variants retain the original `io::Error`, and parse
failures retain their path, exact source text and complete AST errors.
[The CLI formatter](../cli/src/diagnostic.rs) renders each parse error through
the shared packages, joined by newlines on stderr; with color it highlights only
a report's leading `error`. Other failures take an `error:` prefix. AST debug
output remains an inspection format, not a diagnostic protocol.

## Syntax tooling

The `cartan-syntax-cli` workspace member supplies a `cartan-syntax` binary. Its
dependency points to the root parser library; the library has no CLI dependency.
The workspace sets `default-members = ["cli"]`, matching codegen's package
arrangement. The selected package has one binary, so no `default-run` is needed.
The binary disables rustdoc output to avoid colliding with the same-named
library's documentation; CLI usage lives in its help.

The [Clap builder tree](../cli/src/command.rs) is the sole command definition.
Help flags, the generated outline, `--describe-cli`, `--color` and the `--json`
envelope come from `cartan-cli-support` (`support/cli`), which every component
CLI shares.
The root command takes `SOURCE [BINDING|UPDATE]` and the flags `--ast` and
`--json`, each requiring a source; there are no subcommands. The whole document
is always parsed and normalized: `SOURCE: valid syntax` (status 0) or the shared
rendered diagnostics on stderr (status 1). A selector is parsed by
[`selection::Selector`](ast-interface.md#selecting-one-item) before the file is
read, so a malformed one is a usage error (status 2); a given selector, or
`main` by default for `--ast`, must then name exactly one top-level item, or the
typed selection diagnostic fails with status 1. `--ast` prints the selected
item as an outline, and `--json` prints the check result object with the same
`selection` and, under `--ast`, the same `item`. There is no whole-document
syntax-tree view. Usage lives in the CLI help.

Both syntax-tree views print one value: the CLI enables the library's optional
`serde` feature, which derives `Serialize` for the AST and `Span` only when
requested, so the library's default build gains no registry dependency.
[The outline](../cli/src/tree.rs) is a presentation of that value. It names
each node's variant, written text and `line:column` range, with columns counting
Unicode scalar values; the JSON keeps exact byte spans and declaration field
order. Color adds escapes only, and JSON is never colored. The check result's
diagnostics use the shared `cartan_diagnostics::to_json` envelope through the
CLI formatter. Every `--json` outcome uses the shared result envelope
`{schema_version: 1, status: "ok" | "error", source, diagnostics}` plus any
payload (`selection`, `item`). An unreadable file is a result on stdout with
`status: "error"` and one `source.read` diagnostic whose fields carry the path,
the I/O error `kind` and any OS error code; the status is 1 and stderr stays
empty. In text mode it is an ordinary CLI error on stderr.

The command takes `-h` for concise help and `--help` for help followed by a
full command outline generated from the tree; there is no `help` subcommand. Help
and version arguments are explicit tree entries, which lets the color probe edit
the unbuilt tree (Clap does not refresh option lookup after a built command is
edited). The probe also relaxes `required` and keeps `ignore_errors`, so neither
a missing operand nor a conditional requirement drops earlier matches.

The outline lists each command path depth first, then its visible arguments in
definition order. A required positional prints `<NAME>`, an optional one `[NAME]`;
options print `--long <NAME>` or a bare flag; repeatable arguments end with `...`.
Value arguments note `default:` and `choices:`. Help/version entries, globals
inherited below the root and argument relationships such as `requires_if` are
omitted. The workspace adapter builds its outline from `--describe-cli` with the
same rules.
Bare groups and help exit 0 on stdout, usage errors exit 2 on stderr, and domain
failures retain exit 1 and typed causes until the formatter. File operands use
OS-native paths and `--` terminates option parsing. Color accepts auto/always/never;
explicit choices take precedence over `NO_COLOR`, and auto uses the destination's
terminal state. AST output and metadata JSON never contain presentation escapes.

`--describe-cli` exports the built tree using the
[version-one shared schema](../../docs/records/component-apps.md#rust-authority-and-command-metadata).
Visible command/argument aliases, inherited globals, value arity, defaults and
choices are derived through Clap introspection. Custom validation/relationships
remain Rust responsibilities. Each command's `notes` is its after-help prose
(the selector and `--json` explanation here), without the generated outline. The
workspace `cartdev syntax` adapter is driven only by this metadata and the
`--json` results; Syntax ships no Python package.

The CLI alone depends on Clap (default features off; `std`, `help`, `usage`,
`error-context` and `color` on) and `serde_json`; `Cargo.lock` records the
versions. The parser library's dependencies are unchanged by the CLI.

[CLI process tests](../cli/tests/cli.rs) cover help and usage statuses, checks
against the shared renderer, selection, JSON results, the readable tree, color
and OS path operands, including discovery from `--describe-cli`. Tree-sitter parser generation and execution are
untested.

`grammar::grammar()` returns the shared grammar value. The same value drives:

- `grammar::print()`: Tree-sitter grammar JavaScript.
- `print_textmate()`: lexical highlighting grammar.
- `print_vim()` and `print_vim_ftdetect()`: Vim syntax and file detection.
- `print_sublime()`: Sublime syntax definitions.
- `print_emacs(builtin_names)`: Emacs source using caller-supplied builtin names.

The generic `treegram` package interprets and prints this grammar. Tooling does
not import a language builtin registry or require compiler execution. These
generators do not provide a language server or semantic completion features.

## Prompt support

[`prompt`](../src/prompt.rs) supplies the facts an interactive prompt needs, as
data. `completeness(text)` reads the text as a document and as one expression:
`Complete` when either has no grammar diagnostic, `Incomplete` when neither
does but one stopped at the end of the text (the typed `incomplete` flag), and
`Invalid` otherwise. It is a grammar judgment; normalization errors appear when
the text is parsed with `ast::parse_document`, rendered through the shared
diagnostic packages.

`highlight(text)` returns non-overlapping byte spans with a `HighlightClass`
(`Keyword`, `Constant`, `Number`, `String`, `Comment`, `Operator`, `Punctuation`
or `Name`). It scans lexically, so partially typed text colors: words, constants
and operators come from the grammar's lexical layer, the same sets the editor
grammars print, and only comments, strings and numbers are scanned by their
token shapes. Template holes color as part of their literal, and an unterminated
literal colors to its line's end. The reserved words are `KEYWORDS`.

An unfinished call is incomplete rather than refused: the call-head refusal
applies only to a complete argument list, and the positional-argument refusal
only where the call closes after it, so `f(1` and `f(1, 2` followed by another
line report the open `(`.

## Acceptance limits

Grammar parsing is one layer of syntax processing. The separate
[`ast` interface](ast-interface.md) normalizes a supported subset into an
abstract syntax tree (AST), the representation of source constructs used for
further analysis. It includes file/namespace use heads, selectors, aliases and
qualified value, call and type names without loading files or resolving names,
and documentation strings on the document and its declarations. It refuses
alias-plus-selector combinations and written comparison/range chains, and requires
value bodies to have exactly one result. The CST API still returns grammar
trees for those forms without performing the AST checks.

Consequently, `has_errors() == false` is not a claim of complete language
validity. There is also no promise that consumers can analyze or execute every
construct recognized by either API.

The five fixture programs parse and retain their expected loops and fields.
Tests also pin precedence, typed versus untyped patterns, scalar versus list
binders, newline rules, byte spans, Unicode handling and diagnostic identity.
The fixture copies are tests, not a second authoritative workload collection.
They hold the five workload programs; Conway's subtraction places its operator at
the end of the preceding line, as the newline rule requires.

Inside a body, `i, j := [10, 2]` and `[i, j] := [10, 2]` both parse as one
`local` with two identifiers in its `pattern` field. Their CST kinds remain
`bare_list_pattern` and `list_pattern`, respectively; the AST normalizes both
to `PatternKind::List` while preserving the original spans. At document scope,
pattern bindings use `=`; both forms with `:=` report `Rule::PatternValue` at
the colon.
The [syntax tests](../tests/syntax.rs) verify these structures and byte spans.

JavaScript syntax of the generated Tree-sitter grammar is checked with Node.
Actual Tree-sitter parser generation and execution are untested because its CLI
is unavailable in this environment. There is no full editor-conformance claim.

## Licensing

All packages are licensed under the MIT License; see [LICENSE](../LICENSE).
Manifests declare `MIT` and disable crates.io publication.

## Consumer integration

Graph consumes the source AST and `selection`; Graph, Codegen and Compile use
the diagnostic packages. Each owns its adapter and locked Git dependency,
following the [local dependency contract](ast-interface.md#local-dependency-delivery);
its `Cargo.lock` is the record of the revision it consumes. Name resolution,
overload selection, type checking and graph construction remain outside syntax. Integration and execution evidence belongs to the
consuming components.

## Current limitations

- Only `while` blocks normalize as update blocks; `every` blocks, namespace
  bodies, default arguments, function-header
  configuration and Tap bodies parse to a CST but are unsupported in the AST.
- Generated Tree-sitter JavaScript passes a syntax check; building and running a
  Tree-sitter parser is untested here.
