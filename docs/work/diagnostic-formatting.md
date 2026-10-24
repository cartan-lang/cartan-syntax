---
id: diagnostic-formatting
title: Adopt central formatting of structured diagnostics
kind: work
status: done
implementation: partial
topics: [syntax, interface, tooling]
owner: cartan-syntax-agent
depends: [ast-inspection, python-app]
authority: Owner's workspace-wide diagnostic doctrine relayed by the supervisor on 2026-10-24
evidence: cli/src/diagnostic.rs formatter tests; Rust, Python and documentation checks below
---

# Adopt central formatting of structured diagnostics

Apply the explicit doctrine to local guidance and the authoritative
[diagnostic contract](../component.md#diagnostics), audit current behavior and fix
small CLI violations. The scope does not redesign Python process transport.

Acceptance: Rust operations propagate typed facts and original I/O causes to a
single central CLI formatter; formatter tests own wording assertions; process
checks cover status/streams and library tests continue checking typed spans and
payloads. Run the documented Rust/Python/docs checks and report larger gaps.

## Existing migration gaps

- `src/syntax_dev/service.py` constructs launch and timeout prose in `stderr`.
  A future migration needs structured launch/timeout facts, timeout duration and
  original exceptions kept until presentation, separate from opaque child output.
- `src/syntax_dev/application.py` generates configuration `ValueError` messages,
  HTTP validation messages and result summaries in separate branches. They need
  typed failure payloads and one central application diagnostic formatter shared
  by CLI, JSON and HTML presentation.
- The version-one process-result envelope contains status/exit code and opaque
  stdout/stderr; it is not a structured Rust diagnostic channel. Preserving spans
  across this process boundary needs an agreed transport contract rather than
  parsing debug/error strings or silently changing the envelope.

These gaps are reported for coordinated follow-up, not presented as compliant.
No Python behavior change or transport redesign is part of this bounded task.

## Completion evidence

The bounded doctrine/audit and small Rust CLI correction are complete; overall
implementation remains partial because the Python gaps above are unresolved.
`CliError` retains typed usage/read/parse/output variants, parse path and AST
errors/spans, and original I/O causes exposed through `Error::source`. Operations
return errors to `main`; the sole diagnostic `Display` implementation lives in
`cli/src/diagnostic.rs`. Parse diagnostics now include the input path. No-argument
help, broken-pipe success and failure exit/stream behavior remain unchanged.

Parser wording assertions moved from CLI process tests to the formatter's tests.
The Python HTML test checks error presentation structure/status instead of a
specific diagnostic sentence. Existing external-stream preservation assertions
remain valid because those streams are opaque data, not locally generated wording.

Verification passed: 31 Rust workspace tests (including two formatter tests),
offline workspace check, strict Clippy/rustdoc, default rustfmt, all 22 Python app
tests with `uv run --no-sync --package cartan-syntax-dev`, nine-document metadata
validation, seven documentation-tool tests and diff whitespace checks. The
existing Starlette HTTPX deprecation warning remains. No shared sync, dependency
change, device run, helper or legacy access occurred.

Documentation review retains the adopted rule in AGENTS and the component
contract, with audit findings and completion evidence here. No parallel design
or historical archive was introduced. Larger Python transport work remains for
coordinated follow-up; this record does not claim complete application compliance.
