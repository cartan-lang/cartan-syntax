---
id: ast-interface
title: Workload source AST interface proposal
kind: design
status: draft
implementation: planned
topics: [syntax, interface, frontend]
---

# Workload source AST interface proposal

This proposal transfers parsing and syntax-form validation for the five current
numerical workloads to `cartan-syntax`. An abstract syntax tree (AST) represents
written constructs independently of concrete grammar production names. This
interface is awaiting agreement with the codegen component; it is not an
implemented or approved API. The existing [CST interface](component.md) remains
available for editor tooling and broader grammar inspection.

## Boundary

Syntax owns source parsing, normalization, source locations and syntax-form
errors. The codegen frontend owns conversion to its analysis representation,
name resolution, type checking, constants, domain interpretation, array
composition, and numerical lowering. The core codegen library remains independent
of syntax. In particular, the syntax AST contains neither inferred types nor
the frontend's inserted `Wrap` expressions, selected update target, resource
references or chosen computational units.

The current codegen frontend's source representation combines syntax and
analysis conveniences. The proposed AST preserves source declarations and
imports instead of assuming a document has exactly one `while true` update
and discarding `use std::*` or `main = 0`. Enforcing the supported execution
shape remains the adapter's responsibility.

## Proposed entry points and data

Add `ast::parse_document(&str) -> Result<ast::Document, Vec<ast::Error>>` and
`ast::parse_expression(&str) -> Result<ast::Expr, Vec<ast::Error>>`.
Both use the existing grammar parser and then normalize its CST; there is no
second lexer or expression parser. Grammar errors prevent AST construction.
The document owns its source text and ordered items. Expressions and all
declarations, patterns, names and annotations retain half-open UTF-8 byte spans
in the supplied source. Numeric literals retain their spelling; machine number
conversion and range restrictions belong to the consumer.

The concrete Rust fields and variant names require agreement before coding.
The proposed source constructs are:

| Area | Representation needed by the workloads |
| --- | --- |
| Items | Imports, ordinary and `param` bindings, `state` declarations, fiber declarations, function equations, and guarded `while` update blocks. |
| Functions | Name, ordered parameters with patterns and optional annotations, and expression body. |
| Fibers | Name, written base annotation such as `Module`, and ordered named components with annotations. |
| Annotations | Named types and generic applications, preserving names and numeric arguments such as `Array<Primitive, 1>` without interpreting rank. |
| Patterns | Name, wildcard, recursive list pattern, and positional site pattern with its enclosing annotation retained. |
| Expressions | Literal number/boolean, name, unary/binary operation, conditional, call, indexing, `of` projection, list, range, `periodic`, body, `for` comprehension and `reduce`. |
| Body | Ordered local bindings with patterns and optional annotations, followed by exactly one result expression. |
| Update block | Written guard and ordered writes, preserving target names, values and spans. |

Operators should use enums, while names and number spellings remain strings.
Indexing retains its source expression and index expressions. A list of periodic
ranges remains a list containing periodic range expressions; recognizing a
domain belongs to analysis. Comprehensions retain their pattern, collection and
body. Reductions retain accumulator, seed, driver pattern, collection and body.
Support beyond the forms needed by the workload fixtures is not implied.

## Normalization and diagnostics

Bare `i, j` and bracketed `[i, j]` patterns normalize to the same list-pattern
variant with the same ordered name structure. Their whole-pattern and child
spans remain different, matching the source. A scalar binder `i` stays a name;
it must not normalize to the singleton pattern `[i]`. Site patterns such as
`(rho, _, pre): Primitive` remain distinct from list patterns.

Grouping parentheses may normalize away after form validation, with the outer
expression span covering the grouping. The normalizer must check comparison
and range chains on the CST before removing grouping, so it can distinguish a
written chain from an explicitly parenthesized operand. The precise refusal
rules await supervisor-supplied legacy findings and component agreement.

Proposed structured errors distinguish `Grammar(Diagnostic)` from
`Form(FormDiagnostic)`. Form diagnostics carry a span and typed kind with
relevant payloads: unsupported construct and its CST kind, comparison/range
chain, missing or multiple body results, and a result followed by further body
items. Unsupported syntax is rejected explicitly; declarations, options and
clauses must never be silently dropped. This scoped AST does not imply complete
language validation, and CST acceptance alone remains weaker evidence.

The grammar's current precedence and newline rules are authoritative for this
transfer. The old codegen parser erased newlines, accepted unary `+`, and used
different power associativity. The adapter must not recreate those differences.
The Conway fixture already documents moving its continued subtraction operator
to the preceding line; codegen must account for that source difference when
integrating the shared parser.

## Standalone dependency delivery proposal

After syntax commits the agreed interface, codegen can vendor an explicit source
snapshot of this package and `support/treegram` under its own repository, with
relative path dependencies and the originating syntax commit recorded. Codegen
owns the destination paths, Cargo integration and provenance. This does not
require registry publication, a remote or an adjacent checkout. Syntax remains
the authoritative editing location; subsequent fixes are committed here before
refreshing the consumer snapshot. This delivery method requires codegen's
agreement and does not authorize edits in its checkout by the syntax agent.

## Acceptance and open coordination

- Agree Rust types, entry points, diagnostics, unsupported-form policy and
  snapshot delivery with the existing codegen component agent.
- Obtain the supervisor's focused legacy findings on syntax-form invariants
  and pitfalls; component agents do not consult the legacy checkout.
- Normalize all five syntax fixtures and assert representative AST structure,
  source spans, pattern equivalence, grouping and precedence.
- Reject malformed or unsupported inputs with structured facts and preserve
  existing CST behavior and editor grammar generation.
- Run syntax's documented checks and hand off a clean commit. Codegen then
  verifies the adapter, removes its provisional parser and checks its workload
  and numerical behavior independently.

No production Rust code is assigned to another agent in this repository. The
syntax agent owns implementation here; the codegen agent owns the adapter and
consumer dependency snapshot in `cartan-codegen`.
