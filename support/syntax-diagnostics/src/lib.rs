//! Diagnostic reports for Cartan syntax errors, owned beside the parser.
//!
//! The parser returns typed errors and never invokes this crate. An application
//! calls [`syntax`] at its presentation boundary and renders the report with
//! `cartan_diagnostics`. Codes and fields are stable identities; messages are
//! wording for people and are never parsed.

use cartan_diagnostics::{Label, Report, SourceLocation, Span};
use cartan_syntax::{ast, DiagnosticKind, Rule};
use serde_json::json;

/// The report for one syntax error. `source` is the index of the erroneous
/// text in the caller's source snapshot table; spans stay exact byte offsets.
pub fn syntax(error: &ast::Error, source: usize) -> Report {
    let mut report = Report::error(
        "syntax.invalid",
        "invalid syntax",
        Some(location(source, error.span())),
    );
    match error {
        ast::Error::Grammar(error) => {
            let found = found_text(error.found.as_deref());
            match &error.kind {
                DiagnosticKind::Expected { expected } => {
                    report.code = "syntax.expected";
                    report.message = match expectation(expected) {
                        Some(expected) => format!("expected {expected}, found {found}"),
                        None => format!("unexpected {found}"),
                    };
                    report.fields = json!({"expected":expected});
                    if let Some(open) = &error.unclosed {
                        unclosed(&mut report, open, error.found.as_deref(), &found, source);
                    }
                }
                DiagnosticKind::Unexpected => {
                    report.code = "syntax.unexpected";
                    report.message = format!("unexpected {found}");
                }
                DiagnosticKind::Stated {
                    rule,
                    fields,
                    refused,
                } => {
                    let (code, message) = rule_message(*rule);
                    report.code = code;
                    report.message = match (rule, &error.found) {
                        (Rule::ItemEnd, Some(_)) => format!("expected end of line, found {found}"),
                        _ => message.into(),
                    };
                    if let Rule::NamespaceDot = rule {
                        report
                            .notes
                            .push("write `::` to select a namespace member".into());
                    }
                    report.fields = json!({"refused":refused,"captures":fields.iter().map(|field|json!({"name":field.name,"span":{"start":field.span.start,"end":field.span.end}})).collect::<Vec<_>>()});
                }
            }
            report.fields["found"] = json!(error.found);
            report.fields["incomplete"] = json!(error.incomplete);
            // An expected or unexpected headline already names the end of
            // the file; a stated rule's headline does not.
            let headline_names_end = error.found.is_none()
                && matches!(
                    error.kind,
                    DiagnosticKind::Expected { .. } | DiagnosticKind::Unexpected
                );
            if error.incomplete && !headline_names_end {
                report
                    .notes
                    .push("the source ends before this construct is complete".into());
            }
        }
        ast::Error::Form(error) => {
            use ast::FormDiagnosticKind::*;
            let (code, message) = match &error.kind {
                UnsupportedConstruct { kind } => {
                    report.fields = json!({"construct":kind});
                    (
                        "syntax.unsupported_construct",
                        format!("{} are not supported", construct_plural(kind)),
                    )
                }
                UseAliasAndImports => (
                    "syntax.use_alias_imports",
                    "a use declaration cannot combine an alias and imported selectors".into(),
                ),
                ComparisonChain => {
                    report
                        .notes
                        .push("join them with `and`, as in `a < b and b < c`".into());
                    (
                        "syntax.comparison_chain",
                        "comparisons cannot be chained".into(),
                    )
                }
                RangeChain => (
                    "syntax.range_chain",
                    "a range must have only two endpoints".into(),
                ),
                BodyResultCount { found } => {
                    report.fields = json!({"found":found});
                    let (message, note) = match found {
                        0 => (
                            "this body has no result expression".to_string(),
                            "end the body with the value it produces",
                        ),
                        found => (
                            format!("this body has {found} result expressions"),
                            "bind all but one of them with `:=`",
                        ),
                    };
                    report.notes.push(note.into());
                    ("syntax.body_result_count", message)
                }
                FormatPrecision { written } => {
                    report.fields = json!({"written":written});
                    (
                        "syntax.format_precision",
                        "format precision must be a non-negative digit count fitting u32".into(),
                    )
                }
                FormatNotation { written } => {
                    report.fields = json!({"written":written});
                    (
                        "syntax.format_notation",
                        "format notation must be `sci` or `eng`".into(),
                    )
                }
                SignatureOutsideParameter { .. } => (
                    "syntax.signature_position",
                    "a function signature is only allowed on a function parameter".into(),
                ),
                RemovedSpatialUpdate { .. } => (
                    "syntax.removed_spatial_update",
                    "spatial replacement with `but key: value` is no longer supported".into(),
                ),
                ComponentKeyPositions { found } => {
                    report.fields = json!({"found":found});
                    (
                        "syntax.component_key_positions",
                        format!("a component key requires two positions; found {found}"),
                    )
                }
                UnionAlternative => (
                    "syntax.union_alternative",
                    "only None may follow `|` in this type annotation".into(),
                ),
                PatternTypeAnnotation { .. } => (
                    "syntax.pattern_annotation",
                    "this pattern cannot carry a type annotation".into(),
                ),
                SumAlgebra => (
                    "syntax.sum_algebra",
                    "a Sum declaration cannot specify an algebra".into(),
                ),
                ConstructorVariantCount { found } => {
                    report.fields = json!({"found":found});
                    (
                        "syntax.constructor_variants",
                        format!("Sum construction requires one variant; found {found}"),
                    )
                }
                ConstructorWithoutVariant => (
                    "syntax.constructor_variant",
                    "Sum construction requires a named variant".into(),
                ),
                DuplicateElseArm { first_span } => {
                    report.related.push(Label {
                        location: location(source, *first_span),
                        message: "first else arm".into(),
                    });
                    (
                        "syntax.duplicate_else",
                        "match has more than one else arm".into(),
                    )
                }
                ArmAfterElse { else_span } => {
                    report.related.push(Label {
                        location: location(source, *else_span),
                        message: "else arm".into(),
                    });
                    (
                        "syntax.arm_after_else",
                        "variant arms must precede the else arm".into(),
                    )
                }
                LineLeadingArmElse {
                    conditional_span, ..
                } => {
                    report.related.push(Label {
                        location: location(source, *conditional_span),
                        message: "conditional begins here".into(),
                    });
                    (
                        "syntax.arm_else",
                        "bracket this conditional to disambiguate its else from the match arm"
                            .into(),
                    )
                }
            };
            report.code = code;
            report.message = message;
        }
    }
    report
}

