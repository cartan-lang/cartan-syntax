# Working in cartan-syntax

Read README and docs/component.md first. Shared rules, workflow commands,
ownership and the HANDOFF shape are in the workspace
[AGENTS.md](../AGENTS.md); joint Codex/Claude operation, ownership handback and
mailboxes are in [joint operation](../docs/records/joint-operation.md). In a
standalone clone, the essentials are:

- One agreed writer per checkout, recorded in the ignored
  `.agent-mail/ownership.json`; others review read-only until it is handed back.
- Commits: sentence-case imperative subjects of about 60 characters, no trailing
  period or prefix, the configured Git identity, no agent attribution or
  co-author trailers; stage explicit paths; no push or publication without owner
  authorization.
- Diagnostics stay typed until one central formatter per application; wording
  lives only there and is never parsed.
- Tests are sparse and discriminating; no tests that only check retired
  behavior is absent.
- Documentation follows the workspace's
  [conventions](../docs/documentation.md), of which a standalone clone has no
  copy: current state only, one home per fact, done work items deleted after
  consolidation.
- HANDOFF.md is ignored session state, never committed.

## Design from requirements

For substantial features, repairs and design work, state the problem, required
behavior and owner constraints before inspecting implementation. Ask: "If we
were building this from scratch, what is the simplest coherent solution?"
Write that candidate first; set familiar architecture aside. Then examine the
existing architecture and compare solutions. Existing interfaces are choices,
not implicit requirements; preserve mechanisms and verified behavior that fit.
Consider diff size, migration cost, reviewability and regression risk alongside
the simplicity of the resulting system. Neither a small patch nor a rewrite is
inherently preferable, and sunk effort alone does not justify retaining a design.
Always escalate to the supervisor or owner when an architectural change makes an
obviously simpler solution possible, before implementing an accommodation.
State the requirements, architectural change, benefit, retirement plan and smallest
resolving experiment. Keep the checkpoint bounded and preserve a working baseline.
Reuse what fits; update callers and retire superseded mechanisms coherently.
Preserve user work without inventing compatibility obligations or leaving parallel
implementations without an explicit scope and retirement condition.

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
- Do not depend on, read or run the legacy checkout; semantic questions go to
  the lead supervisor.

## Layout

- The root package `cartan-syntax` is the library. It has no registry
  dependency by default; its optional `serde` feature serializes the source AST
  for a consumer that stores or prints it.
- `support/treegram` is the grammar engine the library reads and prints the
  grammar with.
- There is no command-line tool, no diagnostic wording and no Python package.
  The library returns typed diagnostics; the application words them and reaches
  syntax through its own executable. `examples/grammar.rs` prints the
  Tree-sitter grammar JavaScript.

## Dependencies

The compiler and application depend on this repository through
Cargo Git dependencies on `main` with the absolute local `file://` URL, and take
all Syntax packages in one lock update. Each consumer's `Cargo.lock` records the
revision it uses. Agree interface changes with the consuming owner and announce
the tested commit; never edit a consumer's checkout or Cargo cache. Create a
separate supplier branch only by agreement, and retire it when consumers move.

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
cargo run -p cartan-syntax --offline --example grammar > out/grammar.js && node --check out/grammar.js
```

- Documentation is checked from the workspace, and a standalone clone has no
  checker.
- Offline builds of the `serde` feature need the cached serde crates.
- `cargo test` generates the Tree-sitter parser from `grammar::print_json()`
  with the `tree-sitter-generate` crate, compiles it with the system C
  compiler through `cc`, and requires its trees to equal the interpreter's on
  one-line fixtures (`tests/editors.rs`). These are dev-dependencies only. A
  cold run generates for about a minute; the result is cached under
  `target/tmp/` by the grammar's hash. The compile step is set up for macOS
  and Linux. The parser runs in no editor here; editor behavior is untested.
- Test syntax-tree structure and spans and malformed input; parser acceptance
  is not semantic validation. A test's Cartan text is as close to a one-liner
  as it can be and tests one capability; programs live only in the
  application's examples, and no document is copied here from elsewhere. Tests assert typed
  diagnostics, never wording.
