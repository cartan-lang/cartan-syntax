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
cargo run --offline -- ast tests/fixtures/diffusion.cart | less
```

This prints Rust's indented debug representation of source items, not typed
computational IR or generated target code. Errors go to stderr with a nonzero
exit status. The inspection format is not a stable interchange format.

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
disabled and require no registry downloads or adjacent repository.

## Current scope and next work

- Shared grammar parsing, workload AST normalization and editor-source
  generation are implemented.
- Tests cover grammar invariants, five numerical workload
  fixtures, tree structure, source spans, Unicode, errors and tooling.
- The AST covers the five workload fixtures, ordered body items, list/site
  patterns and comparison/range-chain checks. Other constructs may parse into
  a CST but return a structured unsupported-form error during normalization.
  Neither API performs semantic validation or verifies execution support.
- The generated Tree-sitter JavaScript passes a syntax check. Building and
  executing the actual Tree-sitter parser remains untested here.
- The codegen frontend owns its AST adapter and locked local Git dependency.
  Codegen's typed library remains parser-independent; consumer integration
  and device execution are verified by that component.

The [source provenance](docs/provenance.md) records the extraction inputs.
The [documentation conventions](docs/documentation.md) define the queryable
Markdown format used in this repository.