fn rule_message(rule: Rule) -> (&'static str, &'static str) {
    use Rule::*;
    match rule {
        ComprehensionBody => ("syntax.comprehension_body", "expected a comprehension body"),
        MixedSeparators => (
            "syntax.mixed_separators",
            "do not mix `,` and `|` in a fiber",
        ),
        ComprehensionIn(_) => (
            "syntax.comprehension_in",
            "expected `in` after the comprehension binder",
        ),
        ItemEnd => ("syntax.item_end", "unexpected text after this item"),
        UseHead => (
            "syntax.use_head",
            "expected a file path or namespace after `use`",
        ),
        UseImports => ("syntax.use_imports", "expected imported names after `::`"),
        UseAlias => ("syntax.use_alias", "expected an alias name after `as`"),
        WithHoles => (
            "syntax.with_holes",
            "expected parenthesized arguments after `with`",
        ),
        HoleArgument => (
            "syntax.hole_argument",
            "expected `:` after the argument name",
        ),
        ModuleSpelling => (
            "syntax.module_spelling",
            "declare a code unit with `namespace`",
        ),
        NamespaceName => ("syntax.namespace_name", "expected a namespace name"),
        NamespaceHoles => (
            "syntax.namespace_holes",
            "expected parenthesized namespace parameters",
        ),
        NamespaceBrace => (
            "syntax.namespace_brace",
            "expected `{` before the namespace body",
        ),
        HoleFiber => (
            "syntax.hole_fiber",
            "expected `:` and a type for the namespace parameter",
        ),
        NamespaceItem => (
            "syntax.namespace_item",
            "this item is not allowed in a namespace body",
        ),
        PortValue => ("syntax.port_value", "expected `=` and an initial value"),
        LockCondition => (
            "syntax.lock_condition",
            "expected `when` and a lock condition",
        ),
        FiberDeclares => (
            "syntax.fiber_declares",
            "expected components or variants in the fiber declaration",
        ),
        StructFiber => (
            "syntax.struct_fiber",
            "fiber components cannot be written as a brace value",
        ),
        StructName => ("syntax.struct_name", "expected a Struct name"),
        StructEquals => ("syntax.struct_equals", "expected `=` after the Struct name"),
        StructEntries => ("syntax.struct_entries", "expected braced Struct entries"),
        StructEntryType => (
            "syntax.struct_entry_type",
            "expected `:` and a Struct field type",
        ),
        ComponentType => ("syntax.component_type", "expected `:` and a component type"),
        BindingValue | PatternValue => ("syntax.binding_value", "expected `=` and a binding value"),
        FunctionBody => ("syntax.function_body", "expected `=` and a function body"),
        AnnotationAlternative => (
            "syntax.annotation_alternative",
            "unexpected extra type alternative after `|`",
        ),
        UpdateBody => ("syntax.update_body", "expected a braced update body"),
        WhileGuard => ("syntax.while_guard", "expected a condition after `while`"),
        Then => ("syntax.then", "expected `then` after the condition"),
        NamespaceDot => (
            "syntax.namespace_dot",
            "`.` cannot select a namespace member",
        ),
        ComponentDot => ("syntax.component_dot", "use `of` to read a component"),
        CallHead => (
            "syntax.call_head",
            "a call must name a statically resolved function",
        ),
        PrimedCall => (
            "syntax.primed_call",
            "expected an argument list after the primed function name",
        ),
        PositionalArgument => (
            "syntax.positional_argument",
            "configuration keys do not belong in a positional argument list",
        ),
        ConfigAtom => (
            "syntax.config_atom",
            "expected a Map, name or selection in the configuration",
        ),
        FoldSeed => ("syntax.fold_seed", "expected `:=` and a recurrence seed"),
        FoldIn => ("syntax.fold_in", "expected `in` in the recurrence driver"),
        FoldBody => ("syntax.fold_body", "expected a braced recurrence body"),
        MatchArms => ("syntax.match_arms", "expected braced match arms"),
        ArmThen => ("syntax.arm_then", "expected `then` in the match arm"),
        HeaderParen => (
            "syntax.header_paren",
            "a header binder cannot use grouping parentheses",
        ),
        BracketParen => (
            "syntax.bracket_paren",
            "a bracket pattern position cannot use grouping parentheses",
        ),
        ParenComma => (
            "syntax.paren_comma",
            "a grouping expression cannot contain a comma",
        ),
        ListRepeat => (
            "syntax.list_repeat",
            "List repetition with `;` is not supported",
        ),
        TapBody => ("syntax.tap_body", "expected braced writes after `tap`"),
        WalkIn => ("syntax.walk_in", "expected `in` after the Map walk binders"),
        WalkBody => ("syntax.walk_body", "expected a Map entry for the walk body"),
        LocalParam => (
            "syntax.local_param",
            "parameters cannot be declared inside a body",
        ),
        BodyEquals => (
            "syntax.body_equals",
            "use `:=` for a local binding inside a body",
        ),
        ClauseComprehension => (
            "syntax.clause_comprehension",
            "clause comprehensions are no longer supported",
        ),
        ComponentWrite => (
            "syntax.component_write",
            "a state write must name the whole state",
        ),
        StateLocal => (
            "syntax.state_local",
            "expected `:=` for this local state binding",
        ),
        TemplateHole => (
            "syntax.template_hole",
            "expected an expression inside the template hole",
        ),
        TemplateClose => (
            "syntax.template_close",
            "expected `}` to close the template hole",
        ),
        RequireCondition => (
            "syntax.require_condition",
            "expected a parenthesized condition after `require`",
        ),
        RequireMessage => (
            "syntax.require_message",
            "a requirement message must be a string or tagged literal",
        ),
    }
}

