---
id: structural-annotations
title: Normalize inline struct annotations and unary signatures
kind: work
status: done
implementation: implemented
topics: [syntax, interface, frontend]
owner: cartan-syntax-agent
depends: [static-signatures, string-interpolation]
authority: Owner clarified inline structural annotations and the unary arrow on 2026-10-27; Codex lead started this bounded checkpoint through codex:cartan-syntax
evidence: tests/ast.rs; src/ast/normalize.rs; maintained AST contract
---

# Normalize inline struct annotations and unary signatures

The [AST contract](../ast-interface.md#strings-templates-and-structs) owns inline
`AnnotationKind::Struct` entries; its
[signature contract](../ast-interface.md#function-signature-parameters) owns
unary `T -> R` and parenthesized signatures. These forms preserve names, entry
order, nested annotations and written spans. Optional and Sum annotations compose
through the existing annotation representation. Graph decides type identity,
eligibility and signature resolution.

Inline product descriptions and named signature formals such as `(x: Real) -> R`
remain unsupported. Ordinary value signatures do not introduce runtime function
values, returned function types or field reordering.

## Verification

The [AST tests](../../tests/ast.rs) retain unary/parenthesized equivalence, nested,
duplicate and empty inline Structs, generic arguments, Unicode spans, Optional
Struct fields and signature composition. Located errors distinguish forbidden
signature positions and unsupported inline product descriptions.
