---
id: documentation
title: Documentation conventions
kind: reference
status: accepted
implementation: implemented
topics: [documentation, tooling]
---

# Documentation conventions

Maintained documents describe the present state. Git holds previous versions
where the directory is version-controlled;
documents contain no change logs, completed-work narratives or accounts of
superseded behavior. Historical benchmark observations remain valid data.

## File roles

- `README`: plain text, two or three sentences stating the component's purpose.
- `AGENTS.md`: standing operational instructions.
- `HANDOFF.md`: ignored local resume note for the directory's owning agent.
- `docs/*.md`: current designs, reference material, explanations and how-to guides.
- `docs/rulings/<id>.md`: one question or decision per file.
- `docs/direction/<id>.md`: one steering theme, its priority and horizon. Direction
  states intent and success criteria; it does not authorize implementation or
  replace a record, ruling or work item. Cross-component direction belongs at
  the workspace root; components may keep component-local direction.
- `docs/records/<id>.md`: one agreed contract per file, whether planned, partially
  implemented or implemented; the `implementation` field distinguishes these.
- `docs/work/<id>.md`: one tracked, disposable delivery plan or bounded task per
  file. Do not Git-ignore this directory: active scope, blockers and acceptance
  criteria must survive sessions and remain reviewable.
- Source documentation: public types, functions and invariants beside their code.
- Tests: executable correctness evidence. Link to the tests relevant to a claim.

Keep one authoritative home for each fact and link to it instead of copying it.
The master workspace owns cross-component decisions, integration tasks and agent
coordination. Components own local contracts, implementation tasks and evidence.
Both levels maintain this structure. Each directory's owning agent maintains
an untracked `HANDOFF.md`, excluded by `/HANDOFF.md` in the repository's
`.gitignore`. The workspace root uses the same ignore rule as components. On "update HANDOFF", refresh that file
in place with owner and working directory, current state and revision, active
work or blockers, next actions, verification and links to authoritative docs.
Read it when resuming; confirm its state against the checkout and task records.
Keep it concise and replace stale information rather than appending a diary.
It is a local resume aid, not a second status catalogue: durable decisions,
contracts and task evidence remain in structured docs. The handoff is outside
`docs/` and does not require document metadata or enter the generated inventory.
The supervisor coordinates updates to this schema and its validation.
One validator serves every repository: the workspace module `cartan_dev.docs`,
run as `cartdev docs check --root <component>` from the workspace root.
Documentation is checked from the workspace, and a standalone clone has no
checker. Each component keeps a byte-identical copy of this file, and
`cartdev docs drift` reports copies that differ from the workspace.
Design documents may state approved behavior that is not yet implemented, but
must identify the gap. Reference material describes implemented behavior.

## Metadata

Each Markdown file under `docs/` begins with YAML front matter: a structured
header bounded by lines containing `---`. The deliberately small supported
subset uses plain one-line scalars and an inline list of topic identifiers.

```yaml
---
id: target-module
title: Target module interface
kind: design
status: draft
implementation: planned
topics: [codegen, interface]
---
```

These six fields are required for every kind. IDs are stable lowercase hyphenated identifiers,
unique within the repository. Topics use the same identifier syntax and have
no duplicates. The title is a nonempty one-line string. This restricted schema
does not accept YAML aliases, multiline values, nested objects or unspecified fields.

| Field | Allowed values or meaning |
| --- | --- |
| `id` | Stable document identifier; filename changes do not change it. |
| `title` | Human-readable document title. |
| `kind` | `design`, `reference`, `explanation`, `howto`, `ruling`, `record`, `work`, `direction`. |
| `status` | Kind-specific lifecycle below; approval reflects actual authorization. |
| `implementation` | `planned`, `partial`, `implemented`, `not-applicable`. |
| `topics` | Nonempty list of queryable subject identifiers. |

Approval and implementation are separate dimensions. An accepted design can
remain planned; a draft proposal does not claim implemented behavior. The
implementation field describes the document's scope, not the entire component.
Document identities can be qualified by repository name when used across repos.

## Lifecycles and additional fields

Designs, references, explanations, how-to guides and records use `draft` or
`accepted`. An accepted record also requires `authority` (who authorized the
contract and where that authorization is recorded) and `decided` (`YYYY-MM-DD`).
Record bodies state scope, the contract, rationale, limits and relevant evidence.

Direction uses `draft` or `accepted` and always has `implementation: not-applicable`.
It requires `priority` (`high`, `moderate`, `low`) and `horizon` (`now`, `near`, `far`).
`now` means next work to schedule, `near` means after current commitments, and
`far` means exploratory. Accepted direction requires `authority` and a valid
`decided` date. Priority and horizon are exclusive to this kind. Keep decided
requirements distinct from proposals; adopted contracts belong in records and
unsettled choices in rulings. Acceptance of a direction is not an implementation
assignment or evidence that a proposed mechanism has been delivered.

