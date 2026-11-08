# cartan-syntax

Cartan's grammar, parser and source syntax tree. One grammar produces concrete
syntax trees (CSTs), a normalized source abstract syntax tree (AST) for the
numerical workload subset, and editor grammar definitions. Syntax performs no
name resolution, typing, graph construction or evaluation.

## Entry points

- Library: `cartan_syntax::parse_document` returns a CST with structured grammar
  diagnostics; `cartan_syntax::ast::parse_document` returns the source AST or
  typed errors with byte spans; `cartan_syntax::selection` picks one top-level
  binding or update block; `cartan_syntax::prompt` classifies prompt text as
  complete, incomplete or invalid and colors it; `cartan_syntax::KEYWORDS` lists
  the reserved words. Contracts: [component](docs/component.md) and
  [AST interface](docs/ast-interface.md).
- Diagnostics: `support/diagnostics` (`cartan-diagnostics`) renders reports;
  `support/syntax-diagnostics` (`cartan-syntax-diagnostics`) is the one home of
  Syntax wording. The parser library depends on neither.
- CLI: `cartan-syntax SOURCE [BINDING|UPDATE] [--ast] [--json]`.

  ```sh
  cargo run --offline -- tests/fixtures/diffusion.cart
  cargo run --offline -- tests/fixtures/diffusion.cart update --ast
  cargo run --offline -- tests/fixtures/diffusion.cart ni --json
  ```

  `SOURCE` checks the whole document. A selector (a top-level binding, or
  `update:N`; `update` means `update:0`) must name exactly one item; `--ast`
  prints the selected item's syntax tree, `main` by default; `--json` prints the
  same result as JSON. Usage errors exit 2 and failures exit 1. Details:
  [syntax tooling](docs/component.md#syntax-tooling).
- Workspace: `cartdev syntax` (run from the workspace root) is generic root code
  driven by this CLI's `--describe-cli` and `--json` output.

## Checks

```sh
cargo fmt --check
cargo check --workspace --offline
cargo test --workspace --offline
cargo clippy --workspace --all-targets --offline -- -D warnings
RUSTDOCFLAGS=-Dwarnings cargo doc --workspace --no-deps --offline
python3 tools/docs.py check
python3 -m unittest discover -s tools -p 'test_*.py'
cargo run -p cartan-syntax --offline --example grammar > out/grammar.js && node --check out/grammar.js
```

`out/` is ignored scratch space.

## Current limitations

- Only `while` blocks normalize as update blocks; `every` blocks, namespace
  bodies, default arguments, qualified annotations, function-header
  configuration and Tap bodies parse to a CST but are unsupported in the AST.
- Generated Tree-sitter JavaScript passes a syntax check; building and running a
  Tree-sitter parser is untested here.

## Consumers and next work

Graph consumes the source AST and selection; Graph, Codegen and Compile use the
diagnostic packages, and Runtime's interactive prompt uses `prompt`. Their
`Cargo.lock` files record the revision each one consumes; all of them pin
`main`. No component work is scheduled.
