# Working in cartan-syntax

Read README.md and docs/component.md first. This repository is independently
buildable; the parent workspace's AGENTS.md coordinates component work.

## Scope

- Own Cartan syntax: parsing, syntax trees, source spans, structured parse
  diagnostics and syntax-only tooling. Keep supported syntax explicit.
- A syntax tree describes written constructs. Typed computational intermediate
  representations, name resolution, type checking, graph construction, runtime
  values and execution belong to other components.
- Preserve established Cartan idioms. Do not replace function equations or `of`
  projections with unrelated surface syntax for implementation convenience.
- Reject malformed/unsupported syntax explicitly; do not silently discard text.
  Parsing a construct does not promise another component can execute it.
- Keep syntax tooling independent of builtin registries or semantic engines.
  Highlighting is not semantic analysis or a language server.
- Do not depend on a legacy checkout, its evaluator, its diagnostic framework
  or its conformance suite. Only the top-level supervisor may consult legacy;
  request findings rather than reading, searching or executing that checkout.
- Preserve user changes, coordinate concurrent file ownership and commit explicit
  paths. No remote creation, visibility choice, push or publication without owner
  instruction. Use this repository's own target directory.
- Work in this component's existing checkout. Creating or using a Git worktree
  requires explicit supervisor permission for that task.

## Rust and verification

- Run default `cargo fmt` after edits and `cargo fmt --check` before committing.
  Use conventional Rust names and module organization.
- One blank line separates functions and type declarations. Group related
  statements; avoid whitespace between every statement or multiple blank lines.
- Represent parse failures as typed facts with source spans. Render diagnostics
  at presentation boundaries. Tests assert structured identity and payload.
- Test syntax-tree structure and spans, malformed input and representative
  supported programs. Parser acceptance alone is not semantic validation.
- Run `cargo check --workspace --offline`, `cargo test --workspace --offline`,
  `cargo clippy --workspace --all-targets --offline -- -D warnings`, and
  `git diff --check`. Check public rustdoc links with
  `RUSTDOCFLAGS=-Dwarnings cargo doc --workspace --no-deps --offline`.
- Run `python3 tools/docs.py check` and
  `python3 -m unittest discover -s tools -p 'test_*.py'` for documentation tooling.

## Documentation and terminology

- Follow docs/documentation.md: Markdown, small structured headers, current
  state only. Git records history; no completed-work diary or manual status index.
- Maintain decisions, contracts and bounded tasks in structured docs/rulings,
  docs/records and docs/work files. Resume through the generated inventory and
  relevant tasks; do not create handoff or session-summary files. Coordinate
  schema changes with the supervisor and synchronize conventions and validators.
- Keep one authoritative home per fact. Separate proposal, approval and actual
  implementation; list limitations and untested behavior explicitly.
- Use mainstream technical terminology and define terms on introduction.
  Refresh meanings in chat when useful. Agree on unfamiliar or project-specific
  vocabulary with the owner before putting it into authoritative documentation.
