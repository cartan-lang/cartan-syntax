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
`DiagnosticKind` and `unclosed`. A kind is an expected-token set, an unexpected token, or
a stated grammar rule with its refusal flag and captured fields. A captured
field has a name and span; it is not a rendered error sentence. `unclosed` is
the innermost bracket open on the parse path at the furthest failure when its
closer was among the expected tokens: the opener's span, the opening and closing
text and the grammar node kind that opened it, such as `list`. The parser keeps
these brackets per sequence, so a remembered rule's match never leaks one, and
acceptance is unaffected. A rule's own stated diagnostic carries none.

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

The Python development app is not yet fully migrated: its process service embeds
launch/timeout messages in `stderr`, configuration failures use message-bearing
`ValueError`, and HTTP validation/status presentation is spread through the
application. Its version-one JSON envelope transports opaque child streams and
cannot reconstruct typed Rust diagnostics. These are existing gaps, not exceptions
to the rule; [the diagnostic work item](work/diagnostic-formatting.md) records their
scope. Migrating them requires an explicit transport/formatter design that preserves
current CLI/web behavior and underlying causes. No message matching is permitted
as an adapter.

## Syntax tooling

The `cartan-syntax-cli` workspace member supplies a `cartan-syntax` binary. Its
dependency points to the root parser library; the library has no CLI dependency.
The workspace sets `default-members = ["cli"]`, matching codegen's package
arrangement. The selected package has one binary, so no `default-run` is needed.
The binary disables rustdoc output to avoid colliding with the same-named
library's documentation; CLI usage lives in its help and the README.
See the [README](../README.md) for source inspection and library-example commands.

The [Clap builder tree](../cli/src/command.rs) is the sole command definition.
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
syntax-tree view. Usage lives in the [README](../README.md).

Both syntax-tree views print one value: the CLI enables the library's optional
`serde` feature, which derives `Serialize` for the AST and `Span` only when
requested, so the library's default build gains no registry dependency.
[The outline](../cli/src/tree.rs) is a presentation of that value. It names
each node's variant, written text and `line:column` range, with columns counting
Unicode scalar values; the JSON keeps exact byte spans and declaration field
order. Color adds escapes only, and JSON is never colored. The check result's
diagnostics use the shared `cartan_diagnostics::to_json` envelope through the
CLI formatter. An unreadable file is an ordinary CLI error on stderr, also in
JSON mode.

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
omitted. The Python outline applies the same rules, and both sides test identical
expected lines.
Bare groups and help exit 0 on stdout, usage errors exit 2 on stderr, and domain
failures retain exit 1 and typed causes until the formatter. File operands use
OS-native paths and `--` terminates option parsing. Color accepts auto/always/never;
explicit choices take precedence over `NO_COLOR`, and auto uses the destination's
terminal state. AST output and metadata JSON never contain presentation escapes.

