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
evidence: src/grammar.rs; src/ast.rs; src/ast/normalize.rs; tests/ast.rs; verification below
---

# Normalize static function-signature parameter annotations

The workspace [static dispatch work](../../../docs/work/static-multiple-dispatch.md)
governs this slice. Graph owns overload resolution and keeps the one
authoritative [resolution table](../../../cartan-graph/docs/work/static-dispatch.md);
syntax links to it rather than restating its semantics. The lead authorized
this syntax-only subset (rows S1–S7) independently of the remaining proposed
and open graph rows, which stay unadopted and are not implemented here. The
[AST contract](../ast-interface.md) owns the final data structures.

Scope: `(T1, …, Tn) -> R` as a function-definition parameter annotation, with
mandatory parentheses for zero, one or many arguments. `Parameter.annotation`
becomes `Option<ParameterAnnotation>` with `Value(Annotation)` and
`Signature(Signature { arguments, result, arrow_span, span })`. Slots admit only
the existing named and generic-application annotations. Arrows anywhere else,
nested signatures in argument or result slots are
typed form errors carrying a position enum and the offending arrow span. Ordinary
value parameters are unchanged. The slice also corrects stale statements that
codegen consumes this AST: graph is the only direct consumer.

Excluded: overload markers, definition result annotations, optional `T|None`
and member, update, sum and merge forms. Syntax decides no concrete-type set,
specificity, conversion or overlap timing; those belong to graph and the lead.

Grammar plan: a `signature_type` alternative, `(` items of type annotations `)`
`->` result, is tried before the existing `fiber_type` reading inside
`type_annotation`, because fiber descriptions accept `()` as an empty product.
An optional trailing `->` after an ordinary annotation is the unary `T -> R`
spelling, adopted later in the [structural annotation work](structural-annotations.md). `type_annotation` appears in document bindings
(including `param`/`state`), body locals, parameters, namespace holes and generic
type arguments, so the position enum covers `Binding`, `Local`, `TypeArgument`,
`SignatureArgument` and `SignatureResult`. Struct entries and fiber components
take a bare `fiber_type`, so an arrow there remains a structured grammar error.

Unknowns: whether the signature alternative disturbs existing parenthesized
descriptions or the parameter colon readings, checked by the grammar and AST
suites. Namespace declarations and `use` hole clauses keep their structured
rejections, and no definition-result arrow is added.

Checkpoint: a tested normalization of the owner's example, zero/one/many slots, generic
annotations, spans and Unicode, typed rejections, unchanged string/struct and
five-workload tests and `ir` output, committed locally for graph to pin.

## Verification

- `claude:cartan-graph` agreed the `ParameterAnnotation` shape and position
  rules, and both Codex owners reviewed the proposal. Graph pins the tested
  commit. Resolution rows remain proposed or open in graph's table and are not
  implemented here.
- Three new AST tests (29 in `tests/ast.rs`) cover:
  - the owner's example, with parameter, signature, slot, result and arrow spans;
  - zero, one and three argument slots, value and unannotated parameters;
  - generic slots at UTF-8 offsets after Unicode text;
  - every `SignaturePosition`, with arrow and signature spans;
  - bare arrows in parameters and bindings;
  - grammar errors for struct-entry and fiber-component arrows;
  - unsupported optional slots and namespace holes.
- 61 Rust workspace tests, strict offline check, Clippy and rustdoc, rustfmt,
  diff whitespace, 13-document validation, seven documentation-tool tests, the
  generated grammar JavaScript syntax check and 28 Python app tests pass. The
  five workloads and the string/struct tests are unchanged; `ir` prints
  signatures and typed errors. Tree-sitter generation itself remains unverified.
- Stale consumer statements in README, `ast-interface.md` and `component.md`
  now name graph as the sole direct consumer.
