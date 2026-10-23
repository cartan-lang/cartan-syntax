---
id: normalized-ast
title: Normalize workload source syntax for frontend integration
kind: work
status: done
implementation: implemented
topics: [syntax, interface, frontend]
owner: cartan-syntax-agent
depends: []
authority: Owner parser-transfer assignment relayed by supervisor on 2026-10-23
evidence: Implementation commit 17fba7259087489b973fb70265a9191beaf4ce86; tests/ast.rs; checks below
---

# Normalize workload source syntax for frontend integration

Deliver syntax-owned parsing, AST normalization, source spans and structured
form diagnostics for the five existing numerical workloads. The
[AST contract](../ast-interface.md) owns the supported source forms, adopted
invariants, limitations and owner-authorized local Git dependency delivery. Semantic
analysis, numerical lowering and the consuming adapter belong to codegen.

## Acceptance and evidence

- [Public AST entry points and types](../../src/ast.rs) are implemented in
  commit `17fba7259087489b973fb70265a9191beaf4ce86`. Codegen reviewed and agreed
  the concrete interface before the handoff.
- [Normalization](../../src/ast/normalize.rs) uses the shared CST parser. It
  preserves written declarations, optional annotations, ordered body items and
  half-open byte spans. Bare/bracketed list patterns normalize alike; scalar
  names and positional site patterns remain distinct.
- [AST tests](../../tests/ast.rs) cover all five workloads, source spans, body
  order, pattern equivalence, nested patterns, annotations, ranges, reductions,
  grouping/precedence, unsupported optional clauses and exact grammar errors.
  Full workspace tests passed: 26 Rust tests, including seven AST tests.
- `cargo fmt --check`, offline workspace check/tests, strict Clippy and
  `RUSTDOCFLAGS=-Dwarnings cargo doc --workspace --no-deps --offline` passed.
  `git diff --check` and generated grammar JavaScript's Node syntax check passed.
- Shared documentation conventions and validators match the supervisor's
  copies. Metadata check, work inventory and all seven documentation-tool tests
  pass with this task recorded as done.

The syntax task has no remaining interface questions. Codegen owns integration
and its locked Git revision evidence; acceptance by syntax does not establish frontend
execution support. Actual Tree-sitter parser generation/execution and device
execution were not run by this component. Legacy evidence was supplied by the
supervisor; no legacy checkout was read or executed here.

The agreed AST is available on `main`. The owner authorizes consumers to use the
local Git URL and lock a committed revision, as described by the
[dependency contract](../ast-interface.md#local-dependency-delivery). Specialized
branches are unnecessary for this interface; future consumer-specific requests
are implemented and committed by the syntax component.
