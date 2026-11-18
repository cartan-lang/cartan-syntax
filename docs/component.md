---
id: component
title: Syntax component interface
kind: reference
status: accepted
implementation: partial
topics: [syntax, interface, tooling]
---

# Syntax component interface

This component owns the source grammar, grammar parsing, source locations and
syntax-only tooling. It has no evaluator, runtime values, semantic type checker,
name resolver, graph constructor or typed computational intermediate representation.

## Parsing

`parse_document(&str)` reads the source-file grammar. `parse_expression(&str)`
reads an expression without changing its source offsets. Both return a
`SyntaxTree` with:

- `source`: owned UTF-8 source text.
- `root`: a concrete syntax tree node and its children.
- `diagnostics`: recoverable grammar diagnostics.

A concrete syntax tree (CST) retains written structure, including punctuation,
comments and grammar-specific nodes. Each node has a kind, optional field label,
named/extra flags, children and a half-open byte span. `SyntaxTree::text` returns
the source slice at a node's offsets when those offsets are within the text.
The helper does not establish that an arbitrary supplied node originated in
this particular tree.

`has_errors()` checks diagnostics and error nodes. A tree is returned for
malformed or incomplete input so an editor can inspect the remaining structure.
A document is read item by item, and an item that does not parse becomes one
`ERROR` node holding its one diagnostic, after which the next item is read, so
every well-formed item is in the tree and one bad line hides no other. The
failed item reaches at least to the end of the line its diagnostic stands on,
since the parse read the lines before it as that item, continuation lines
included; it ends at the first line end after that where every bracket it
opened is closed, or where the next line holding a token starts no further in
than the item and does not open with a closing bracket. Literals are read whole:
a quoted string left open ends at its line, and a triple-quoted string left open
runs to the end of the text. The cost of the rule is that a line the parse read
into a failed item, such as a line after a bracket left open, belongs to it,
and that a failed item whose bracketed lines start as far out as the item itself
is split at them, each piece reporting. Recovery is per document item: a second
bad line inside the same item, such as in the same block, is not reported until
the first is repaired.

## Diagnostics

`Diagnostic` preserves a byte `Span`, found text, whether input is incomplete,
`DiagnosticKind` and `unclosed`. A kind is an expected-token set, an unexpected token,
a stated grammar rule with its refusal flag and captured fields, or the nesting
limit below. A captured
field has a name and span; it is not a rendered error sentence. `unclosed` is
the innermost bracket open on the parse path at the furthest failure when its
closer was among the expected tokens: the opener's span, the opening and closing
text and the grammar node kind that opened it, such as `list`. The parser keeps
these brackets per sequence, so a remembered rule's match never leaks one, and
acceptance is unaffected. A rule's own stated diagnostic carries none.

Nesting has a limit. No tree is deeper than `MAX_DEPTH` nodes on its longest
path from the root to a token, the root and the token included
(`Node::height`); a binding `x = ((1))` takes four besides its parentheses.
Text nesting deeper is `DiagnosticKind::TooDeep { limit }` on every entry
point, before parsing can exhaust the stack. A node taller than the limit is
refused where it is built, the diagnostic spanning the first token below the
limit in text order; and the parser refuses to read a level of nesting past the
limit where it builds no node, as in a run of brackets that never closes, the
diagnostic standing where it went past. Each operand of a chain of a
left-grouping operator nests one level, as it does in the tree, so a long sum
reaches the limit too. The refusal is final for the item that holds it, which
becomes an `ERROR` node. The limit bounds the stack that parsing, normalization
and every recursive walk of a tree take, and is chosen to be safe on a 2 MB
thread in a debug build; the [depth tests](../tests/depth.rs) parse at it and
beyond it on such a thread. `highlight` keeps lexical colors at any depth; its type-name overlay
walks only the bounded recovered tree.

The owner requires diagnostics and errors to remain typed variants with payloads,
source spans and underlying causes until one top-level formatter in a central
location per standalone application. Lower layers must not generate diagnostic
sentences, collapse causes into strings or classify errors by wording. The public
`Rule` enumeration identifies grammar-specific failures; consumers inspect that
identity rather than diagnostic prose. Wording assertions belong only to formatter
tests. Other tests check structured identity, payload, location, cause and status.
External process streams remain opaque output, not a source for string classification.

The CST and AST APIs return these typed diagnostics and nothing more: no codes,
sentences or rendered reports. Wording them belongs to the application, which
words `DiagnosticKind`, `Rule`, `FormDiagnosticKind` and the literal and
selection errors at its own presentation boundary from their kinds, spans and
fields.

## Syntax tooling

This repository has no command-line tool; the workspace's one executable reads
syntax through this library. `grammar::grammar()` returns the shared grammar value. The same value drives:

- `grammar::print()`: Tree-sitter grammar JavaScript, which
  [the grammar example](../examples/grammar.rs) prints.
- `print_textmate()`: lexical highlighting grammar.
- `print_vim()` and `print_vim_ftdetect()`: Vim syntax and file detection.
- `print_sublime()`: Sublime syntax definitions.
- `print_emacs(builtin_names)`: Emacs source using caller-supplied builtin names.

