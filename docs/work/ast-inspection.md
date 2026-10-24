---
id: ast-inspection
title: Inspect normalized source syntax from a file
kind: work
status: done
implementation: implemented
topics: [syntax, tooling]
owner: cartan-syntax-agent
depends: [normalized-ast]
authority: Owner requested source inspection, uniform Cargo CLI layout and successful no-argument help
evidence: cli/tests/cli.rs; five-fixture smoke check; workspace and documentation checks below
---

# Inspect normalized source syntax from a file

Provide a terminal entry point to inspect this component's normalized source
AST. Typed computational IR remains outside syntax. Usage and the output-format
limits live in the [README](../../README.md); the CLI delegates parsing to
the existing [AST interface](../ast-interface.md).

## Acceptance and verification

- [The CLI](../../cli/src/main.rs) reads one UTF-8 source file and prints
  indented Rust debug output of its ordered AST items, including byte spans.
  The output is for inspection, not a stable interchange format.
- Direct execution passed for all five workload fixtures. Output includes
  declarations, update blocks and source spans.
- No arguments prints the same help as `--help` to stdout and exits 0, verified
  by invoking the actual binary. Missing `ast` operands, extra arguments, unknown
  commands, missing files and incomplete syntax were
  checked: nonzero exit status, diagnostics on stderr, no AST on stdout.
- The [package contract](../component.md#syntax-tooling) uses a separate CLI
  workspace member selected by default. Library dependencies remain unchanged.
  Root `cargo run --offline -- --help` and `cargo run --offline -- ast` with a
  fixture select the binary. Explicit library-only checking and the grammar
  example also pass. Diffusion AST output matches the previous example exactly.
- All 29 Rust tests pass, including three [CLI tests](../../cli/tests/cli.rs).
  Offline workspace check, formatting, strict Clippy/rustdoc, generated grammar
  JavaScript syntax check, metadata validation and seven Python tests pass.

Documentation review retained README usage, the package contract and this bounded
task's evidence. The example-only executable was removed and its references
updated. No separate design document was needed.