`--describe-cli` exports the built tree using the
[version-one shared schema](../../docs/records/component-apps.md#rust-authority-and-python-exposure).
Visible command/argument aliases, inherited globals, value arity, defaults and
choices are derived through Clap introspection. Custom validation/relationships
remain Rust responsibilities. The local exporter and Python adapter introduce no
component dependency; peers agreed the schema and maintain their own adapters.

The CLI alone depends on Clap 4.6.6 with default features disabled and
`std`, `help`, `usage`, `error-context`, `color` enabled, plus `serde_json`.
The parser library dependency graph is unchanged. On the existing local build
cache, the initial offline CLI dependency check completed in about 1.8 seconds
and build in about 2.0 seconds; these are observed incremental build costs, not
clean-build benchmarks or cross-machine estimates.

[CLI process tests](../cli/tests/cli.rs) exercise sole-command enforcement,
concise versus full help, the absent `help` subcommand, version, usage failures,
OS path operands, termination, color (including options after a help flag) and
JSON discovery. [Python tests](../tests/python/test_application.py) cover
nested/standalone forwarding, terminal color precedence and process-free command
help. [Metadata tests](../tests/python/test_metadata.py) show fixture commands
added or removed in metadata appearing or disappearing without Python edits,
typed discovery failures, single discovery, and the actual binary's commands and
outline matching Rust's own `--help`. The installed
entrypoint preserves statuses 0/1/2 outside the repository and matches Rust output
for all five workload fixtures. The existing Starlette warning remains; actual
Tree-sitter parser generation/execution is outside this CLI change's verification.

`grammar::grammar()` returns the shared grammar value. The same value drives:

- `grammar::print()`: Tree-sitter grammar JavaScript.
- `print_textmate()`: lexical highlighting grammar.
- `print_vim()` and `print_vim_ftdetect()`: Vim syntax and file detection.
- `print_sublime()`: Sublime syntax definitions.
- `print_emacs(builtin_names)`: Emacs source using caller-supplied builtin names.

The generic `treegram` package interprets and prints this grammar. Tooling does
not import a language builtin registry or require compiler execution. These
generators do not provide a language server or semantic completion features.

## Development application

The installable `cartan-syntax-dev` distribution supplies `syntax_dev` and the
unique `cartdev-syntax` executable. It composes with the workspace development
app through the version-one `cartan.dev.components` entry point named `syntax`.
The component owner maintains the implementation under the
[Python app task](work/python-app.md) and the workspace's
[application contract](../../docs/records/component-apps.md).

`syntax_dev.application.component()` returns `api_version`, `id`, `title`,
`create_cli` and `create_app`. The factories accept an explicit absolute
`repo_root` and optional `rust_command` tuple. `create_app` additionally accepts
the tuple of `(label, href)` navigation entries supplied by its composition owner.
Returned Click groups and Starlette apps are independent of the aggregate
package; imports and factories start no processes or servers.

The standalone command locates the project from the invocation directory or
`--repo PATH`. Its repeatable `--rust-command TOKEN` option supplies literal
command-prefix arguments; use `--rust-command=-c` for an argument beginning with
a hyphen. Factories and standalone commands read a component-local
`[tool.cartdev]` `rust-command` array in `pyproject.toml` when no explicit command
is supplied. The default is `target/debug/cartan-syntax` under the configured
root. Missing binaries yield a build instruction, never an implicit build.

The CLI factory declares only Python-owned commands such as `serve`; its Rust
commands come from the configured binary's [metadata](../src/syntax_dev/metadata.py).
Every metadata node becomes a display-only Click object: a group for a node with
children, otherwise a command, so the generated hierarchy is traversable. These
objects never parse; the root group forwards the original argv. Discovery runs at
most once per group, only for group help or listing, and validates every
version-one key with its exact type, including an integer `schema_version`. Launch, status, JSON, schema and collision
failures are typed `MetadataError` kinds, worded by one presentation function;
group help then still lists Python commands. A Rust command may not share a
Python command's name. `-h` on the group prints concise help; `--help` adds the
command outline derived from metadata followed by the visible Python commands;
hidden Python commands still dispatch. Bare invocation and group help discover
metadata; a launch, status or schema failure there exits 0 with the Python commands
and the build or compatibility instruction, never building. Python command help
starts no Rust process. Leading options always belong to Rust, so a Python command
name must come first. Every other argument vector, including
generated command names, unknown names, options and `--describe-cli`, is
forwarded literally to the configured binary, which owns validation and the help
of its own commands. Invoking-directory paths
retain their meaning because CLI forwarding does not change the child cwd.
Real stdin/stdout/stderr are inherited without buffering or decoration, preserving
terminal detection and child exit codes (signals map to conventional 128+signal).
Ctrl-C reaches the child through the shared process group; the forwarder keeps
waiting and returns the child's own status rather than exiting early.
In-process callers whose streams lack descriptors use captured byte streams.
The standalone configuration prefix stops at the first Rust/Python command, so
later `--repo` or `--rust-command` tokens remain literal Rust arguments.

Python declares no syntax command of its own: any argument list not addressed to
`serve` reaches Rust unchanged, so `cartdev syntax SOURCE --ast --json` needs no
wrapper. A source file literally named `serve` is written `./serve`. Web
diagnostics invoke `--ast -- SOURCE`, so they show the document's `main`, with
repository-relative paths and a default 30-second bound. Their version-one result envelope preserves opaque
child streams and status; it is not command metadata or structured Rust IR.
The asynchronous web service kills its POSIX process group and reaps the child
on timeout or cancellation; cancellation propagates after cleanup.

The web app serves `/`, `/docs/`, `/diagnostics/` and locally packaged CSS and
JavaScript. It follows the [shared visual conventions](../../docs/records/component-apps.md)
with global and component navigation, a keyboard skip link, visible focus,
responsive panels and bounded code/table overflow. Styles and scripts are
packaged independently; no aggregate or sibling import is required. Documentation
is restricted to resolved Markdown paths inside `docs/` and the root README;
symlinks escaping those sources are refused. Metadata headers and raw HTML are
not rendered as page content. Relative Markdown links are routed inside the
viewer; relative non-document links and images are disabled rather than exposing
other repository files. Local URLs honor deployment and named-mount prefixes;
supplied aggregate navigation URLs are not rewritten.

GET requests start no Rust process. An explicit diagnostic form or
`POST /api/inspect` with JSON `{"source": "path.cart"}` invokes the same bounded
service and returns its unchanged result envelope. Browser form submissions
return an HTML result with escaped output and diagnostics, preserving the entered
path. A small local script disables the submit button and announces pending work;
inspection also works without JavaScript. Invalid form input renders a styled
error with HTTP status 400, matching JSON validation status. Web source paths are relative to the
configured repository. Each app owns active operation tasks through its lifespan;
the aggregate must enter and exit each child's lifespan once. `serve` uses
Uvicorn on loopback by default with an explicit port option.

In this parent uv workspace, Python packages share its `uv.lock` and `.venv`.
After the workspace owner runs `uv sync --locked --all-packages` at the root,
use `uv run --no-sync --package cartan-syntax-dev` from this component directory
so test and fixture paths remain local. `--locked` alone can still mutate the
shared environment; `--no-sync` uses the prepared environment without syncing.
The component retains its own `pyproject.toml` and `uv.lock` for independent
clones outside the parent workspace, where `uv sync --locked` creates a local
environment. It has no dependency on the aggregate Python package. The parent
owns workspace membership and shared environment changes; coordinate mutations
there with its owner. See the [README](../README.md) for commands in both layouts.

An offline install requires cached artifacts. Set `UV_CACHE_DIR` to a writable
cache when the default location is unavailable. No GPU, SSH, persistent server
or browser execution is needed by the
[in-process tests](../tests/python/test_application.py).

## Acceptance limits

Grammar parsing is one layer of syntax processing. The separate
[`ast` interface](ast-interface.md) normalizes a supported subset into an
abstract syntax tree (AST), the representation of source constructs used for
further analysis. It includes file/namespace use heads, selectors, aliases and
qualified value/call names without loading files or resolving names. It refuses
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

Graph is the only direct consumer. It owns its adapter and locked syntax Git
dependency, following the [local dependency contract](ast-interface.md#local-dependency-delivery).
Name resolution, overload selection, type checking and graph construction remain
outside syntax. Codegen consumes graph's typed representation without a
dependency on this crate. Integration and execution evidence belongs to the
consuming components.