fn location(source: usize, span: cartan_syntax::Span) -> SourceLocation {
    SourceLocation {
        source,
        span: Span {
            start: span.start,
            end: span.end,
        },
    }
}

/// The reports for text that is not one literal. Spans index the text the
/// caller registered as `source`; a syntax failure yields its own reports.
pub fn literal(error: &cartan_syntax::literal::LiteralError, source: usize) -> Vec<Report> {
    use cartan_syntax::literal::LiteralError::*;
    let (code, message, span, note, written) = match error {
        Syntax(errors) => return errors.iter().map(|error| syntax(error, source)).collect(),
        NotLiteral { span } => (
            "syntax.literal_expected",
            "expected a literal value".to_string(),
            span,
            "write a number, `true`, `false`, `none` or a quoted string",
            None,
        ),
        IntRange { written, span } => (
            "syntax.literal_int_range",
            format!("`{written}` is outside the Int range"),
            span,
            "Int values are signed 32-bit, from -2147483648 to 2147483647",
            Some(written),
        ),
        RealRange { written, span } => (
            "syntax.literal_real_range",
            format!("`{written}` is not a finite Real"),
            span,
            "Real values are 32-bit floating point",
            Some(written),
        ),
    };
    let mut report = Report::error(code, message, Some(location(source, *span)));
    report.notes.push(note.into());
    report.fields = json!({"written": written});
    vec![report]
}

