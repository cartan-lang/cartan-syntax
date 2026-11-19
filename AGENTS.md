# Working in cartan-syntax

Read README and docs/component.md first. The workspace root's
[AGENTS.md](../AGENTS.md) holds the shared rules: workflow, ownership, HANDOFF,
design doctrine and escalation, commits and documentation. This file adds only
what is specific to this repository.

## Scope

- Own Cartan syntax: the grammar, parsing, concrete syntax trees, the
  normalized source AST, source spans, structured parse diagnostics, source
  selection, command-line literals, prompt facts and syntax-only tooling
  (editor grammars). Keep supported syntax explicit.
- A syntax tree describes written constructs. Name resolution, type checking,
  typed computational IR, graph construction, runtime values and execution
  belong to other components; parsing a construct promises nothing downstream.
- Preserve established Cartan idioms; do not replace function equations or `of`
  projections with other surface syntax for implementation convenience.
- Reject malformed or unsupported syntax explicitly; never silently drop text.
- Keep tooling independent of builtin registries and semantic engines;
  highlighting uses lexical and written syntax contexts, not semantic analysis or a language server.

## Layout

- The root package `cartan-syntax` is the library. It has no registry
  dependency by default; its optional `serde` feature serializes the source AST
  for a consumer that stores or prints it.
- `support/treegram` is the grammar engine the library reads and prints the
  grammar with.
- There is no command-line tool, no diagnostic wording and no Python package.
  The library returns typed diagnostics; the application words them and reaches
  syntax through its own executable. `src/editors.rs` prints the editor
  packages committed under `editors/`, and the editor tests generate the
  Tree-sitter parser beside them.

## Dependencies

The compiler and application depend on this repository through
Cargo Git dependencies on `main` at `https://github.com/cartan-lang/cartan-syntax`, and take
all Syntax packages in one lock update. Each consumer's `Cargo.lock` records the
revision it uses. Create a separate supplier branch only by agreement, and
retire it when consumers move.

## Checks

Run from this directory:

```sh
cargo fmt --check
cargo check --workspace --offline
cargo check --features serde --offline
cargo test --workspace --offline
cargo clippy --workspace --all-targets --offline -- -D warnings
RUSTDOCFLAGS=-Dwarnings cargo doc --workspace --no-deps --offline
# From the workspace root:
uv run --no-sync cartdev docs check --root cartan-syntax --strict
git diff --check
node --check editors/tree-sitter-cartan/grammar.js
```

- Offline builds of the `serde` feature need the cached serde crates.
- `cargo test` generates the Tree-sitter parser from `grammar::print_json()`
  with the `tree-sitter-generate` crate, compiles it and the generated
  `grammar::print_scanner()` with the system C compiler through `cc`, and
  requires its trees to equal the interpreter's on one-line fixtures and every
  emitted query to load against it (`tests/editors.rs`). These are
  dev-dependencies only. The result is cached under `target/tmp/` by the
  grammar's hash. The compile step is set up for macOS and Linux.
- The same tests fail when a file under `editors/` differs from the printed
  one; `CARTAN_SYNTAX_BLESS=1 cargo test --test editors` prints them again and
  writes the ignored Zed development copy under `out/zed/`. Never edit
  `editors/` by hand. [editors/README.md](editors/README.md) states the Zed
  revision pin and local installation.
- No editor runs here: highlighting and editing behavior in Zed, VS Code, Vim,
  Sublime Text and Emacs is untested unless the owner reports observing it.
- Test syntax-tree structure and spans and malformed input; parser acceptance
  is not semantic validation. Tests assert typed diagnostics, never wording.