Rulings use `open`, `approved`, `declined` or `withdrawn`. Every ruling requires
`proposer` and `decision_maker`. Approved or declined rulings also require
`authority` and `decided`; open or withdrawn rulings carry no decision metadata.
The body states the question, alternatives, rationale and, when decided, the
actual decision. Link an adopted durable contract to its authoritative record.
Do not require a separate ruling when an explicit owner instruction already
settles the matter; attribute that instruction directly in the record.

Work uses `planned`, `approved`, `active`, `blocked`, `done` or `dropped`:

- `planned`: proposed work without an execution assignment.
- `approved`: authorized work ready to be taken up.
- `active`: implementation or investigation is underway.
- `blocked`: progress waits on a named condition or dependency.
- `done`: the stated acceptance criteria have evidence.
- `dropped`: the work is no longer pursued; state why in the body.

Work may carry an optional `direction` reference using a local ID or a
repository-qualified ID such as `cartan-lang:demand-planning`. The checker
validates identifier syntax, not reference existence or kind. The field is
exclusive to work items and remains visible in generated inventory.

Every work item requires `owner` and `depends`, an inline list of local IDs or
repository-qualified IDs such as `[cartan-syntax:normalized-ast]`; use `[]` when
there are no dependencies. Use `cartan-lang` for the master workspace. Approved,
active, blocked and done work requires `authority`, identifying the actual
assignment or authorization. Blocked work requires a nonempty `blocked_on`;
remove it when the blocker clears. Done work requires `evidence`, pointing to
the completion evidence. Other evidence belongs in the body or linked component
verification document. Status does not itself create authorization.

The body states scope, acceptance criteria, dependencies and current evidence
or remaining questions. Keep only current facts, not append-only activity logs.
At completion, consolidate lasting requirements into records, usage into maintained
documentation, and verification into the relevant contracts, tests or benchmark
data. Then delete the completed work item by default and repair incoming links
and dependency references. Retain a concise completed item only while its
acceptance evidence serves a continuing purpose. Git preserves deleted plans;
ignored HANDOFF files and mailboxes hold temporary session state. The work
directory is not a permanent task archive.
The `implementation` field still describes delivered capability: a completed
investigation may be `not-applicable`, while an active task may be `partial`.
Additional fields are plain one-line scalars except the `depends` list. Only
the kind-specific fields described here are allowed.

```yaml
---
id: module-loader
title: Load modules through their declared interfaces
kind: work
status: blocked
implementation: planned
topics: [runtime, interface]
owner: runtime-agent
depends: [module-interface]
authority: Owner assignment recorded in module-interface
blocked_on: Module interface decision remains open
---
```

The schema separates decisions, work status, ownership and completion
criteria through YAML metadata and a generated inventory, not manual status
tables or append-only logs. Material migrated from earlier work is reviewed
individually and restated as current design with its rationale and current
authority; documents do not cite the earlier codebase.

## Content and vocabulary

- Begin with purpose and scope. State the current contract, examples and limits.
- Define technical terms where first introduced, including expanded acronyms.
- Prefer mainstream terminology. Establish unfamiliar or project-specific terms
  with the owner before committing them as the project's vocabulary.
- Separate normative requirements from explanatory rationale and proposals.
- Keep open questions explicit. Once decided, update the current contract and
  remove the question; recover the discussion from Git when necessary.
- Keep commands reproducible and links relative where possible. A cross-repository
  path is optional navigation, never an undocumented build dependency.
- Give measurement claims their revision and environment, and distinguish checks
  actually executed from unavailable or skipped targets.
- Remove documents that no longer have current purpose; Git retains them.

## Validation and inventory

At the workspace root:

```sh
uv run cartdev docs check
uv run cartdev docs inventory --kind work --status active
uv run cartdev docs inventory --kind direction --priority high --horizon near
uv run python -m unittest discover -s tests -p 'test_*.py'
```

```sh
uv run cartdev docs check --strict
uv run cartdev docs drift
```

For a component, also from the workspace root:

```sh
uv run --no-sync cartdev docs check --root <component> --strict
uv run --no-sync cartdev docs inventory --root <component>
uv run --no-sync cartdev docs inventory --root <component> --kind work --status active
uv run --no-sync cartdev docs inventory --root <component> --kind ruling --status open
```

The inventory is generated JSON. It is a query result, not a second maintained
catalogue. The checker validates header fields, kind-specific statuses, required
decision/blocker/completion metadata, dates and unique IDs, and resolves relative
links and anchors: a broken link or anchor inside the repository is an error.
It also reports content that tends to go stale, as warnings that `--strict`
turns into failures: links into a sibling repository that do not resolve,
commit-like hexadecimal hashes in README, records and work prose, citations of
ignored `out/` paths, `done` work items still present, and work items over 300
lines. Links into an absent sibling are skipped, so standalone clones need no
adjacent checkout. The checker does not establish actual authority, prose
correctness or implementation truth.