/// The reports for text that is not one `NAME=VALUE` assignment. A value
/// error's spans index the whole assignment text registered as `source`.
pub fn assignment(error: &cartan_syntax::literal::AssignmentError, source: usize) -> Vec<Report> {
    use cartan_syntax::literal::AssignmentError::*;
    match error {
        MissingEquals { written } => {
            let mut report = Report::error(
                "syntax.assignment_equals",
                format!("`{written}` is not a NAME=VALUE assignment"),
                None,
            );
            report
                .notes
                .push("write a name, `=` and a literal value, as in `nx=64`".into());
            report.fields = json!({"written": written});
            vec![report]
        }
        InvalidName { name } => {
            let mut report = Report::error(
                "syntax.assignment_name",
                format!("`{name}` is not a name"),
                None,
            );
            report.fields = json!({"name": name});
            vec![report]
        }
        Value { error, .. } => literal(error, source),
    }
}

/// The report for a selector that names no single item of a valid document.
/// `source` indexes the document in the caller's source table.
pub fn selection(error: &cartan_syntax::selection::SelectionError, source: usize) -> Report {
    use cartan_syntax::selection::SelectionError::*;
    match error {
        UnknownBinding { name, available } => {
            let mut report = Report::error(
                "syntax.selection_unknown",
                format!("no top-level binding `{name}`"),
                None,
            );
            report.notes.push(if available.is_empty() {
                "this document has no top-level bindings".into()
            } else {
                const SHOWN: usize = 8;
                let mut names: Vec<String> = available
                    .iter()
                    .take(SHOWN)
                    .map(|name| format!("`{name}`"))
                    .collect();
                if available.len() > SHOWN {
                    names.push(format!("and {} more", available.len() - SHOWN));
                }
                format!("top-level bindings: {}", names.join(", "))
            });
            report.fields = json!({"selector": name, "kind": "binding", "available": available});
            report
        }
        UnknownUpdate { index, count } => {
            let mut report = Report::error(
                "syntax.selection_unknown",
                format!("no update block `update:{index}`"),
                None,
            );
            report.notes.push(match count {
                0 => "this document has no update blocks".into(),
                1 => "this document has one update block, `update:0`".into(),
                count => format!(
                    "this document has {count} update blocks, `update:0` to `update:{}`",
                    count - 1
                ),
            });
            report.fields = json!({"selector": format!("update:{index}"), "kind": "update",
                "index": index, "update_count": count});
            report
        }
        AmbiguousBinding { name, candidates } => {
            let mut report = Report::error(
                "syntax.selection_ambiguous",
                format!("`{name}` is bound by {} top-level items", candidates.len()),
                candidates.first().map(|span| location(source, *span)),
            );
            for span in candidates.iter().skip(1) {
                report.related.push(Label {
                    location: location(source, *span),
                    message: "also bound here".into(),
                });
            }
            report.fields = json!({"selector": name, "kind": "binding",
                "candidates": candidates.iter().map(|span| json!({"start": span.start, "end": span.end})).collect::<Vec<_>>()});
            report
        }
    }
}

