---
id: configurable-source-ast
title: Normalize maps and configurable source forms
kind: work
status: done
implementation: implemented
topics: [syntax, interface, frontend]
owner: cartan-syntax-agent
depends: [normalized-ast, cartan-lang:compile-language-integration]
authority: Owner-authorized configurable stream; graph and coordinator accepted exact AST and coordinator authorized implementation on 2026-10-25
evidence: src/ast.rs; src/ast/normalize.rs; src/grammar.rs; tests/ast.rs; verification below
---

# Normalize maps and configurable source forms

Supply ordinary named maps, dedicated map join, ordered configurable settings
atoms with a body, and colon/literal-dot application. The authoritative
[AST contract](../ast-interface.md#maps-and-configurables) owns the forms,
precedence, public fields, spans and limits, including repeated constructor atoms.
Consumers own semantic adapters and deliberate locked dependency updates after
the tested supplier commit.

Preserve duplicate keys, ordered operands/atoms/entries, full child source spans
and exact punctuation spans. Reuse existing Body form checks and configuration
atoms; no parser workaround or semantic inference. Computed map keys/walks,
retired function-header configuration, parameter defaults and named/trailing-dot
closing remain excluded. No helper, legacy access, sibling edit or shared Python
mutation is authorized by this task.

Acceptance: structural and span tests for all added forms, precedence and chained
applications, body-order checks, unchanged structured refusals, all existing
workload fixtures, documented Rust/Python/docs checks, generated grammar JavaScript
syntax check, authoritative contract update and tested local supplier commit.
Actual Tree-sitter parser generation/execution remains outside verified scope.

## Verification and delivery

Verified retained source `029cc28884bf310cd72a745923d5f1dc3d24878a` on
2026-10-26 with unchanged source and dependency locks. Recovery changes only
documentation; existing operational guidance remains applicable.

- All 42 Rust workspace tests pass, including 18 AST tests. Added coverage checks
  named/empty/nested maps and duplicate key provenance, UTF-8 offsets through
  comments, left-associative joins and arithmetic precedence, exact continuation
  punctuation spans, ordered constructor atoms, function/document composition,
  local/result order, colon/dot chains, call/index atoms, structured body errors,
  unsupported maps and malformed forms. All five workloads remain accepted.
- Locked offline workspace check, strict Clippy/rustdoc, rustfmt and diff whitespace
  checks pass. Generated Tree-sitter JavaScript passes `node --check`; actual
  Tree-sitter parser generation/execution is untested.
- All 22 Python app tests, 7 documentation-tool tests and 11-document validation
  pass. The existing Starlette HTTPX deprecation warning remains. Shared Python
  checks used the prepared root environment's Python executable; no environment
  or dependency mutations were performed. The `uv run --no-sync` wrapper could
  not access its default cache in the sandbox.
- The rebuilt prebuilt Rust CLI and installed Python CLI both accept all five
  workload fixtures and print successful no-argument help. No syntax execution
  commands or runtime function mechanisms are exposed.
- Consumers choose the tested commit and own semantic support, rejection and
  dependency updates. No execution, type-fitting, setting-scope or device claim
  is made by this supplier delivery.

The [workspace language scope](../../docs/records/language-scope.md) retains this
syntax while deferring configurable semantics and static function arguments.
Runtime function values, returned functions and dynamic callees are excluded;
the syntax delivery grants no execution capability or implementation authority.
