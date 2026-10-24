---
id: python-app
title: Deliver the syntax Python development application
kind: work
status: done
implementation: implemented
topics: [syntax, tooling, interface]
owner: cartan-syntax-agent
depends: [ast-inspection]
authority: Owner-authorized supervisor assignment of the component application doctrine and a Sol builder on 2026-10-24
evidence: tests/python/test_application.py; independent wheel and Rust delegation checks below
---

# Deliver the syntax Python development application

Provide `syntax_dev` with installable standalone/nested commands and a mounted
documentation/diagnostics app. The [component contract](../component.md#development-application)
owns this app's behavior; the workspace's
[accepted application contract](../../../docs/records/component-apps.md) owns
the common registration/factory interface. No new analysis or Rust API is needed.

## Acceptance and current evidence

- A user-authorized `gpt-6.1-sol` builder implemented the package with exclusive
  file/build ownership. The component owner reviewed the handoff and then took
  exclusive ownership for fixes and verification; no concurrent builds occurred.
- Version-one registry mapping, explicit-context factories, standalone/nested
  Click commands, Rich presentation, Starlette/Uvicorn and packaged assets are
  implemented. Plain compiler output remains undecorated.
- Tests cover fake success/failure/unavailable/timeout results, cancellation
  reaping, a real existing Rust AST invocation, no subprocesses on imports/docs,
  source-path handling, config overrides, symlink containment, local links under
  named mounts/deployment prefixes, child lifespans and independent roots.
- Dependencies resolved normally offline into `uv.lock`, with Click 8.5.0,
  Rich 15.0.0, Starlette 1.8.0, Uvicorn 0.54.0, markdown-it-py 4.2.0,
  HTTPX 0.28.1 and pytest 9.1.1 on Python 3.12.13. Verification uses writable
  `UV_CACHE_DIR=/private/tmp/cartan-syntax-uv-cache`, populated from readable
  existing uv artifacts because PyPI DNS is unavailable in this session.
- Owner verification passed 20 Python app tests, all 29 Rust tests, workspace
  check/strict Clippy/rustdoc, formatting, metadata validation and seven existing
  documentation-tool tests. Source and wheel builds pass offline. An independently
  installed wheel exposes the exact registry and packaged CSS, displays help
  outside a checkout and delegates real Rust inspection with matching AST output.

No persistent server, GPU, SSH, publication, remote cleanup or legacy access is
part of this task. The installed HTTPX-based Starlette TestClient emits an
upstream deprecation warning; request behavior is tested. The subprocess service
currently uses POSIX process groups; other operating systems are untested.

Documentation review retains the app contract within `component.md`, concise
README commands and this bounded evidence record. No separate proposal or
parallel design document is introduced.
