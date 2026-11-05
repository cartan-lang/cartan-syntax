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
evidence: tests/ast.rs; src/ast/normalize.rs; maintained AST contract
---

# Normalize maps and configurable source forms

The [AST contract](../ast-interface.md#maps-and-configurables) owns literal
entries, join nodes, ordered configurable settings atoms with a body, and
colon/literal-dot application. Static and computed entry keys retain their
source forms, order, duplicates and exact punctuation spans. Bodies preserve
local/result order with exactly one result.

Embedded Map walks, parameter defaults, function-header configuration and
named/trailing-dot closing remain outside normalized syntax. Graph owns
configurable semantics, type checks and execution limits. Static named-function
arguments are implemented downstream within the bounded
[workspace language scope](../../../docs/records/language-scope.md); their
representation does not imply runtime function values or dynamic calls.

## Verification

The [AST tests](../../tests/ast.rs) retain empty/nested/mixed entries, duplicate
provenance, UTF-8 spans, precedence, continuation punctuation, ordered settings
and chained application, body errors and unsupported forms. Actual Tree-sitter
parser generation/execution is unverified. The contract records the open SYN-1
nested-value-body parser defect; this completed delivery item does not imply a
repair or complete grammar coverage.
