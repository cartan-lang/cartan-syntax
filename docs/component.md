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

`has_errors()` checks diagnostics and error nodes. A tree can be returned for
malformed or incomplete input so an editor can inspect the remaining structure.

## Diagnostics

`Diagnostic` preserves a byte `Span`, found text, whether input is incomplete,
and `DiagnosticKind`. A kind is an expected-token set, an unexpected token, or
a stated grammar rule with its refusal flag and captured fields. A captured
field has a name and span; it is not a rendered error sentence.

The public `Rule` enumeration identifies grammar-specific failures. Display
wording belongs to a client; callers need not parse diagnostic prose to decide
which failure occurred.

## Syntax tooling

`grammar::grammar()` returns the shared grammar value. The same value drives:

- `grammar::print()`: Tree-sitter grammar JavaScript.
- `print_textmate()`: lexical highlighting grammar.
- `print_vim()` and `print_vim_ftdetect()`: Vim syntax and file detection.
- `print_sublime()`: Sublime syntax definitions.
- `print_emacs(builtin_names)`: Emacs source using caller-supplied builtin names.

The generic `treegram` package interprets and prints this grammar. Tooling does
not import a language builtin registry or require compiler execution. These
generators do not provide a language server or semantic completion features.

## Acceptance limits

Grammar parsing is one layer of syntax processing. This bootstrap does not yet
normalize an abstract syntax tree (AST), the representation of source constructs
used for further analysis. Additional form checks belong to that syntax layer:
for example, refusing comparison/range chains and requiring a body to have one
result. Some such forms produce a grammar tree without a diagnostic today.

Consequently, `has_errors() == false` is not a claim of complete language
validity. There is also no promise that the provisional codegen frontend accepts
every construct recognized here. It supports a much smaller numerical subset.

The five fixture programs parse and retain their expected loops and fields.
Tests also pin precedence, typed versus untyped patterns, scalar versus list
binders, newline rules, byte spans, Unicode handling and diagnostic identity.
The fixture copies are tests, not a second authoritative workload collection.

Inside a body, `i, j := [10, 2]` and `[i, j] := [10, 2]` both parse as one
`local` with two identifiers in its `pattern` field. Their CST kinds remain
`bare_list_pattern` and `list_pattern`, respectively; no normalized AST currently
establishes their equivalence. The grammar describes the bare form as the
bracketed form's shorthand. At document scope, pattern bindings use `=`;
both forms with `:=` report `Rule::PatternValue` at the colon.
The [syntax tests](../tests/syntax.rs) verify these structures and byte spans.

The generated Tree-sitter productions match the extracted source grammar's
output apart from provenance comments. JavaScript syntax is checked with Node.
Actual Tree-sitter parser generation and execution are untested because its CLI
is unavailable in this environment. There is no full editor-conformance claim.

## Next integration

Provide a normalized syntax interface and the missing form validation here.
Keep graph construction, type checking and source-to-computational-IR lowering
in the frontend. A codegen client should consume the resulting typed unit,
without acquiring a dependency on this syntax crate.
