---
id: ast-inspection
title: Inspect normalized source syntax from a file
kind: work
status: done
implementation: implemented
topics: [syntax, tooling]
owner: cartan-syntax-agent
depends: [normalized-ast]
authority: Owner asked how to view pretty-printed IR in the syntax agent conversation
evidence: examples/ast.rs; CLI smoke checks and component checks below
---

# Inspect normalized source syntax from a file

Provide a terminal entry point to inspect this component's normalized source
AST. Typed computational IR remains outside syntax. Usage and the output-format
limits live in the [README](../../README.md); the example delegates parsing to
the existing [AST interface](../ast-interface.md).

## Acceptance and verification

- [The example](../../examples/ast.rs) reads one UTF-8 source file and prints
  indented Rust debug output of its ordered AST items, including byte spans.
  The output is for inspection, not a stable interchange format.
- Direct execution passed for all five workload fixtures. Output includes
  declarations, update blocks and source spans.
- Missing arguments, extra arguments, missing files and incomplete syntax were
  checked: nonzero exit status, diagnostics on stderr, no AST on stdout.
- All 26 Rust tests, offline workspace check, formatting, strict Clippy and
  rustdoc checks pass. Documentation validation and seven Python tests pass.

Documentation review retained the README usage and this bounded task's evidence.
No separate design document was needed; no redundant document was created.
