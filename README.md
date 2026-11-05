# cartan-syntax

Standalone Cartan grammar parsing and syntax-based editor tooling. The shared
grammar produces both concrete syntax trees and editor grammar definitions.
A concrete syntax tree (CST) preserves grammar nodes, punctuation, comments,
field labels and source spans. It does not resolve names or construct a
semantic computation graph.

```rust
let tree = cartan_syntax::parse_document("param n = 32\n");
assert!(!tree.has_errors());
```

`parse_expression` parses an individual expression. Both functions return a
`SyntaxTree` containing owned source text, the root node and structured grammar
diagnostics. Errors retain rule identity, byte spans and incomplete-input facts.
See the [component contract](docs/component.md) for API details and limitations.

`ast::parse_document` and `ast::parse_expression` normalize the numerical
workload subset into a source AST with full byte spans and structured errors.
The [AST contract](docs/ast-interface.md) states its supported constructs,
form checks and boundary with semantic analysis.

Inspect a `.cart` file's normalized AST, including source spans:

```sh
cargo run --offline -- ir tests/fixtures/diffusion.cart | less
```

This prints Rust's indented debug representation of source items, not typed
computational IR or generated target code. Errors go to stderr with a nonzero
exit status. The inspection format is not a stable interchange format.
With no arguments, `cargo run` prints concise help to stdout and exits
successfully. `ir` is the sole inspection command. At any command, `-h` prints
concise help and `--help` adds the full command outline generated from the Clap
tree; there is no `help` subcommand. `--version` prints the version and
`--describe-cli` prints version-one JSON discovery derived from the same tree. Usage errors exit 2;
execution failures exit 1. `--color auto|always|never` controls help/diagnostics;
`NO_COLOR` disables automatic color unless an explicit option overrides it.
Inspection and JSON output remain undecorated.

The Python development app provides the composable `cartdev-syntax` commands
and a documentation/diagnostics web app. In an independent clone, run these
from this repository; uv uses its own `uv.lock` and `.venv`:

```sh
uv sync --locked
uv run cartdev-syntax
uv run cartdev-syntax ir tests/fixtures/diffusion.cart
uv run cartdev-syntax ir -h
uv run cartdev-syntax --help
uv run cartdev-syntax --describe-cli
uv run cartdev-syntax serve --port 8000
```

In this parent Python workspace, uv uses the shared root `uv.lock` and `.venv`.
After the workspace owner runs `uv sync --locked --all-packages` at
the workspace root, run these from this component directory:

```sh
uv run --no-sync --package cartan-syntax-dev cartdev-syntax
uv run --no-sync --package cartan-syntax-dev cartdev-syntax ir tests/fixtures/diffusion.cart
uv run --no-sync --package cartan-syntax-dev pytest -q tests/python
```

The component lockfile remains for independent clones; it does not select
dependencies inside the parent workspace. Coordinate shared environment changes
with the workspace owner. Use `--no-sync` for checks and commands against the
prepared environment; `--locked` alone can still change installed packages.

Python derives its Rust commands, descriptions and `--help` outline from the
configured binary's `--describe-cli` metadata, with no handwritten command
registrations. Their arguments still reach Rust unchanged, preserving the
invocation directory, streams and exit status. `serve` and its help need no Rust
process; group help reports a missing or incompatible binary without building. The former Python-only
`ir --json` and `--timeout` options are removed: Rust validates its own options.
The web diagnostics API retains its bounded version-one result envelope.

IR inspection invokes the existing debug binary; build it explicitly with
`cargo build --offline` when needed. Help and documentation browsing do not need
Rust. See the [development app contract](docs/component.md#development-application)
for configuration, installed factories and mounted use.

## Checks and tooling

```sh
cargo fmt --check
cargo check --workspace --offline
cargo test --workspace --offline
cargo clippy --workspace --all-targets --offline -- -D warnings
RUSTDOCFLAGS=-Dwarnings cargo doc --workspace --no-deps --offline
python3 tools/docs.py check
python3 tools/docs.py inventory
python3 -m unittest discover -s tools -p 'test_*.py'

mkdir -p out
cargo run -p cartan-syntax --offline --example grammar > out/grammar.js
node --check out/grammar.js
```

The root library uses the local `support/treegram` package. The `cli` workspace
member supplies the `cartan-syntax` binary and is the default member, so ordinary
`cargo run` selects it. Use `-p cartan-syntax` for library examples or library-only
checks; use `--workspace` to check all packages. All packages have publication
disabled and require no adjacent repository. Offline builds require cached
Clap and JSON dependencies for the CLI; the parser library gains no registry
dependency.
For Python tests in an independent clone, run
`uv run --locked pytest -q tests/python`; in the parent workspace use the
package-selected command above. Cargo packages and build directories remain
independent of the Python workspace.

## Current scope and next work

- Shared grammar parsing, workload AST normalization and editor-source
  generation are implemented.
- Tests cover grammar invariants, five numerical workload
  fixtures, tree structure, source spans, Unicode, errors and tooling.
- The AST covers the five workload fixtures, ordered body items, list/site
  patterns, file/namespace imports, qualified values/calls, static and computed
  Map literal entries and joins,
  configurable constructors and colon/literal-dot application, strings, tagged
  templates with structured format specs, struct declarations, sum rosters,
  constructors and `match`,
  function-signature parameter annotations, tuple annotations, value-producing
  `each` walks, and source-form checks. Import paths retain raw text, decoded strings and byte spans; syntax
  performs no file loading, name resolution or formatting. Other constructs may parse into
  a CST but return a structured unsupported-form error during normalization.
  Neither API performs semantic validation or verifies execution support.
  Graph implements bounded static named-function arguments and owns configurable
  semantics and execution limits under the
  [workspace language scope](../docs/records/language-scope.md).
- The generated Tree-sitter JavaScript passes a syntax check. Building and
  executing the actual Tree-sitter parser remains untested here.
- Graph is the only direct AST consumer and owns its adapter and locked local
  Git dependency. Codegen consumes graph's typed representation without a
  syntax dependency; integration and device execution are verified downstream.

The [documentation conventions](docs/documentation.md) define the queryable
Markdown format used in this repository.
