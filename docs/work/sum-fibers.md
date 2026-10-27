---
id: sum-fibers
title: Normalize sum rosters, constructors and match
kind: work
status: active
implementation: partial
topics: [syntax, interface, frontend]
owner: cartan-syntax-agent
depends: [optional-surface, members-and-patterns]
authority: Owner adopted the workspace sum fiber record on 2026-10-27; slice 3 syntax assigned by the lead
---

# Normalize sum rosters, constructors and match

The syntax part of slice 3 of the workspace
[optional members, patterns and sums](../../../docs/work/optional-members-patterns-and-sums.md)
work, under the workspace [sum fibers](../../../docs/records/sum-fibers.md)
record. The [AST contract](../ast-interface.md#sum-fibers) owns the normalized
shapes. Typing, identity, layout, exhaustiveness and demand belong to graph and
later stages.

The grammar already parsed rosters, constructors and `match` arms. This work
normalizes them and settles the newline `else` ambiguity before `match` is
normalized.

## Rule coverage

| Rule | Coverage |
| --- | --- |
| `fiber S = (A: T \| B)`: payload and unit variants, mixed, declared order | AST: `ItemKind::Sum`, `Variant { name, payload, span }` |
| A roster needs at least two variants; mixing `,` and `\|` | Grammar refuses |
| A repeated variant name | Graph: the AST keeps duplicates, as for struct entries |
| A sum takes no algebra | AST: `SumAlgebra` at the word |
| Width parameter `fiber S<k> = (…)` | Unsupported construct, as for products |
| Inline roster wherever a fiber type stands, payloads nesting sums | AST: `AnnotationKind::Sum` |
| `S\|None` is an optional sum; no `none` variant | AST: `Optional` around `Sum` or a name; graph owns the rest |
| Alias identity is structural and ordered | Graph |
| `Shape(A: 1.0)`, qualified head, tight colon | AST: `Construct { head: Named, variant, payload }` |
| `(A: Real \| B)(A: 1.0)`, inline head | AST: `Construct { head: Inline, … }` |
| `(A: Real \| B)(B)`, inline head, unit variant | AST: `Construct { payload: None }` |
| `Mode(Binary)`, `a: Shape = (B: …)`, `m: Mode = Binary`, `f(Binary)` | Graph: AST `Call`, `Product`, `Name`; only resolution and the annotation choose a variant |
| A constructor naming other than one variant | AST: `ConstructorVariantCount { found }` |
| A bare payload never chooses a variant (inline head) | AST: `ConstructorWithoutVariant` |
| A bare payload under a named head, `Shape(1.0)` | Graph: the AST is a `Call` |
| Variant outside the roster; payload fitting and promotion; array construction | Graph |
| An inline product head with a variant argument, `(A: Real)(A: 1)` | Unsupported construct, as for inline product heads |
| `match s { … }` arms, comma or newline separated | AST: `ExprKind::Match`, arms in source order |
| `A(x)`, `A(_)`, `B(X: x, Y: y)` as `B((X: x, Y: y))`, unit `Unit` | AST: `MatchArm.payload`, `Components` for the shorthand |
| Positional `B(x, y)` without inner parens | Grammar refuses |
| A final `else` arm, at most once | AST: `ElseArm`; `DuplicateElseArm`, `ArmAfterElse` |
| Newline `else` after an arm body's conditional | Pending the proposal below; currently continues the conditional |
| Exhaustiveness, duplicate arms, arm outside the roster, parens disagreeing with the roster, non-sum or array scrutinee, arm result meet, selected-arm demand | Graph |
| Guards, literal patterns, alternatives, several scrutinees | Grammar refuses: no such arm form exists |
| `match` on `T\|None` with a `none` arm | Open in the record |

## Proposal: newline `else` in `match`

`else` is a continuation word, so a line beginning with `else` continues a
conditional, and an `if` without `else` is valid. In

```text
match s {
  A(x) then if x > 0 then 1
  else 0
}
```

the last line may be the `if`'s alternative or the `match`'s `else` arm. The
parser currently reads it as the alternative, binding to the nearest `if` as
everywhere else, which silently removes the `else` arm.

Proposed rule: **inside a `match` arm list, a conditional in an arm body that
is not enclosed in brackets must not take a line-leading `else`.** The source
is refused with a typed form diagnostic spanning that `else`. The writer
disambiguates by:

- writing the alternative on the consequence's line: `then 1 else 2`;
- enclosing the conditional in parentheses or a braced body, where the
  ordinary continuation rule applies; or
- ending the arm with a comma before a `match` `else` arm on the next line.

Rationale: neither reading is silently chosen, the check is local to
normalization and needs no grammar change, and the editor grammar remains
unchanged. Either reading can later be adopted without changing the meaning of
any accepted program.

Alternatives considered:

- *Nearest `if` (status quo).* Consistent with conditionals elsewhere, but an
  intended `else` arm silently becomes the inner `if`'s alternative. That is
  only caught by exhaustiveness when variants are missing.
- *The `match` arm wins.* A line-leading `else` in an arm list always opens the
  `else` arm. This matches the layout most writers intend, but needs a second
  copy of the expression grammar for arm bodies. It also gives the same text
  different meanings inside and outside `match`.

The adopted rule applies in the normalizer's single arm-body step, so either
the refusal or a grammar change for the arm-wins reading slots in there.

## Checkpoints

1. Rule table, newline `else` proposal, and tested roster and constructor
   normalization. Accepted.
2. `match` normalization: arms, payload patterns and `else` arm ordering.
   Done, awaiting review.
3. The adopted newline `else` rule, once ruled.

## Verification

Checkpoint 1: four new AST tests cover:

- roster order, unit, mixed, duplicate and nested variants with spans;
- inline rosters in binding, parameter, type-argument, optional, struct-entry
  and fiber-component positions;
- named, qualified and inline constructor heads, and unit inline constructors;
- the `Call`, `Product` and `Name` forms left to graph;
- each typed form error, with its span.

The unsupported-construct test now covers inline product heads with a variant
argument.

Checkpoint 2: two new AST tests cover:

- every arm form with spans, including shorthand and explicit components,
  repeated variants, nested and newline-separated arms;
- same-line conditional `else` and a comma before the `else` arm;
- empty and else-only matches;
- `DuplicateElseArm` and `ArmAfterElse` with both spans.
