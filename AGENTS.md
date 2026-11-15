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
  highlighting is lexical, not semantic analysis or a language server.
- Do not depend on, read or run the legacy checkout; semantic questions go to
  the lead supervisor.

## Layout

- The root package `cartan-syntax` is the reusable library. It has no registry
  dependency by default; its optional `serde` feature is for the CLI only.
- `support/treegram` is the grammar engine; `support/diagnostics`
  (`cartan-diagnostics`) renders reports; `support/syntax-diagnostics`
  (`cartan-syntax-diagnostics`) is the one home of Syntax diagnostic wording.
  The library depends on neither diagnostic package. `support/cli`
  (`cartan-cli-support`) holds the help, outline, `--describe-cli`, `--color`
  and `--json` envelope conventions every component CLI shares; extend it
  rather than copying helpers into a CLI.
- `cli/` (`cartan-syntax-cli`, binary `cartan-syntax`) is the default workspace
  member: `cartan-syntax SOURCE [BINDING|UPDATE] [--ast] [--json]`. With no
  arguments it prints help and exits 0; usage errors exit 2 and failures 1;
  under `--json` every outcome is a JSON result on stdout. Library examples need
  `-p cartan-syntax`.
- There is no Python package. The workspace `cartdev syntax` adapter is generic
  root code driven only by `--describe-cli` (including each command's `notes`,
  its after-help) and `--json`; keep both accurate.

## Dependencies

Consumers (Graph, Codegen, Compile, Runtime) depend on this repository through
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
- Offline builds need the cached Clap, serde and JSON crates.
- Building and running an actual Tree-sitter parser is untested here; only the
  generated JavaScript is syntax-checked.
- Test syntax-tree structure and spans, malformed input and representative
  programs; parser acceptance is not semantic validation. Wording tests belong
  to `support/syntax-diagnostics`.
