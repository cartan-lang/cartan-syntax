---
id: diagnostic-formatting
title: Carry structured failures through the Python development app
kind: work
status: planned
implementation: partial
topics: [syntax, interface, tooling]
owner: cartan-syntax-agent
depends: []
---

# Carry structured failures through the Python development app

The Rust side follows the [diagnostic contract](../component.md#diagnostics):
typed errors reach one central formatter, and wording lives in
`cartan-syntax-diagnostics`. The Python development app does not yet:

- `src/syntax_dev/service.py` writes launch and timeout prose into `stderr`
  instead of returning structured launch/timeout facts, the timeout duration
  and the original exception.
- `src/syntax_dev/application.py` raises message-bearing `ValueError`s for
  configuration and builds HTTP validation messages and result summaries in
  separate branches, rather than through one application formatter shared by
  CLI, JSON and HTML presentation.
- The version-one web result envelope carries status and opaque child streams.
  Rust diagnostics cannot be reconstructed from it; preserving their spans
  needs an agreed transport, such as the CLI's `--json` result, never parsing
  rendered text.

Acceptance: typed Python failure payloads with original causes, one central
Python formatter used by every presentation, and a transport that carries the
Rust `--json` diagnostics. Verify with
`uv run --no-sync --package cartan-syntax-dev pytest -q tests/python`.
No execution assignment exists; this is a planned follow-up.
