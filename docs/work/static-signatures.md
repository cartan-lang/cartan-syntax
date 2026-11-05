---
id: static-signatures
title: Normalize static function-signature parameter annotations
kind: work
status: done
implementation: implemented
topics: [syntax, interface, frontend]
owner: cartan-syntax-agent
depends: [normalized-ast]
authority: Owner scheduled static multiple dispatch on 2026-10-27; Codex lead started this syntax checkpoint through codex:cartan-syntax; lead authorized syntax rows S1–S7 independently of open graph rows
evidence: tests/ast.rs; src/ast/normalize.rs; maintained AST contract
---

# Normalize static function-signature parameter annotations

The [AST contract](../ast-interface.md#function-signature-parameters) owns the
implemented parameter signature forms and source spans. Parenthesized zero/one/
many-argument signatures and unary `T -> R` signatures use `ParameterAnnotation`;
value parameters remain distinct. Slots use ordinary annotations, including
Structs, Tuples, Sums and `T|None`. Nested signatures and signatures outside
parameters retain their located form errors. Inline product descriptions and
named signature formals remain unsupported.

Graph owns overload resolution and static named-function binding under its
[resolution contract](../../../cartan-graph/docs/work/static-dispatch.md) and the
[workspace language scope](../../../docs/records/language-scope.md). Syntax
admission does not create runtime function values or an executable call ABI.

## Verification

The [AST tests](../../tests/ast.rs) retain zero/unary/multiple slots, generic and
structural annotations, Optional signature composition, Unicode byte spans and
every `SignaturePosition` diagnostic. The five workload fixtures remain represented
by the same AST. Actual Tree-sitter parser generation/execution is unverified.
