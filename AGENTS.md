# Working in cartan-syntax

Read README.md and docs/component.md first. Shared rules, workflow commands,
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
- Documentation follows [docs/documentation.md](docs/documentation.md): current
  state only, one home per fact, done work items deleted after consolidation.
- HANDOFF.md is ignored session state, never committed.

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
  The library depends on neither diagnostic package.
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
python3 tools/docs.py check --strict
python3 -m unittest discover -s tools -p 'test_*.py'
git diff --check
cargo run -p cartan-syntax --offline --example grammar > out/grammar.js && node --check out/grammar.js
```

- `tools/docs.py` and `docs/documentation.md` are byte-identical copies of the
  workspace validator and conventions; never edit them here. The workspace's
  `cartdev docs drift` checks them.
- Offline builds need the cached Clap, serde and JSON crates.
- Building and running an actual Tree-sitter parser is untested here; only the
  generated JavaScript is syntax-checked.
- Test syntax-tree structure and spans, malformed input and representative
  programs; parser acceptance is not semantic validation. Wording tests belong
  to `support/syntax-diagnostics`.
