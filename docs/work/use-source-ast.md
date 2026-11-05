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
evidence: tests/ast.rs; src/ast/normalize.rs; maintained AST contract
---

# Normalize imports and qualified source names

The [AST contract](../ast-interface.md#imports-and-qualified-names) owns
file/namespace use heads, optional alias or selectors, and qualified value/call
names. Source spelling and UTF-8 spans are retained. Absent selectors remain
distinct from wildcard selectors; single/braced name selectors share one
representation. File paths retain both raw and decoded text without filesystem
access or synthetic namespace bindings.

Alias-plus-selector combinations retain their typed form refusal. Namespace
functors, `with`/hole clauses and qualified annotations remain unsupported in
normalization. Ordinary String expressions are represented separately from
import paths. Graph is the direct AST consumer and owns import resolution,
namespace binding and deliberate supplier lock updates.

## Verification

The [AST tests](../../tests/ast.rs) retain selectors, aliases, decoded paths,
qualified segments/calls, Unicode locations, malformed imports and the complete
grammar diagnostics. The five workload fixtures retain their import items. No
file loading, module execution or builtin availability is inferred by syntax.
