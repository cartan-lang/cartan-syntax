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
evidence: src/ast.rs; src/ast/normalize.rs; tests/ast.rs; verification below
---

# Normalize inline struct annotations and unary signatures

The [AST contract](../ast-interface.md) owns the adopted shapes: inline
`AnnotationKind::Struct` entries and the unary `T -> R` signature spelling, as
agreed with `claude:cartan-graph`. Every form already parsed, so the grammar is
unchanged; only normalization changed.

Lead rulings for this slice: inline product descriptions stay unsupported (not a
final policy), named fiber aliases are graph semantics, there are no named
signature formals such as `(x: Real) -> R`, and graph decides where an inline
struct is semantically eligible. No runtime function values, returned function
types or field reordering.

## Verification

- Two new AST tests and two adapted ones (31 in `tests/ast.rs`) cover:
  - the owner's `f(x: Real, g: Real -> {a: Real, b: Real})`, with written spans;
  - the same shape for unary and parenthesized spellings with named, generic and
    inline-struct arguments;
  - nested, duplicate and empty inline structs with brace spans at UTF-8 offsets,
    inside `Array<…>` and struct entries;
  - admission in bindings, locals, fiber components and `List<…>`;
  - unary arrows refused in bindings, locals and chained results;
  - unsupported inline products, `(x: Real) -> R` and optional arguments.
- The five workloads and the string, struct and signature tests are unchanged;
  `ir` prints the new nodes. `UnparenthesizedSignature` is retired.
