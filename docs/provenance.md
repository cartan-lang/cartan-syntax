---
id: provenance
title: Source provenance
kind: reference
status: accepted
implementation: not-applicable
topics: [syntax, provenance]
---

# Source provenance

The grammar and generic parser are derived from `counterplot` revision
`a2916b3e93d8a28561f34b31f5208ceffa9fea9e`. This attribution does not make that
repository a build dependency or import its project requirements.

| Local source | Extraction source at that revision |
| --- | --- |
| `src/grammar.rs` | `crates/cartan/src/grammar.rs` |
| `src/diagnostic.rs` rule identities | `crates/cartan/src/diag/parse/mod.rs` |
| `src/lexer.rs` keyword roster | `crates/cartan/src/lexer.rs` |
| `support/treegram/src/` | `crates/treegram/src/` |

The current grammar retains the source productions. Diagnostic identities are
syntax-local; the registry-independent Emacs generator receives builtin names
from its caller. Public diagnostics preserve exact rule identity and field
spans. Source documentation and generated headers describe this standalone
component. The generic pattern engine uses equivalent iterator loops, and
formatting follows rustfmt. Package license declarations retain MIT OR Apache-2.0.

Fixture inputs derive from the five `cartan-kernel-lab` examples at `bc11736`.
Conway's subtraction is formatted with the operator at the end of the preceding
line, as required by the established newline rule. The normalized fixture is
also used by the extracted codegen repository; its computation is unchanged.

Parser-library tests establish grammar/CST behavior and the bounded normalized
AST contract, not equivalence with the full legacy parser. The
[AST contract](ast-interface.md#adopted-source-contracts) records the supervisor's
source findings for adopted form rules. See the
[component limits](component.md#acceptance-limits).
