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
and `DiagnosticKind`. A kind is an expected-token set, an unexpected token, or
a stated grammar rule with its refusal flag and captured fields. A captured
field has a name and span; it is not a rendered error sentence.

The owner requires diagnostics and errors to remain typed variants with payloads,
source spans and underlying causes until one top-level formatter in a central
location per standalone application. Lower layers must not generate diagnostic
sentences, collapse causes into strings or classify errors by wording. The public
`Rule` enumeration identifies grammar-specific failures; consumers inspect that
identity rather than diagnostic prose. Wording assertions belong only to formatter
tests. Other tests check structured identity, payload, location, cause and status.
External process streams remain opaque output, not a source for string classification.

The Rust CST/AST APIs already retain structured diagnostics. The standalone Rust
CLI carries usage, file-read, parse and stdout failures as `CliError` variants;
I/O variants retain the original `io::Error`, and parse failures retain their path
and complete AST errors/spans. Only [the CLI formatter](../cli/src/diagnostic.rs)
implements diagnostic presentation, called by the top-level CLI exit boundary.
AST debug output remains an inspection format, not a diagnostic protocol.

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

`ir` resolves the input against the caller's invocation directory before running
Rust's `ast` command with `cwd=repo_root`. Text output and child error streams
are preserved. `--json` emits the version-one process-result envelope, retaining
opaque Rust debug text and the actual child exit code when available. A missing
executable has a null exit code; failed/timeout operations exit nonzero. The
default timeout is 30 seconds. The asynchronous service kills its POSIX process
group and reaps the child on timeout or cancellation; cancellation propagates
after cleanup.

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
validity. There is also no promise that the codegen frontend can execute every
construct recognized by either API. It supports a smaller numerical subset.

The five fixture programs parse and retain their expected loops and fields.
Tests also pin precedence, typed versus untyped patterns, scalar versus list
binders, newline rules, byte spans, Unicode handling and diagnostic identity.
The fixture copies are tests, not a second authoritative workload collection.

Inside a body, `i, j := [10, 2]` and `[i, j] := [10, 2]` both parse as one
`local` with two identifiers in its `pattern` field. Their CST kinds remain
`bare_list_pattern` and `list_pattern`, respectively; the AST normalizes both
to `PatternKind::List` while preserving the original spans. At document scope,
pattern bindings use `=`; both forms with `:=` report `Rule::PatternValue` at
the colon.
The [syntax tests](../tests/syntax.rs) verify these structures and byte spans.

The generated Tree-sitter productions match the extracted source grammar's
output apart from provenance comments. JavaScript syntax is checked with Node.
Actual Tree-sitter parser generation and execution are untested because its CLI
is unavailable in this environment. There is no full editor-conformance claim.

## Consumer integration

The codegen frontend owns its adapter and locked syntax Git dependency, following
the [local dependency contract](ast-interface.md#local-dependency-delivery).
Graph construction, type checking and source-to-computational-IR lowering remain
outside syntax. The typed codegen library consumes explicit computational units
without acquiring a dependency on this syntax crate. Integration and execution
evidence belongs to the consuming component.
