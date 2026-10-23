---
id: documentation
title: Documentation conventions
kind: reference
status: accepted
implementation: implemented
topics: [documentation, tooling]
---

# Documentation conventions

Maintained documents describe the present state. Git holds previous versions;
documents contain no change logs, completed-work narratives or accounts of
superseded behavior. Historical benchmark observations remain valid data.

## File roles

- `README.md`: purpose, public entry points, local checks, limitations and next work.
- `AGENTS.md`: operational instructions for agents working in this component.
- `docs/*.md`: current designs, reference material, explanations and how-to guides.
- Source documentation: public types, functions and invariants beside their code.
- Tests: executable correctness evidence. Link to the tests relevant to a claim.

Keep one authoritative home for each fact and link to it instead of copying it.
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

All six fields are required. IDs are stable lowercase hyphenated identifiers,
unique within the repository. Topics use the same identifier syntax and have
no duplicates. The title is a nonempty one-line string. This restricted schema
does not accept YAML aliases, multiline values, nested objects or extra fields.

| Field | Allowed values or meaning |
| --- | --- |
| `id` | Stable document identifier; filename changes do not change it. |
| `title` | Human-readable document title. |
| `kind` | `design`, `reference`, `explanation`, `howto`. |
| `status` | `draft` or `accepted`; acceptance requires owner authorization. |
| `implementation` | `planned`, `partial`, `implemented`, `not-applicable`. |
| `topics` | Nonempty list of queryable subject identifiers. |

Approval and implementation are separate dimensions. An accepted design can
remain planned; a draft proposal does not claim implemented behavior. The
implementation field describes the document's scope, not the entire component.
Document identities can be qualified by repository name when used across repos.

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
```

The inventory is generated JSON. It is a query result, not a second maintained
catalogue. The checker validates the header schema and unique IDs; it does not
claim to validate prose correctness, implementation status or every link.
