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
errors. Graph owns configurable semantics and its source adapter; their semantic
implementation is deferred under the workspace's
[language scope](../../docs/records/language-scope.md). Retained configurable
syntax does not imply runtime function values, returned functions or dynamic
callees. Syntax normalizes function-signature parameter annotations; overload
resolution and static function-argument binding belong to graph.

Graph is the only direct consumer of this AST. It owns its source adapter, name
resolution, type checking, overload selection, constants, domain interpretation
and graph construction. Codegen consumes graph's typed computational
representation, does not depend on this crate, and owns array composition and
numerical lowering. The syntax AST therefore contains no inferred types,
selected update target, resource references or chosen computational units. It
preserves declarations and imports instead of assuming a document has exactly one
`while true` update or discarding `use std::*` or `main = 0`. Enforcing a
supported execution shape is the consumer's responsibility.

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
| Items | Imports, ordinary and `param` bindings, `state` declarations, fiber declarations, struct declarations, function equations, and guarded `while` update blocks. |
| Functions | Name, ordered parameters with patterns and optional value or function-signature annotations, and expression body. |
| Fibers | Name, written base annotation such as `Module`, and ordered named components with annotations. |
| Annotations | Named types, generic applications and inline struct types, preserving names and numeric arguments such as `Array<Primitive, 1>` without interpreting rank. |
| Patterns | Name, wildcard, recursive list pattern, and positional site pattern with its enclosing annotation retained. |
| Expressions | Literal number/boolean/string, tagged template, local/qualified name, unary/binary operation, conditional, local/qualified call, indexing, `of` projection, list, named map, map join, configurable constructor/application, range, `periodic`, body, `for` comprehension and `reduce`. |
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
decoded string. String expressions share this decoding but remain a distinct
`StringLiteral`; a path is never a string value. Path validity and filesystem
behavior belong to consumers.

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

The revised `Use` fields and call head type required deliberate consumer
adapter updates. When this seam was introduced, graph and codegen's provisional
frontend both accepted it before edits. Graph is now the sole consumer and owns
its adapter and locked revision. Accepting a form in syntax does not promise
execution support.

## Strings, templates and structs

