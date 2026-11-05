---
id: string-interpolation
title: Normalize strings, tagged templates and struct declarations
kind: work
status: done
implementation: implemented
topics: [syntax, interface, frontend]
owner: cartan-syntax-agent
depends: [normalized-ast, use-source-ast]
authority: Owner started source value work and added string interpolation on 2026-10-27; Codex lead delegated this slice and graph's struct prerequisite through codex:cartan-syntax
evidence: tests/ast.rs; src/ast/normalize.rs; maintained AST contract
---

# Normalize strings, tagged templates and struct declarations

The [AST contract](../ast-interface.md#strings-templates-and-structs) owns quoted
and triple-quoted strings, tagged templates and Struct declarations. Raw source
spelling stays beside decoded text; templates preserve ordered text/hole pieces,
ordinary hole expressions and structured precision/notation fields. Import paths
remain `PathLiteral` values. String decoding is syntax-only; formatting, type
checking and precision policy belong to consumers.

Struct declarations preserve duplicate entries and their order for analysis.
Inline nested Struct and Tuple annotations are supported through the shared
annotation interface. Standalone triple-quoted documentation items retain their
explicit unsupported-form diagnostic.

## Verification

[Decoding tests](../../src/ast/text.rs) and [AST tests](../../tests/ast.rs) retain
quoted escapes, raw multiline indentation, Unicode spans, adjacent/nested holes,
raw text across hole boundaries, control characters, malformed format fields and
Struct entry provenance. These are the maintained acceptance evidence; actual
Tree-sitter parser generation/execution remains unverified.