/// The report for a selector spelled as neither a binding nor an update block.
pub fn selector(error: &cartan_syntax::selection::SelectorError) -> Report {
    let cartan_syntax::selection::SelectorError::Malformed { written } = error;
    let mut report = Report::error(
        "syntax.selection_malformed",
        format!("`{written}` is not a binding name or update block"),
        None,
    );
    report.notes.push(
        "select a top-level binding by name, or an update block as `update` or `update:N`".into(),
    );
    report.fields = json!({"written": written});
    report
}

/// Explain a bracket the parse left open. Where the text ends, or a
/// different closing bracket stands, the open bracket is certainly the
/// defect and the headline says so. Elsewhere the token found may instead
/// lack a separator before it, so the headline stays and the label points
/// at the opener; a line break between the two is not evidence either way.
fn unclosed(
    report: &mut Report,
    open: &cartan_syntax::Unclosed,
    found: Option<&str>,
    found_text: &str,
    source: usize,
) {
    let noun = match open.construct.as_deref() {
        Some("list") => "List".to_string(),
        Some("argument_list") => "call".to_string(),
        Some("map") => "Map".to_string(),
        Some("body") => "body".to_string(),
        Some("subscript") => "subscript".to_string(),
        Some("parenthesized_expression") => "parenthesized expression".to_string(),
        _ => format!("`{}`", open.open),
    };
    if found.is_none_or(|found| CLOSERS.contains(&found)) {
        report.code = "syntax.unclosed_delimiter";
        report.message = format!(
            "expected `{}` to close this {noun}, found {found_text}",
            open.close
        );
    }
    report.related.push(Label {
        location: location(source, open.span),
        message: format!("{noun} opened here"),
    });
    report.fields["unclosed"] = json!({
        "open": open.open,
        "close": open.close,
        "construct": open.construct,
        "span": {"start": open.span.start, "end": open.span.end},
    });
}

/// The grammar's closing brackets as written.
const CLOSERS: &[&str] = &[")", "]", "}"];

/// What the parser found where it stopped, as a headline names it.
fn found_text(found: Option<&str>) -> String {
    match found {
        None => "end of file".into(),
        Some(text) if text.starts_with(['\n', '\r']) => "end of line".into(),
        Some(text) => format!("`{}`", text.escape_debug()),
    }
}

/// The number literal's token: only an expression can begin with it.
const NUMBER: &str = "/[0-9][_0-9]*/";

/// Tokens that close or separate what came before, named exactly.
const CLOSING: &[&str] = &[
    "`)`", "`]`", "`}`", "`,`", "`then`", "`in`", "`else`", "`=`", "`:=`",
];

