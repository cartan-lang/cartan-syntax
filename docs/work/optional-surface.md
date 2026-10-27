---
id: optional-surface
title: Normalize the optional-value surface
kind: work
status: active
implementation: implemented
topics: [syntax, interface, frontend]
owner: cartan-syntax-agent
depends: [members-and-patterns, structural-annotations]
authority: Owner released slice 2 of the workspace optional/members/patterns/sums work on 2026-10-27, assigned to syntax by the lead
---

# Normalize the optional-value surface

The syntax part of slice 2 of the workspace
[optional members, patterns and sums](../../../docs/work/optional-members-patterns-and-sums.md)
work, under the workspace [optional values](../../../docs/records/optional-values.md)
record. The [AST contract](../ast-interface.md#optional-values) owns the
normalized shapes. Sums and `match` are outside this slice, and presence,
typing and lifting checks remain graph's.

## Rule coverage

| Rule | Coverage |
| --- | --- |
| `T\|None` in annotations, parameters, `state` and signature slots | AST: `AnnotationKind::Optional { value, none_span }` |
| `T\|None` as a type argument, e.g. `List<Real\|None>` | AST, as above |
| A second alternative, `T\|None\|None` | Grammar refuses (`AnnotationAlternative`) |
| An alternative other than `None`, e.g. `Real\|Int` | AST: `UnionAlternative` at the alternative |
| `T\|None` as a struct entry, in aliases, inline and nested structs | AST: `StructEntry.annotation` is `Optional` |
| `T\|None` as a fiber component type | Grammar refuses: components take a bare fiber type |
| `None` as a type | AST: an ordinary named type |
| `none` | AST: `ExprKind::None` |
| `if c then a` without `else`; `else` binds to the nearest `if` | AST: `If { alternative: None }`; grammar decides the binding |
| `a ?? b` as one token, right-associative | AST: `ExprKind::Default`; grammar precedence and longest match |
| `??` looser than arithmetic, tighter than comparisons and Boolean words | Grammar; tested |
| A line beginning with `??` continues the expression | Grammar; tested |
| Postfix `x?` at an argument, subscript, component read, local value, collection and lockstep member | AST: `ExprKind::Lift { value, mark_span }` wherever it is written |
| A mark with no operation to lift through | Graph: the AST keeps every mark |
| Left operand that can never be absent, incompatible fallback, optional condition, unproven payload read, optional array fiber, optional Boolean operand | Graph |
| Matching on `T\|None` | Open in the record; outside this slice |

Optional struct fields, `{a: Real|None}`, are delivered together with graph's
optional-value work, under the workspace
[optional struct fields](../../../docs/records/optional-values.md#optional-struct-fields)
ruling. The entry is always present and its value may be absent; graph owns
presence and missing-key checks.

## Verification

Four new AST tests cover the recorded precedence cases (right association,
`m ?? n + 1`, `m ?? 0 > 3`, `a ?? b and c`, `a == b ?? c`, `not m ?? false`, line
continuation and token spans), `none` and else-less conditionals with nearest
binding, marks at each lifting position with mark spans, and optional
annotations with the `UnionAlternative` refusal. A fifth covers optional struct
entries in aliases, inline binding and parameter structs, nested structs and
over an inline sum, with whole and `None` spans. It also covers the
`UnionAlternative` refusal there, the grammar's refusal of a second
alternative, and fiber components still refusing an alternative.
