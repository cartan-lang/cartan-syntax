---
id: web-styling
title: Apply shared styling to the syntax web app
kind: work
status: done
implementation: implemented
topics: [syntax, tooling, interface]
owner: cartan-syntax-agent
depends: [python-app, cartan-lang:web-styling]
authority: Owner-authorized uniform web styling assignment relayed by the supervisor on 2026-10-24
evidence: tests/python/test_application.py; independent wheel asset checks; coordinator browser review described below
---

# Apply shared styling to the syntax web app

Apply the coordinator's shared visual conventions to the syntax overview,
documentation and diagnostics. Preserve standalone asset packaging, deployment
and mount prefixes, JSON inspection behavior and explicit process execution.
The [workspace work item](../../../docs/work/web-styling.md) governs coordination;
the [component contract](../component.md#development-application) owns behavior.

Acceptance requires desktop and narrow browser review, visible keyboard focus,
readable documentation/code/tables, browser diagnostic results and errors, packaged
asset and mounted-route checks, existing Python tests and documentation validation.
No Rust changes, shared environment sync, benchmarks or existing service changes
are in scope.

## Completion evidence

- Shared typography, spacing, palette and components are packaged locally. The
  shell separates supplied workspace navigation from syntax navigation and
  identifies current pages. Overview cards, document lists/prose and diagnostics
  use the common conventions; missing documents retain HTTP 404 with a styled alert.
  Documents keep their own primary heading, with the file name as a small label;
  heading-free documents receive a page heading.
- HTML form submissions render escaped AST output/errors and preserve the source
  path. JSON submissions keep the existing response envelope. The small local
  script supplies pending feedback and restores controls after browser history
  navigation; JavaScript is unnecessary for inspection itself.
- All 22 Python tests pass, including named mounts/deployment prefixes, packaged
  script routing, form result escaping and unchanged JSON behavior. Seven existing
  documentation-tool tests, eight-document validation, JavaScript syntax and
  whitespace checks pass. The existing Starlette HTTPX deprecation warning remains.
- An offline wheel build passes. An independent wheel import, using the prepared
  environment only for dependencies, serves the packaged stylesheet and script
  exactly. No shared lockfile/environment, Rust source or build output changed.
- The styling coordinator reviewed actual Chromium rendering at 1440 and 375
  pixels, both standalone on port 8766 and mounted under `/dev/syntax` on port
  8770. Overview, index, documentation with code/tables and diagnostics returned
  200 with no document overflow, asset failures or JavaScript errors. Keyboard
  navigation exposed the skip link and visible focus. Real diffusion inspection
  and a nonexistent source produced readable success/error results at both widths
  and mounts. The component owner also inspected representative captured images.
- Local review artifacts are in workspace `out/web-styling/`, with
  `standalone-syntax-*.png`, `mounted-syntax-*.png` and `interactions.json`.
  The workspace work item records integrated evidence; screenshots are not a
  runtime dependency. Other browser engines were not reviewed.

Documentation review retains behavior in the component contract and this bounded
verification record, linked to the governing workspace task. No parallel design
or historical archive was introduced.