The [string interpolation work](work/string-interpolation.md) supplies the
interface agreed with graph under the workspace's
[settled string semantics](../../docs/work/source-values-and-configurables.md#first-checkpoint-semantic-resolutions).
`ExprKind::String(StringLiteral { form, raw, value })` covers `"…"` and, in
expression position, `"""…"""`; `form` is `StringForm::Quoted` or `Triple`, `raw`
is the written spelling including delimiters, and the expression span covers it.
Quoted text uses the path decoding rule above. Triple-quoted text is raw, except
that one LF or CRLF immediately after the opening delimiter is dropped and the
longest common run of leading spaces/tabs of the non-blank lines is removed; a
blank line shorter than that indentation becomes empty. A standalone triple
string doc declaration is not an expression and keeps its typed unsupported error.

`ExprKind::Template(Template { form, pieces })` covers the tagged `f"…"` and
`f"""…"""` forms. Pieces stand in source order as `Piece::Text(TextPiece { raw,
value, span })` or `Piece::Hole(Hole { expr, format, span })`. Adjacent chunks and
escapes form one maximal text piece whose span and raw spelling cover what was
written; a piece whose decoded value is empty is omitted, so `f""` has no pieces
and adjacent holes have no text between them. `{{` and `}}` decode to braces in
both forms. Backslash escapes decode as quoted text in `f"…"`. In `f"""…"""`
a backslash is ordinary text, including before a hole or the closing delimiter,
and one or two quotation marks are text wherever three do not close the literal,
including directly before a hole or doubled brace. The raw form's newline and indentation removal applies
across its text pieces: lines continue through holes, and a hole counts as
non-blank content, so a line opening with a hole contributes no indentation.
Untagged literals keep braces as text.

A hole's span includes its braces and its `expr` is an ordinary expression,
resolved by consumers in the enclosing scope. `FormatSpec { precision, notation,
raw, span }` records the written format after the colon: `precision` is
`Precision { digits: u32, span }` for `.N`, and `notation` is `Notation::Sci` or
`Eng`. Its span includes the colon; `raw` excludes it. A point without a digit
count that fits `u32` yields `FormDiagnosticKind::FormatPrecision { written }`
spanning the point and digits. Remaining text other than `sci`/`eng` yields
`FormatNotation { written }` spanning that text. The precision range, which
values a format applies to, desugaring into joins, conversion and formatting
belong to graph, codegen and runtime. Syntax provides no runtime template parser.

`ItemKind::Struct(Struct { name, entries })` covers `struct P = {x: Real, …}`.
Each `StructEntry` retains `name`, `annotation` and the entry span in source
order, including duplicate keys; graph diagnoses duplicates and empty structs.
Annotations naming a struct use the existing `AnnotationKind::Named`, and a brace
value remains `ExprKind::Map`. A struct declaration is a structural alias, not a
nominal constructor or type identity; graph interprets an annotated brace value
structurally through the alias, never from AST spelling alone.

An inline struct type `{a: Real, b: Int}` written where an annotation stands is
`AnnotationKind::Struct(Vec<StructEntry>)`, using the same entries as a
declaration: source order, duplicate keys and an empty `{}` are preserved, and
entry types nest. The annotation span covers the braces. It is admitted wherever
a value annotation stands, including signature slots and results, struct entries,
fiber components and generic type arguments. Graph interns an inline struct like
a `struct` alias and decides where a struct is semantically eligible. The
[structural annotation work](work/structural-annotations.md) records this slice.
Inline product descriptions such as `(a: Real, b: Real)` remain an
`UnsupportedConstruct` of kind `component` in annotations.

## Function-signature parameters

The [static signature work](work/static-signatures.md) supplies the interface
agreed with graph; graph's [resolution table](../../cartan-graph/docs/work/static-dispatch.md)
owns overload semantics. `Parameter.annotation` is
`Option<ParameterAnnotation>`: `ParameterAnnotation::Value(Annotation)` for an
ordinary type, or `ParameterAnnotation::Signature(Signature { arguments, result,
arrow_span, span })` for `g: (T1, …, Tn) -> R`, including `() -> R` and
`(T) -> R`. One argument may also be written without parentheses, `g: T -> R`,
which yields the same `Signature` with one argument. Argument and result slots
are ordinary `Annotation`s: named types, generic applications and inline structs.
Optional and sum annotations remain unsupported, and `(x: Real) -> R` reads as an
inline product argument, which is unsupported; there are no named signature
formals. `span` covers what was written through the result,
`arrow_span` covers `->`, every slot keeps its own span, and the parameter span
covers the whole `g: (…) -> R`. Unannotated and value-annotated parameters are
unchanged.

A signature describes a statically named function a parameter receives. It is not
a function value. Syntax adds no overload marker, no result annotation on
definitions, and no checks of whether a formal must be annotated; overload sets,
matching, specificity, conformance and binding belong to graph.

A signature written anywhere else yields
`FormDiagnosticKind::SignatureOutsideParameter { position, signature_span }` with
the diagnostic span on its arrow. `SignaturePosition` is `Binding` (including
`param` and `state`), `Local`, `TypeArgument`, `SignatureArgument` or
`SignatureResult`. The last two reject signatures nested in another signature's
argument or result slot; a chained `T -> R -> S` therefore yields
`SignatureResult`. Struct entries and fiber
components take a bare fiber type in the grammar, so an arrow there is a
structured grammar error. Namespace declarations and `use` hole clauses keep
their unsupported-construct rejections.

The grammar tries a signature before the fiber-type reading inside a type
annotation, because fiber descriptions also open with a parenthesis and accept
`()`. The generated Tree-sitter grammar has not been generated or executed with
this alternative.

## Maps and configurables

The [configurable source work](work/configurable-source-ast.md) supplies the
interface agreed with graph. Ordinary named
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

The constructor chain and literal closing are the adopted configurable forms;
[the governing workspace work item](../../docs/work/compile-language-integration.md)
records their bounded contract. Syntax adopts only the written forms; evaluation
and expansion machinery are outside this component.

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
chain, body result count, a use alias combined with selectors, malformed
format precision or notation, and misplaced signatures. `Error::span()` returns the full byte range; errors
derive `Clone`, `Debug`, `PartialEq` and `Eq`. Grammar diagnostics are preserved
without rewriting. Normalization reports the first form error in each document
item; it returns no partial AST if any error occurs. Unsupported syntax is
rejected explicitly; declarations, options and clauses must never be silently
dropped. This scoped AST does not imply complete
language validation, and CST acceptance alone remains weaker evidence.

The grammar's current precedence and newline rules are authoritative for this
transfer. The parser it replaced in codegen erased newlines, accepted unary `+`
and used different power associativity; consumers must not recreate those
differences. The Conway fixture moves its continued subtraction operator to the
preceding line, as the newline rule requires.

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

These form rules are adopted design. List patterns stay distinct from site
patterns, written grouping is checked before parentheses are removed, and a body
requires exactly one result without requiring it to come last. The normalizer is
implemented over this component's CST; adopting these rules adds no evaluation,
semantic analysis or full-language obligations.

For imports, an alias combined with selectors is refused and quoted paths decode
with the exact backslash rule above, so syntax and module loading cannot diverge.
The import prerequisite adds no module execution, functors or builtin catalogue.

For strings and templates, the decoding, dedent, brace and format-spec rules
above are the adopted behavior. Every piece keeps spans and raw spelling,
malformed forms are typed errors rather than rendered refusals, text pieces are
never empty, and raw-form dedent follows lines across holes directly rather than
joining text around a separator character, so any character, U+0001 included,
may appear in text.

## Local dependency delivery

The owner accepts absolute local Git URLs and named branches for this workspace,
as recorded in the master's [dependency policy](../../docs/records/local-git-dependencies.md).
The current agreed AST is supplied on syntax's existing `main` branch:

```toml
cartan-syntax = { git = "file:///Users/jzrake/Work/Codes/cartan-lang/cartan-syntax", branch = "main" }
```

Graph owns this dependency declaration and its `Cargo.lock`, which records the
consumed commit. Cargo's cached committed source includes the local
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
Graph verifies its adapter and source-to-graph behavior independently.

The syntax component owns implementation here; graph owns the adapter and
consumer dependency declaration and lockfile in its repository.
