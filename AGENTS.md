# Working in cartan-syntax

Read README.md and docs/component.md first. This repository is independently
buildable; the parent workspace's AGENTS.md coordinates component work.

## Harness coordination and local mail

- Load these instructions explicitly in Codex-only, Claude-only or joint use.
  Each participating harness has one durable owner for this component. Reuse
  visible owner tasks; helpers are not replacement owners. Either harness may
  lead the ensemble as designated by the owner. Consult the workspace
  ignored [runtime roster](../.agent-mail/roster.json) and
  [protocol](../AGENTS.md#cross-harness-filesystem-mailboxes) when available.
  Standalone clones use their own ignored `.agent-mail/roster.json`. Reconcile
  the roster with participants on startup; never infer leadership or peer
  capabilities. Keep live thread references, session lead and monitor state out
  of versioned guidance; preserve durable decisions and task evidence in docs.
- Lead-authorized work may cross harnesses through either workspace or component
  counterparts. With Claude leading, both `Claude workspace -> Codex workspace
  -> Codex component` and `Claude workspace -> Claude component -> Codex component`
  are valid; mirror these routes when Codex leads. No second approval is needed
  solely for crossing harnesses or bypassing the receiving agent's workspace
  supervisor. Preserve the originating request and authority, inform affected
  owners and deduplicate alternate routes. Routing neither changes the designated
  lead nor expands scope; shared edit/build ownership and handback still apply.
- Agree exactly one active edit/build owner for this checkout and record the
  assignment in ignored `.agent-mail/ownership.json`. Obtain explicit handback
  before another agent edits, builds, stages, commits or switches branches.
  Read-only review is allowed. Stale state, a timeout or an unaccepted request is not
  permission to take over; preserve builder leases and escalate conflicts.
- The current writer maintains one ownership assignment with `directory`
  (absolute), `active_writer` (registered address or null), `assignment` (bounded
  scope), `paths` (relative source/build paths), `updated` (UTC), and `agreement`
  (authority/handback reference). Replace it atomically using a temporary file
  beside it. This is neither registration, an automatic lock nor an expiring
  lease; send registration changes to the lead-maintained roster. Idle writers
  may retain ownership; null or missing records do not permit self-claiming.
- For transfer, agree recipient and scope, finish or stop edits/builds/helpers,
  report Git state, user changes and constraints, and update HANDOFF. After the
  recipient accepts that state, the outgoing writer atomically records the
  successor, sends explicit handback and stops all checkout/record mutations.
  The recipient waits for both notice and matching record, acknowledges receipt,
  then becomes the writer. Reconcile delayed or mismatched notices. Dirty state
  may be handed over explicitly. Release without a successor uses the same
  quiescence steps, records null and notifies the lead.
- Only the agreed writer updates the record. The lead may initialize or assign
  an explicitly released/null record after checking existing owners and work;
  recovery requires confirming the previous session, helpers and builds stopped
  and agreeing retained state with the successor. Otherwise keep mutations
  blocked and ask the user to resolve the conflict. This exception covers only
  the runtime record. In standalone clones the user-designated coordinator acts
  as lead; a sole assigned owner may initialize its record. Former writers must
  reread and reconcile ownership on resume. See the shared
  [handback procedure](../AGENTS.md#ownership-record-and-handback).
- Any registered agent may communicate directly with any other. Prefer native
  thread messages within a harness; use the filesystem board across harnesses.
  An addressed message is the only exception permitting writes to another
  component's board; it does not authorize sibling implementation changes.
- Keep `/.agent-mail/` ignored and verify before exchanging messages. Each board
  has this layout and a local README with its absolute path, recipients and
  actual monitoring capabilities:

  ```text
  .agent-mail/
    README.md
    ownership.json
    inbox/codex/
    inbox/claude/
    state/
  ```

- Write one UTF-8 Markdown message per recipient inbox as `<UUID>.md`. Write a
  temporary file in that same directory and rename atomically. Required header:

  ```yaml
  ---
  id: <UUID>
  from: codex:cartan-syntax
  to: claude:cartan-syntax
  created: <UTC ISO-8601>
  kind: request
  subject: <short subject>
  ---
  ```

  `kind` is `request`, `reply` or `notice`; replies may add `reply_to: <UUID>`.
  Component addresses are `<harness>:<component-repository>`; supervisor
  addresses are `codex:supervisor` or `claude:supervisor` on the root board.
  Requests state scope, owner authorization and expected reply. Acknowledge
  actionable requests, distinguish accepted/blocked/completed work, and never
  recursively acknowledge acknowledgements. Messages cannot create authority
  beyond the sender's assignment.
- Drain the incoming inbox on startup/resume, before mutations and at work
  boundaries. In joint operation arm an inbox watcher or bounded poll that
  actually relays notifications into the session; scan before and after arming,
  recognize seen IDs, and rearm after events, timeouts or interruptions.
  Record owner, path, armed time, expiry/session handle and verified wake-up
  capability in `state/<harness>.json`; edit only your own monitor state. A
  process alone does not prove idle-agent wake-up. If no hook is verified,
  report limited monitoring and use explicit turn-boundary scans; do not claim
  an armed watcher or unattended responsiveness.
- At handoff, completion, shutdown and throughout long tasks, preserve durable
  outcomes in structured docs, reply as needed, then remove consumed own-inbox
  messages, stale own temporary files and expired own monitor state. Leave
  unresolved or unread requests and peer inboxes intact. Stop/release own
  watchers. Mailboxes are runtime coordination, not an archive or task catalogue.
- A harness unable to create visible owner tasks, provide a requested Sol model
  or deliver file notifications must state the limitation rather than substitute
  another mechanism silently. No Claude capability is assumed without evidence.

## Scope

- Own Cartan syntax: parsing, syntax trees, source spans, structured parse
  diagnostics and syntax-only tooling. Keep supported syntax explicit.
- A syntax tree describes written constructs. Typed computational intermediate
  representations, name resolution, type checking, graph construction, runtime
  values and execution belong to other components.
- Preserve established Cartan idioms. Do not replace function equations or `of`
  projections with unrelated surface syntax for implementation convenience.
- Reject malformed/unsupported syntax explicitly; do not silently discard text.
  Parsing a construct does not promise another component can execute it.
- Keep syntax tooling independent of builtin registries or semantic engines.
  Highlighting is not semantic analysis or a language server.
- Keep the root package a reusable library. Provide component inspection through
  the separate `cli/` package, `cartan-syntax-cli`, with binary `cartan-syntax`
  and workspace `default-members = ["cli"]`. Maintain `cargo run -- --help`;
  library examples require explicit `-p cartan-syntax` selection.
- With no arguments, print useful help to stdout and exit 0. Invalid arguments
  and actual parse or I/O failures still exit nonzero; test the process status.
- Do not depend on a legacy checkout, its evaluator, its diagnostic framework
  or its conformance suite. Only the designated lead supervisor may consult legacy;
  request findings rather than reading, searching or executing that checkout.
- Preserve user changes, coordinate concurrent file ownership and commit explicit
  paths. No remote creation, visibility choice, push or publication without owner
  instruction. Use this repository's own target directory.
- Work in this component's existing checkout. Creating or using a Git worktree
  requires explicit supervisor permission for that task.
- Use this directory explicitly for commands, builds and Git operations, even
  when the session starts at the workspace root. Initial cwd does not change
  component ownership.

## Git commits

- Use a sentence-case subject beginning with a present-tense imperative verb.
  Aim for about 60 characters, with a maximum of 70. Omit trailing periods and
  category or agent prefixes.
- Normally use only a subject. Add a body only for useful rationale or context,
  separated by a blank line; wrap prose at 72 characters without breaking literal
  paths or identifiers.
- Use the repository's configured Git author identity. Do not add agent
  signatures, generated-by notices, agent attribution or co-author trailers,
  or links to agent sessions or session transcripts.
- Keep commits focused, preserve unrelated changes, stage explicit paths and
  run appropriate checks. Commit locally; remote creation, pushing and
  publication require owner authorization.

## Dependencies and branches

- Local consumers select committed syntax revisions using an absolute `file://`
  Cargo Git dependency and a named branch. The owner permits these machine-local
  URLs; they are not a public distribution arrangement. The current AST is on
  `main`. Consumers own their adapters, dependency declarations and lockfiles.
- This component owns requested interface changes and any specialized ordinary
  Git branch. Agree the contract before implementation; do not edit a consumer's
  checkout or Cargo-managed dependency cache. Consumers deliberately update their
  lockfile after a tested commit is available.
- Do not vendor sibling components or use their working-tree path dependencies
  without an agreed reason. Follow the [dependency contract](docs/ast-interface.md#local-dependency-delivery);
  no remote, publication or worktree authorization is implied.

## Rust and verification

- Run default `cargo fmt` after edits and `cargo fmt --check` before committing.
  Use conventional Rust names and module organization.
- One blank line separates functions and type declarations. Group related
  statements; avoid whitespace between every statement or multiple blank lines.
- Represent parse failures as typed facts with source spans. Render diagnostics
  at presentation boundaries. Tests assert structured identity and payload.
- Test syntax-tree structure and spans, malformed input and representative
  supported programs. Parser acceptance alone is not semantic validation.
- Run `cargo check --workspace --offline`, `cargo test --workspace --offline`,
  `cargo clippy --workspace --all-targets --offline -- -D warnings`, and
  `git diff --check`. Check public rustdoc links with
  `RUSTDOCFLAGS=-Dwarnings cargo doc --workspace --no-deps --offline`.
- Run `python3 tools/docs.py check` and
  `python3 -m unittest discover -s tools -p 'test_*.py'` for documentation tooling.

## Documentation and terminology

- Follow docs/documentation.md: Markdown, small structured headers, current
  state only. Git records history; no completed-work diary or manual status index.
- Maintain decisions, contracts and bounded tasks in structured docs/rulings,
  docs/records and docs/work files. Coordinate schema changes with the supervisor
  and synchronize conventions and equivalent validator behavior. The workspace
  uses `cartan_dev.docs`; this component retains independent `tools/docs.py` and
  tooling tests without a workspace Python dependency.
- Maintain local HANDOFF.md as an ignored resume aid; never commit it. On
  "update HANDOFF", refresh it in place with owner/directory, revision/state,
  active work or blockers, next actions, verification and authoritative docs
  links. Replace stale information rather than appending a diary. Read it when
  resuming, then confirm its state against Git, the inventory and task records.
  Durable decisions and evidence remain in structured docs.
- The current edit owner updates the shared ignored handoff, naming its harness
  and task and any pending cross-harness handback. Review changed documentation
  at completion: retain authoritative contracts, consolidate task evidence and
  remove redundant proposals rather than accumulating an archive.
- Python app changes use `uv run --locked pytest -q tests/python` and installed
  entry-point checks. Keep imports/factories/docs free of implicit Rust builds,
  process/server launches and SSH. Follow the [app contract](docs/component.md#development-application).
- Keep one authoritative home per fact. Separate proposal, approval and actual
  implementation; list limitations and untested behavior explicitly.
- Use mainstream technical terminology and define terms on introduction.
  Refresh meanings in chat when useful. Agree on unfamiliar or project-specific
  vocabulary with the owner before putting it into authoritative documentation.