/// Tokens that can begin an expression.
const EXPRESSION_START: &[&str] = &[
    "identifier",
    NUMBER,
    "`(`",
    "`[`",
    "`{`",
    "`:`",
    "`\"`",
    "`\"\"\"`",
    "`f\"`",
    "`f\"\"\"`",
    "`true`",
    "`false`",
    "`none`",
    "`if`",
    "`not`",
    "`-`",
    "`+`",
    "`..`",
    "`..=`",
    "`texture`",
    "`periodic`",
    "`locked`",
    "`tap`",
    "`for`",
    "`each`",
    "`fold`",
    "`reduce`",
    "`match`",
];

/// Tokens that can begin a name or pattern.
const PATTERN_START: &[&str] = &["identifier", "`(`", "`[`", "`{`"];

/// Tokens that would continue the construct before them: an operator, a
/// postfix or a zone.
const CONTINUATION: &[&str] = &[
    "`of`",
    "`::`",
    "`..=`",
    "`..`",
    "`periodic`",
    "`or`",
    "`and`",
    "`==`",
    "`!=`",
    "`<=`",
    "`>=`",
    "`<`",
    "`>`",
    "`??`",
    "`+`",
    "`<>`",
    "`-`",
    "`*`",
    "`//`",
    "`/`",
    "`@`",
    "`^`",
    "`with`",
    "`but`",
    "`?`",
    "`(`",
    "`[`",
    "`:`",
    "`.`",
    "`{`",
];

/// **A short statement of what the parser expected**, or `None` where the
/// tokens it tried establish nothing reliable. The tokens are the union of
/// every alternative tried at one position, so the summary names closing
/// tokens exactly, says "an expression" only where a number could begin,
/// drops tokens that would merely continue what came before, and gives up on
/// any token it does not recognize. A newline is layout, never an expectation.
fn expectation(expected: &[String]) -> Option<String> {
    let tokens: Vec<&str> = expected
        .iter()
        .map(String::as_str)
        .filter(|token| *token != "newline")
        .collect();
    let value = tokens.contains(&NUMBER);
    // A quotation mark closes a string only where no value could begin.
    let closing = |token: &str| CLOSING.contains(&token) || (!value && token == "`\"`");
    let mut named: Vec<&str> = tokens.iter().copied().filter(|t| closing(t)).collect();
    let rest: Vec<&str> = tokens.iter().copied().filter(|t| !closing(t)).collect();
    if value {
        if !rest.iter().all(|token| EXPRESSION_START.contains(token)) {
            return None;
        }
        named.push("an expression");
    } else if rest.contains(&"identifier") {
        if !named.is_empty() || !rest.iter().all(|token| PATTERN_START.contains(token)) {
            return None;
        }
        named.push("a name or pattern");
    } else if named.is_empty()
        || !rest
            .iter()
            // A pattern other than a number's start reads the rest of a literal.
            .all(|token| CONTINUATION.contains(token) || token.starts_with('/'))
    {
        return None;
    }
    match named.as_slice() {
        [one] => Some(one.to_string()),
        [first, second] => Some(format!("{first} or {second}")),
        [first, second, third] => Some(format!("{first}, {second} or {third}")),
        _ => None,
    }
}

/// A CST construct kind as plural prose: `payload_expression` reads
/// "payload expressions".
fn construct_plural(kind: &str) -> String {
    let words = kind.trim_start_matches('_').replace('_', " ");
    if words.ends_with('s')
        || words.ends_with('x')
        || words.ends_with("ch")
        || words.ends_with("sh")
    {
        format!("{words}es")
    } else if let Some(stem) = words
        .strip_suffix('y')
        .filter(|stem| !stem.ends_with(['a', 'e', 'i', 'o', 'u']))
    {
        format!("{stem}ies")
    } else {
        format!("{words}s")
    }
}
