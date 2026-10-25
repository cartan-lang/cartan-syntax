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
precedence, public fields, spans and limits. Graph accepted the exact seam before
edits; the coordinator authorized the bounded implementation and relayed the
lead's confirmation of repeated constructor atoms. Consumers own semantic adapters
and deliberate locked dependency updates after the tested supplier commit.

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

- All 42 Rust workspace tests pass, including 18 AST tests. Added coverage checks
  named/empty/nested maps and duplicate key provenance, UTF-8 offsets through
  comments, left-associative joins and arithmetic precedence, exact continuation
  punctuation spans, ordered constructor atoms, function/document composition,
  local/result order, colon/dot chains, call/index atoms, structured body errors,
  unsupported maps and malformed forms. All five workloads remain accepted.
- Offline workspace check, strict Clippy/rustdoc, rustfmt and diff whitespace
  checks pass. Generated Tree-sitter JavaScript passes `node --check`; actual
  Tree-sitter parser generation/execution is untested.
- All 22 Python app tests, 7 documentation-tool tests and 11-document validation
  pass. The existing Starlette HTTPX deprecation warning remains. Shared Python
  checks used `--no-sync`; no environment or dependency mutations were performed.
- Graph/coordinator agreed the exact public interface before implementation;
  codegen was notified of additive exhaustive-match cases without resuming it.
  Consumers choose the tested commit and own semantic support, rejection and
  dependency updates. No execution, type-fitting, setting-scope or device claim
  is made by this supplier delivery.

Documentation review retains the interface in `ast-interface.md`, concise README
scope and this bounded verification record. No separate proposal or redundant
historical design document was created. The ignored handoff records the supplier
revision and immediate consumer coordination only.
