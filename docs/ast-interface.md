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
numerical workloads and the owner-authorized import and configurable source subsets. An abstract syntax tree (AST) represents written constructs
independently of concrete grammar production names. The component agents agreed
this interface under owner-authorized parser-transfer work; the supervisor
confirmed the scoped form rules on 2026-10-23. The owner authorizes the local Git
dependency delivery described below.
The existing [CST interface](component.md) remains available for editor tooling
and broader grammar inspection.

## Boundary

Syntax owns source parsing, normalization, source locations and syntax-form
errors. Graph owns configurable semantics and its source adapter. The existing
codegen frontend owns conversion to its analysis representation,
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

| Area | Supported source representation |
| --- | --- |
| Items | Imports, ordinary and `param` bindings, `state` declarations, fiber declarations, function equations, and guarded `while` update blocks. |
| Functions | Name, ordered parameters with patterns and optional annotations, and expression body. |
| Fibers | Name, written base annotation such as `Module`, and ordered named components with annotations. |
| Annotations | Named types and generic applications, preserving names and numeric arguments such as `Array<Primitive, 1>` without interpreting rank. |
| Patterns | Name, wildcard, recursive list pattern, and positional site pattern with its enclosing annotation retained. |
| Expressions | Literal number/boolean, local/qualified name, unary/binary operation, conditional, local/qualified call, indexing, `of` projection, list, named map, map join, configurable constructor/application, range, `periodic`, body, `for` comprehension and `reduce`. |
| Body | Ordered `BodyItem::Local` and `BodyItem::Result` entries, with exactly one result. A result need not be last. |
| Update block | Written guard and ordered writes, preserving target names, values and spans. |

Operators use enums, while names and number spellings remain strings.
Indexing retains its source expression and index expressions. A list of periodic
ranges remains a list containing periodic range expressions; recognizing a
domain belongs to analysis. Comprehensions retain their pattern, collection and
body. Reductions retain accumulator, seed, driver pattern, collection and body.
Support beyond the listed forms and interfaces below is not implied.

## Imports and qualified names

The owner-authorized [import AST work](work/use-source-ast.md) extends normalization
without changing the grammar. `Use` contains `source: UseSource`, optional
`selection: ImportSelection` and optional `alias: Name`. `UseSource::Namespace`
carries a written bare `Name`; `UseSource::File` carries `PathLiteral` with the raw
quoted spelling, decoded string and full quoted-literal span. `alias` records only
an explicitly written alias, never a synthesized file-stem binding. The item span
covers the complete declaration.

A selection is absent for `use ns` or `use "file.cart"`, distinct from `::*`.
`ImportSelection.span` covers `::` through the star, single name or braced list,
including intervening whitespace. `Imports::All` also carries the star's span;
`Imports::Names` carries each selected name and its span. `::name` and `::{name}`
normalize to the same name-list shape; their selector spans preserve the written
form. Explicit aliases retain their identifier spans. Syntax neither checks
whether a namespace exists nor reads the referenced file.

Quoted path decoding maps backslash followed by `n` to newline and by `t` to tab.
For any other escaped character it removes the backslash and retains that
character, including quote, backslash and Unicode characters. This is not JSON
escaping: a written backslash followed by `u0041` yields `u0041`, not `A`.
The raw quoted spelling and original byte span remain available alongside the
decoded string. Only use-path literals gain this normalization; general string
expressions remain unsupported. Path validity and filesystem behavior belong to
consumers.

`QualifiedName` retains its `namespace: Name`, ordered nonempty `members: Vec<Name>`
and full name span. Every segment retains its own byte span; no dotted/joined
string replaces that structure. Value references use `ExprKind::QualifiedName`;
ordinary value references remain `ExprKind::Name`. Calls use `NameRef::Local(Name)`
or `NameRef::Qualified(QualifiedName)` in `ExprKind::Call.function`. Declaration,
pattern and annotation names are unchanged; qualified annotations and qualified
function definitions remain outside this subset.

An explicit alias combined with any selector produces
`FormDiagnosticKind::UseAliasAndImports`, with the complete use-declaration span.
The CST grammar permits the combination; this AST source-form check rejects it.
`with`/hole clauses remain typed unsupported constructs rather than being dropped.
Namespace declarations and functors are not added. File selectors retain the file
head and absence of explicit alias so graph analysis can apply its adopted default
namespace binding; this AST does not synthesize bindings, resolve names or decide
exports, collisions, cycles or builtin availability.

The revised `Use` fields and call head type require deliberate consumer adapter
updates. Both graph and codegen owners accepted this exact seam before edits;
they own their adapters and locked dependency revisions. Accepting a form in
syntax does not promise execution support in either consumer.

## Maps and configurables

The [configurable source work](work/configurable-source-ast.md) supplies the
interface agreed with graph and the implementation coordinator. Ordinary named
map literals use `ExprKind::Map(Vec<MapEntry>)`; `{}` is an empty map, while
`{value}` remains a body. Each entry has `key: Name`, `value: Expr` and its full
entry `span`. The outer expression span includes braces. Entry order, duplicate
keys, individual key/value spans and nested supported expressions are preserved.
Computed keys and map walks remain explicit unsupported forms in normalization;
this is a bounded named-map subset, not all statically enumerable maps.

