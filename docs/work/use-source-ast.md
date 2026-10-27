---
id: use-source-ast
title: Normalize imports and qualified source names
kind: work
status: done
implementation: implemented
topics: [syntax, interface, frontend]
owner: cartan-syntax-agent
depends: [normalized-ast]
authority: Owner requested graph-led use statements; supervisor authorized the agreed syntax prerequisite on 2026-10-24
evidence: src/ast.rs; src/ast/normalize.rs; tests/ast.rs; verification below
---

# Normalize imports and qualified source names

Extend the existing grammar's AST normalization for file/namespace use heads,
optional alias or selectors, and qualified value/call names. The authoritative
[AST contract](../ast-interface.md) owns final data structures and adopted form
rules. Agree public API with graph/codegen before edits; deliver a tested supplier
commit for deliberate consumer adapter and lockfile updates.

Preserve source spelling and UTF-8 spans. Decode only quoted file paths, without
filesystem access. Keep absent selectors distinct from wildcard; normalize single
and braced name selectors alike. Enforce typed alias-plus-selector refusal; keep
with/hole clauses, namespace declarations/functors, qualified annotations, general
strings and new builtins unsupported. No grammar change or semantic resolution.

The [adopted source contract](../ast-interface.md#adopted-source-contracts)
records the adopted form and decoding rules and their rationale. The task adds
no execution responsibility.

Acceptance: agreement with consumers, five existing workload fixtures, import
selector/alias/path shapes and UTF-8 offsets, exact path decoding, qualified
segments and calls, typed form/unsupported/grammar diagnostics, documented Rust
and Python checks, updated contract and local supplier commit.

## Verification and delivery

- Graph and codegen owners explicitly accepted the exact public fields/variants
  before edits. No consumer checkout or lockfile was changed here; both consumers
  select the tested supplier commit and adapt independently.
- All 37 Rust workspace tests pass, including 13 AST tests. Six new tests cover
  both import-head kinds and absent/single/list/star selectors, aliases, UTF-8
  offsets, raw/decoded path escapes, qualified value/call segments, typed
  alias-plus-selector refusal, retained unsupported forms and unchanged grammar
  diagnostics. The five numerical fixtures continue to normalize with full spans.
- Offline workspace check, strict Clippy/rustdoc, default rustfmt and diff
  whitespace checks pass. All 22 Python application tests pass with no sync,
  alongside ten-document validation and seven documentation-tool tests. The
  existing Starlette HTTPX deprecation warning remains.
- Grammar, dependencies and execution behavior are unchanged. No filesystem
  lookup, source semantics, builtin catalogue, namespace declarations/functors,
  general string expressions, device work or shared-environment mutation was added.
  Actual Tree-sitter parser execution remains outside the existing verified scope.

Documentation review keeps the data/form/decoding contract in `ast-interface.md`,
component boundary/limits in `component.md`, concise README scope and this bounded
verification record. No separate proposal or historical archive was introduced.
