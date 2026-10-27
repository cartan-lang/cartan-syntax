---
id: members-and-patterns
title: Normalize component reads, updates and destructuring patterns
kind: work
status: active
implementation: partial
topics: [syntax, interface, frontend]
owner: cartan-syntax-agent
depends: [normalized-ast, structural-annotations]
authority: Owner released slice 1 of the workspace optional/members/patterns/sums work on 2026-10-27, assigned to syntax by the lead
---

# Normalize component reads, updates and destructuring patterns

The syntax part of slice 1 of the workspace
[optional members, patterns and sums](../../../docs/work/optional-members-patterns-and-sums.md)
work. The adopted rules live in the workspace
[component reads and updates](../../../docs/records/components-and-updates.md)
and [patterns](../../../docs/records/patterns.md) records; the
[AST contract](../ast-interface.md) owns the normalized shapes. Optional
values, `??`/`?`, sums and `match` are outside this slice, and semantic checks
remain graph's.

## Rule coverage

"Grammar" means the concrete grammar already decides the rule; "AST" means
normalization in this component; "graph" means a static check analysis owns.

| Rule | Current coverage | Slice plan |
| --- | --- | --- |
| `of` keys: name, integer position, `(row, column)` pair | AST: all three (`ComponentKey`) | Done; graph checks key kind, range and integer spelling |
| Pair with other than two positions | AST: `ComponentKeyPositions` | Done |
| Computed keys | Grammar: only literal keys stand before `of` | None |
| `of` precedence and right nesting | Grammar; tested | None |
| Wrong key kind, unknown or out-of-range component, `of` on non-fibers, subscript on a fiber, variant as component | Not syntax | Graph |
| `but` precedence, left grouping, line continuation | Grammar; tested | None |
| Override as any value, including partial products | AST: `Update`, `Product` | Done |
| Override key written twice | AST keeps duplicates | Graph diagnoses |
| Product, struct and array-of-product update laws | Not syntax | Graph |
| Removed spatial `base but key: value` | AST: `RemovedSpatialUpdate` at the colon | Done |
| Name and `_` patterns | AST | None |
| Bracket and bare-run patterns (two or more) | AST: `PatternKind::List`; a single bare name stays a name | None |
| Positional tuple `(p, …): F` | AST: `PatternKind::Site`; grammar refuses it untyped, at top level and in headers | None |
| Constructor `Head(p, …)` | AST: `PatternKind::Constructor` | Done |
| Constructor with an inline description head | AST: typed unsupported (`described_head`) | Inline product descriptions are outside this slice |
| Named components `(k: p, …)` with name or integer keys | AST: `PatternKind::Components` | Done |
| Braces `{ k, k2: p }` | AST: `PatternKind::Entries` | Done |
| Nesting of all forms | AST | Done |
| Positions: local, header binder, reduction driver, parameter, top-level `=` | AST for every form | Done |
| `state` takes a single name; reduction seed and accumulator take a single name | Grammar | None |
| Parameter default on a bare name only | Defaults are unsupported in this AST | None in this slice |
| Stated type on a pattern other than a name, discard or positional tuple | AST: `PatternTypeAnnotation` (lead ruling); a typed `_` is accepted | Done |
| Arity, unknown head/component/key, duplicate keys, name bound twice, named components over a struct | Not syntax | Graph |
| `p := e?` | Outside this slice | None |

Held for the owner: a top-level constructor pattern `P(a, b) = m` is
indistinguishable from the function equation `P(a, b) = m` in the grammar and
keeps the function-definition reading until the owner rules; the recommendation
is not to admit constructor patterns at top level.

## Checkpoints and verification

Checkpoint 1 (accepted): `of` keys, partial products, `but` and the removed
spatial form, with tests for `of` precedence and a partial-product update.
Checkpoint 2: constructor, named-component and brace patterns with spans in
every admitted position, and the stated-type rejection. Graph adopts the
breaking AST changes at its own pin.
