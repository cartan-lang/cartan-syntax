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

- `README.md`: purpose, public entry points, local checks, limitations and next work.
- `AGENTS.md`: standing operational instructions.
- `docs/*.md`: current designs, reference material, explanations and how-to guides.
- `docs/rulings/<id>.md`: one question or decision per file.
- `docs/records/<id>.md`: one durable adopted contract per file.
- `docs/work/<id>.md`: one work item or bounded task per file.
- Source documentation: public types, functions and invariants beside their code.
- Tests: executable correctness evidence. Link to the tests relevant to a claim.

Keep one authoritative home for each fact and link to it instead of copying it.
The master workspace owns cross-component decisions, integration tasks and agent
coordination. Components own local contracts, implementation tasks and evidence.
Both levels maintain this structure. No separate handoff or session-summary file
is maintained. Resume work through the inventory and the relevant task records.
The supervisor coordinates updates to this schema and the identical validator
and tests carried by each repository; standalone clones need no adjacent checkout.
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
| `kind` | `design`, `reference`, `explanation`, `howto`, `ruling`, `record`, `work`. |
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

Legacy's `docs/rulings/index.md` and `docs/work/index.md` at `a2916b3e9`
informed the separation of decisions, work status, ownership and completion
criteria. The supervisor inspected those sources. This schema retains the
workspace's YAML metadata and generated inventory, rather than legacy manual
status tables or append-only logs. No legacy ruling or work item is adopted by
this structural adaptation. Any substantive migration is reviewed individually
and records source revision/path, rationale and current authority.

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

```sh
python3 tools/docs.py check
python3 tools/docs.py inventory
python3 tools/docs.py inventory --kind design --status draft
python3 tools/docs.py inventory --topic interface
python3 tools/docs.py inventory --kind work --status active
python3 tools/docs.py inventory --kind ruling --status open
```

The inventory is generated JSON. It is a query result, not a second maintained
catalogue. The checker validates header fields, kind-specific statuses, required
decision/blocker/completion metadata, dates and unique IDs. It does not establish
actual authority, prose correctness, implementation truth or link/dependency
resolution. Cross-repository references must not require adjacent builds.
