---
id: sum-fibers
title: Normalize sum rosters, constructors and match
kind: work
status: done
implementation: implemented
topics: [syntax, interface, frontend]
owner: cartan-syntax-agent
depends: [optional-surface, members-and-patterns]
authority: Owner adopted the workspace sum fiber record on 2026-10-27; slice 3 syntax assigned by the lead
evidence: src/ast.rs; src/ast/normalize.rs; tests/ast.rs; verification below
---

# Normalize sum rosters, constructors and match

The syntax part of slice 3 of the workspace
[optional members, patterns and sums](../../../docs/work/optional-members-patterns-and-sums.md)
work, under the workspace [sum fibers](../../../docs/records/sum-fibers.md)
record. The [AST contract](../ast-interface.md#sum-fibers) owns the normalized
shapes. Typing, identity, layout, exhaustiveness and demand belong to graph and
later stages.

The grammar already parsed rosters, constructors and `match` arms. This work
normalizes them and applies the adopted newline `else` rule. It is retained
while graph's sum slice needs its rule table and evidence.

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
| Line-leading `else` after an unbracketed arm-body conditional | AST: `LineLeadingArmElse` at the `else` |
| Same-line `else`, bracketed conditional, comma-ended arm | AST: ordinary `If` and `ElseArm` |
| Exhaustiveness, duplicate arms, arm outside the roster, parens disagreeing with the roster, non-sum or array scrutinee, arm result meet, selected-arm demand | Graph |
| Guards, literal patterns, alternatives, several scrutinees | Grammar refuses: no such arm form exists |
| `match` on `T\|None` with a `none` arm | Open in the record |

## Newline `else` in `match`

The workspace record adopts the refusal: inside a `match` arm list, a
conditional in an arm body (including the `else` arm's) that is not enclosed in
brackets must not take a line-leading `else`. The normalizer's arm-body step
walks the body's conditionals, skipping every child after an opening bracket
of its parent. It refuses a conditional whose `else` token carries its leading
newline with `LineLeadingArmElse { else_span, conditional_span }`, spanning the
keyword alone. The grammar, the editor grammar and conditionals outside `match`
are unchanged. Nested `match` arms are checked against their own arm list.

The rejected alternatives were the nearest-`if` reading, which silently drops
an intended `else` arm, and an arm-wins reading, which needs a second expression
grammar and gives the same text different meanings inside and outside `match`.

## Checkpoints

1. Rule table, newline `else` proposal, and tested roster and constructor
   normalization. Accepted.
2. `match` normalization: arms, payload patterns and `else` arm ordering.
   Accepted.
3. The adopted newline `else` rule. Done, awaiting review.

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

Checkpoint 3: two new AST tests cover:

- refusal with both spans for a conditional at the body's top, as an operand,
  nested in a consequence, in the `else` arm and in a nested match's arm;
- each fix: same-line `else`, parentheses, a braced body, a comma-ended arm,
  and a conditional inside call brackets;
- unchanged nearest-`if` continuation outside `match`.