The generic `treegram` package interprets and prints this grammar. Tooling does
not import a language builtin registry or require compiler execution. These
generators do not provide a language server or semantic completion features.

## Source formatting

[`format::format_document`](../src/format.rs) returns formatted text for a
document accepted by the source AST. It works from the concrete tree and
changes whitespace between tokens. It keeps existing line breaks,
including the final line break if present, and indents broken bracketed forms
by four spaces per open delimiter. Literal contents, including interpolation
text and holes, comments and whitespace inside continuation tokens are copied
as written. Adjacent top-level function definitions have exactly one blank line
between them, after any trailing comment and before the next leading comment.
Other blank-line placement and line wrapping remain as written.

A changed document is parsed again and returned only if every written token,
comment, field and tree node still matches the original besides byte offsets
and the number of document-level separator newlines.
Grammar and source-form failures return the original diagnostics. A spacing
change that cannot be verified returns `FormatError::ChangedSyntax` with a
span in the original text. The function takes borrowed source and never
modifies it. Tests cover declarations, annotations, Tuples, function equations,
multiline bodies, calls and comprehensions, along with idempotence, comment
placement and exact literal spelling.

## Prompt support

[`prompt`](../src/prompt.rs) supplies the facts an interactive prompt needs, as
data. `completeness(text)` reads the text as a document and as one expression:
`Complete` when either has no grammar diagnostic, `Incomplete` when neither
does but one stopped at the end of the text (the typed `incomplete` flag), and
`Invalid` otherwise. It is a grammar judgment; normalization errors appear when
the text is parsed with `ast::parse_document`.

`highlight(text)` returns non-overlapping byte spans with a `HighlightClass`
(`Keyword`, `Constant`, `Number`, `String`, `Comment`, `Operator`, `Punctuation`,
`Function`, `Type` or `Name`). Its lexical scan colors partially typed text: words, constants
and operators come from the grammar's lexical layer, the same sets the editor
grammars print, and only comments, strings and numbers are scanned by their
token shapes. Template holes color as part of their literal, and an unterminated
literal colors to its line's end. The reserved words are `KEYWORDS`.
Function heads are names immediately followed by optional primes and `(`, even
when the call is unfinished. Type names come from `fiber_type` nodes in the
recovered document tree, including qualified and nested annotations. This
distinguishes annotations from Map entries without name resolution. An item
that cannot yet parse retains its lexical colors.

An unfinished call is incomplete rather than refused: the call-head refusal
applies only to a complete argument list, and the positional-argument refusal
only where the call closes after it, so `f(1` and `f(1, 2` followed by another
line report the open `(`.

## Acceptance limits

Grammar parsing is one layer of syntax processing. The separate
[`ast` interface](ast-interface.md) normalizes a supported subset into an
abstract syntax tree (AST), the representation of source constructs used for
further analysis. It includes file/namespace use heads, selectors, aliases and
qualified value, call and type names without loading files or resolving names,
and documentation strings on the document and its declarations. It refuses
alias-plus-selector combinations and written comparison/range chains, and requires
value bodies to have exactly one result. The CST API still returns grammar
trees for those forms without performing the AST checks.

Consequently, `has_errors() == false` is not a claim of complete language
validity. There is also no promise that consumers can analyze or execute every
construct recognized by either API.

Tests pin precedence, typed versus untyped patterns, scalar versus list
binders, newline rules, byte spans, Unicode handling and diagnostic identity,
each on a few lines of text written for the one construct it tests. Whole
programs are the application's examples, which it parses through this library.

Inside a body, `i, j := [10, 2]` and `[i, j] := [10, 2]` both parse as one
`local` with two identifiers in its `pattern` field. Their CST kinds remain
`bare_list_pattern` and `list_pattern`, respectively; the AST normalizes both
to `PatternKind::List` while preserving the original spans. At document scope,
pattern bindings use `=`; both forms with `:=` report `Rule::PatternValue` at
the colon.
The [syntax tests](../tests/syntax.rs) verify these structures and byte spans.

JavaScript syntax of the generated Tree-sitter grammar is checked with Node.
Actual Tree-sitter parser generation and execution are untested because the
Tree-sitter command-line tool is unavailable in this environment. There is no full editor-conformance claim.

## Licensing

All packages are licensed under the MIT License; see [LICENSE](../LICENSE).
Manifests declare `MIT` and disable crates.io publication.

## Consumer integration

The compiler consumes the source AST, and the application consumes syntax
facts and typed diagnostics. Each owns its adapter and locked Git dependency,
following the [local dependency contract](ast-interface.md#local-dependency-delivery);
its `Cargo.lock` is the record of the revision it consumes. Name resolution,
overload selection, type checking and graph construction remain outside syntax. Integration and execution evidence belongs to the
consuming components.

## Current limitations

- Only `while` blocks normalize as update blocks; `every` blocks, namespace
  bodies, default arguments, function-header
  configuration and Tap bodies parse to a CST but are unsupported in the AST.
- Generated Tree-sitter JavaScript passes a syntax check; building and running a
  Tree-sitter parser is untested here.
