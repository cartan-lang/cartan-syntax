---
id: ast-interface
title: Workload source AST interface
kind: design
status: accepted
implementation: implemented
topics: [syntax, interface, frontend]
---

# Workload source AST interface

`cartan-syntax` owns parsing and syntax-form validation for the five current
numerical workloads. An abstract syntax tree (AST) represents written constructs
independently of concrete grammar production names. The component agents agreed
this interface under owner-authorized parser-transfer work; the supervisor
confirmed the scoped form rules and standalone dependency delivery on 2026-10-23.
The existing [CST interface](component.md) remains available for editor tooling
and broader grammar inspection.

## Boundary

Syntax owns source parsing, normalization, source locations and syntax-form
errors. The codegen frontend owns conversion to its analysis representation,
name resolution, type checking, constants, domain interpretation, array
composition, and numerical lowering. The core codegen library remains independent
of syntax. In particular, the syntax AST contains neither inferred types nor
the frontend's inserted `Wrap` expressions, selected update target, resource
references or chosen computational units.

The codegen frontend's internal representation combines syntax and analysis
conveniences. The source AST preserves declarations and imports instead of
assuming a document has exactly one `while true` update
and discarding `use std::*` or `main = 0`. Enforcing the supported execution
shape remains the adapter's responsibility.

## Entry points and data

`ast::parse_document(&str) -> Result<ast::Document, Vec<ast::Error>>` and
`ast::parse_expression(&str) -> Result<ast::Expr, Vec<ast::Error>>`.
Both use the existing grammar parser and then normalize its CST; there is no
second lexer or expression parser. Grammar errors prevent AST construction.
The document owns its source text and ordered items. Expressions and all
declarations, patterns, names and annotations retain half-open UTF-8 byte spans
in the supplied source. Numeric literals retain their spelling; machine number
conversion and range restrictions belong to the consumer.

The [public Rust types](../src/ast.rs) are authoritative for fields and variants.
`Expr`, `Item` and `Pattern` carry `kind` and `span`; names carry text and a span.
The source constructs are:

| Area | Representation needed by the workloads |
| --- | --- |
| Items | Imports, ordinary and `param` bindings, `state` declarations, fiber declarations, function equations, and guarded `while` update blocks. |
| Functions | Name, ordered parameters with patterns and optional annotations, and expression body. |
| Fibers | Name, written base annotation such as `Module`, and ordered named components with annotations. |
| Annotations | Named types and generic applications, preserving names and numeric arguments such as `Array<Primitive, 1>` without interpreting rank. |
| Patterns | Name, wildcard, recursive list pattern, and positional site pattern with its enclosing annotation retained. |
| Expressions | Literal number/boolean, name, unary/binary operation, conditional, call, indexing, `of` projection, list, range, `periodic`, body, `for` comprehension and `reduce`. |
| Body | Ordered `BodyItem::Local` and `BodyItem::Result` entries, with exactly one result. A result need not be last. |
| Update block | Written guard and ordered writes, preserving target names, values and spans. |

Operators use enums, while names and number spellings remain strings.
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

Grouping parentheses normalize away after form validation, with the outer
expression span covering the grouping. A comparison refuses an immediately
nested ungrouped comparison on its left; a range refuses an ungrouped range
on either side. Checks read the CST before removing grouping. Thus `a < b == c`
and `a..b..c` fail, while `(a < b) == c` and `a..(b..c)` remain syntax trees
whose semantic validity is the consumer's responsibility.

Structured errors distinguish `Grammar(Diagnostic)` from
`Form(FormDiagnostic)`. Form diagnostics carry a span and typed kind with
relevant payloads: unsupported construct and its CST kind, comparison/range
chain, and body result count. `Error::span()` returns the full byte range; errors
derive `Clone`, `Debug`, `PartialEq` and `Eq`. Grammar diagnostics are preserved
without rewriting. Normalization reports the first form error in each document
item; it returns no partial AST if any error occurs. Unsupported syntax is
rejected explicitly; declarations, options and clauses must never be silently
dropped. This scoped AST does not imply complete
language validation, and CST acceptance alone remains weaker evidence.

The grammar's current precedence and newline rules are authoritative for this
transfer. The old codegen parser erased newlines, accepted unary `+`, and used
different power associativity. The adapter must not recreate those differences.
The Conway fixture already documents moving its continued subtraction operator
to the preceding line; codegen must account for that source difference when
integrating the shared parser.

The value body preserves local/result order, including a local after the result.
The adapter must explicitly reject orders it cannot represent, rather than
misclassify them as invalid source syntax. Update bodies currently support
ordered writes only; other update-body forms are outside this AST subset.

Only named types and generic applications, named projections, namespace imports,
and the listed expression forms are normalized. File imports, aliases, default
arguments, configuration/lock clauses, sum fibers, qualified names, general
`fold`, guard/configuration clauses on reductions and additional pattern forms
return unsupported errors. Broader syntax restrictions such as periodic operands
or annotation-placement rules are not comprehensively validated. Names, type
existence, arity, domain/rank restrictions and numeric representability remain
analysis concerns.

## Adopted source contracts

The supervisor supplied read-only findings from legacy revision
`a2916b3e93d8a28561f34b31f5208ceffa9fea9e`, `crates/cartan/src/parser.rs`: pattern normalization at
line 1474, comparison chains at 1870, range chains at 1883, and body handling at
1155/1189. Referenced tests are at 2317 and 3497. The supervisor inspected that
evidence without executing the legacy tests; the syntax agent did not access
the legacy checkout. These findings justify preserving pattern distinctions,
checking written grouping, and requiring one result without requiring it last.
They do not import legacy evaluation, semantic analysis or full-language duties.
The local normalizer is a new implementation over this component's CST.

## Standalone dependency delivery

Codegen owns an explicit source snapshot of this package and `support/treegram`
under its own repository, with
relative path dependencies and the originating syntax commit recorded. Codegen
owns the destination paths, Cargo integration and provenance. This does not
require registry publication, a remote or an adjacent checkout. Syntax remains
the authoritative editing location; subsequent fixes are committed here before
refreshing the consumer snapshot. The agreed delivery method does not authorize
edits in codegen's checkout by the syntax agent. A snapshot's actual commit and
integration evidence are recorded by its consumer.

## Verification and ownership

The [AST tests](../tests/ast.rs) normalize all five fixtures and check representative
structure, spans, body order, pattern normalization, grouping, precedence,
annotations, unsupported clauses and complete grammar-error preservation.
Existing [CST tests](../tests/syntax.rs) retain the grammar/editor checks.
Codegen verifies its adapter, parser removal and numerical behavior independently.

The syntax component owns implementation here; codegen owns the adapter and
consumer dependency snapshot in its repository.
