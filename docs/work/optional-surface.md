---
id: optional-surface
title: Normalize the optional-value surface
kind: work
status: done
implementation: implemented
topics: [syntax, interface, frontend]
owner: cartan-syntax-agent
depends: [members-and-patterns, structural-annotations]
authority: Owner released slice 2 of the workspace optional/members/patterns/sums work on 2026-10-27, assigned to syntax by the lead
evidence: tests/ast.rs optional and absence tests; src/ast/normalize.rs
---

# Normalize the optional-value surface

The [AST contract](../ast-interface.md#optional-values) owns the completed syntax
interface for `none`, else-less conditionals, `T|None` annotations, presence default
`??` and postfix absence marks. Optional Struct fields and signature slots use
the same annotation representation. Graph owns presence proofs, lifting, fitting
and default behavior; syntax does not infer these semantic facts.

## Verification

The [AST tests](../../tests/ast.rs) retain default precedence/right association,
continuation and exact token spans, nearest-else binding, marked argument/index/
component/binding/collection positions, Optional signature composition and nested
Optional Struct annotations. Typed `UnionAlternative` diagnostics and grammar
refusals protect the current annotation laws. No remaining syntax work is
assigned by this completed item; semantic scope belongs to the
[workspace Optional contract](../../../docs/records/optional-values.md).