`a <> b` uses `ExprKind::MapJoin { left, right, operator_span }`. It retains the
existing additive precedence and left associativity: `a <> b <> c` means
`(a <> b) <> c`, and `a <> b + c` means `(a <> b) + c`. Syntax does not validate
operand types or implement strict joins, key compatibility or right bias.

`:base :over {body}` uses `ExprKind::Configurable { settings, body }`.
The nonempty ordered `Vec<SettingsAtom>` retains each atom's `value: Expr` and
introducing colon `operator_span`, including the leading colon. The `Body`
retains its own braces and ordered locals plus exactly one result, as for an
ordinary body. Single-atom forms include `:{c: 1} {c}`, `:base {c}` and
`:(base <> over) {c}`. Each atom uses the existing configuration-atom grammar:
named map, bare/qualified name, named call, index or parenthesized expression.
Joins inside atoms require parentheses. Each repeated atom has its own colon;
there is no implicit synthetic join node or synthetic operator span.

`value:settings` and `value.{key: expression}` use
`ExprKind::Configure { kind, target, settings, operator_span }`, where `kind` is
`ConfigureKind::Colon` or `ConfigureKind::Dot`. Both chain left at postfix
precedence, above arithmetic. Calls and indices within a colon atom bind to that
atom: `value:make(x):choices[i]` has two assignment links. A following literal-dot
link applies to the configured target. Dot accepts only a literal map in this
slice. Named-dot, parenthesized-dot and trailing-dot forms remain unsupported;
no source spelling for named closing is inferred. The `operator_span` fields
cover only the written `:`, `.` or `<>`, excluding continuation whitespace.
Other expression spans retain the complete written form and grouping.

These forms preserve source only. Graph owns setting/default scopes, type
inference and fitting, duplicate/unknown-key diagnostics, joins, live substitution,
interface closing, configurable consumption and the prohibition on configurable
values nested inside settings. Syntax preserves such nested expressions when
otherwise supported so graph can diagnose them. The retired function-header
configuration `f(x):{c:1}=body` and positional parameter defaults remain
unsupported normalized forms. No execution support follows from AST acceptance.

The lead verified the accepted constructor chain and literal closing against
legacy `docs/rulings/configurable-syntax.md` option 4 and
`docs/records/config-map.md` at `a2916b3`, and recorded the bounded contract in
[the governing workspace work item](../../docs/work/compile-language-integration.md).
Syntax adopts the written forms and source provenance, not legacy evaluation or
expansion machinery. The syntax owner did not consult or execute legacy.

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
chain, body result count, and a use alias combined with selectors. `Error::span()` returns the full byte range; errors
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

Only named types and generic applications, named projections, the imports above,
and the listed expression forms are normalized. Default arguments,
function-header configuration/lock clauses, sum fibers, qualified annotations,
general `fold`, guard/configuration clauses on reductions and additional pattern forms
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

For imports, the lead supplied and checked the same legacy revision's
`crates/cartan/src/parser.rs` lines 552–596 and test at 2141 for alias-plus-selector
refusal, and `crates/cartan/src/lexer.rs` lines 72–94 and test at 182 for exact
string decoding. Specification section 2.1 states backslash escaping; the lexer
provides the precise rule above. The owner-authorized import prerequisite adopts
these source-form and decoding rules to avoid divergent syntax/loader behavior.
It does not import legacy module execution, functors or a builtin catalogue.
The syntax owner did not read or execute legacy material.

## Local dependency delivery

The owner accepts absolute local Git URLs and named branches for this workspace,
as recorded in the master's [dependency policy](../../docs/records/local-git-dependencies.md).
The current agreed AST is supplied on syntax's existing `main` branch:

```toml
cartan-syntax = { git = "file:///Users/jzrake/Work/Codes/cartan-lang/cartan-syntax", branch = "main" }
```

Codegen owns this frontend dependency declaration and its `Cargo.lock`, which
records the consumed commit. Cargo's cached committed source includes the local
`support/treegram` package. Supplier working-tree edits and branch switches do
not change a consumer's locked revision; updating that revision is a deliberate
consumer operation after a tested supplier commit is available.

Syntax owns interface changes, commits and any requested specialized ordinary
Git branch. Consumer-specific changes come as requests to this component; the
consumer selects the agreed branch and records its consumed revision and checks.
Dependency caches are not editing locations. No manual vendoring, working-tree
path dependency, agent worktree, remote or publication is needed for this setup.

A fresh dependency cache needs access to the supplier Git repository at the
configured location. The owner accepts this machine-specific address for local
development; portable public distribution remains a separate decision. Syntax
itself still builds from its repository without an adjacent checkout.

## Verification and ownership

The [AST tests](../tests/ast.rs) normalize all five fixtures and check representative
structure, spans, body order, pattern normalization, grouping, precedence,
annotations, unsupported clauses and complete grammar-error preservation.
Existing [CST tests](../tests/syntax.rs) retain the grammar/editor checks.
Codegen verifies its adapter, parser removal and numerical behavior independently.

The syntax component owns implementation here; codegen owns the adapter and
consumer dependency declaration and lockfile in its repository.
