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
evidence: src/ast.rs; src/ast/text.rs; src/ast/normalize.rs; tests/ast.rs; verification below
---

# Normalize strings, tagged templates and struct declarations

The workspace [value work](../../docs/work/source-values-and-configurables.md#first-checkpoint-semantic-resolutions)
governs this slice and settles its string semantics. The [AST contract](../ast-interface.md)
owns the final data structures and adopted rules.

Scope: quoted and triple-quoted string expressions; tagged `f"…"`/`f"""…"""`
templates with ordered pieces, ordinary hole expressions and a purely syntactic
format specification; typed format-spec errors; and graph's prerequisite
`struct` declaration item with ordered, duplicate-preserving entries. Syntax
performs no join desugaring, typing, formatting, precision-range validation or
execution. Import paths remain `PathLiteral`, not string values. Inline nested
struct entry types and tuple annotations remain unsupported.

Agreed with `claude:cartan-graph`: one template node without desugaring; no empty
text pieces; raw spelling beside decoded text; standalone triple-string doc items
keep their structured rejection; struct identity comes from the declaration and
annotation, while the brace value stays a map expression.

Acceptance: empty, no-hole and adjacent-hole templates; Unicode; escaped braces;
quoted and raw multiline strings; nested hole expressions; digits and notation;
invalid formats as typed errors; struct entry order, duplicates and spans;
documented Rust, docs-tool and Python checks.

## Verification

- `claude:cartan-graph` accepted the template, string and struct shapes before
  edits; graph pins the tested supplier commit and owns its adapter.
- 57 Rust workspace tests pass, including four decoding unit tests and seven new
  AST tests for Unicode spans, quoted/raw strings, merged text and empty pieces,
  adjacent/nested holes, raw dedent across holes, U+0001 beside holes, all
  malformed precision/notation shapes, doc-declaration rejection, and struct
  order, duplicates, named annotations and inline-nesting refusal. The five
  workload fixtures normalize unchanged; `ir` prints the new nodes.
- Offline check, strict Clippy and rustdoc, rustfmt, diff whitespace,
  12-document validation, eight documentation-tool tests and 28 Python app tests
  pass. The existing Starlette deprecation warning remains.
- No grammar, CLI, dependency or execution change. Precision range validation,
  formatting, desugaring and struct semantics remain with consumers.
