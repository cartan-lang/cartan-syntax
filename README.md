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
cargo run --offline --example grammar > out/grammar.js
node --check out/grammar.js
```

The root package uses the local `support/treegram` package. Neither package
requires registry downloads or an adjacent repository. Both have publication
disabled; no GitHub remote or public/private visibility has been assigned.

## Current scope and next work

- Shared grammar parsing and editor-source generation are implemented.
- Seventeen Rust tests cover grammar invariants, five numerical workload
  fixtures, tree structure, source spans, Unicode, errors and tooling.
- Normalized abstract syntax tree (AST) construction and additional syntax-form
  checks are not implemented. Grammar acceptance alone is not full language
  validation or evidence that codegen supports a construct.
- The generated Tree-sitter JavaScript passes a syntax check. Building and
  executing the actual Tree-sitter parser remains untested here.
- The next integration is a normalized syntax/AST interface and an adapter in
  the codegen frontend. Codegen's typed library must remain parser-independent.

The [source provenance](docs/provenance.md) records the extraction inputs.
The [documentation conventions](docs/documentation.md) define the queryable
Markdown format used in this repository.
