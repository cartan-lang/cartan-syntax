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
            match &error.kind {
                DiagnosticKind::Expected { expected } => {
                    report.code = "syntax.expected";
                    report.message = if expected.is_empty() {
                        "expected an expression".into()
                    } else {
                        format!("expected {}", expected.join(" or "))
                    };
                    report.fields = json!({"expected":expected});
                }
                DiagnosticKind::Unexpected => {
                    report.code = "syntax.unexpected";
                    report.message = "unexpected input".into();
                }
                DiagnosticKind::Stated {
                    rule,
                    fields,
                    refused,
                } => {
                    let (code, message) = rule_message(*rule);
                    report.code = code;
                    report.message = message.into();
                    report.fields = json!({"refused":refused,"captures":fields.iter().map(|field|json!({"name":field.name,"span":{"start":field.span.start,"end":field.span.end}})).collect::<Vec<_>>()});
                }
            }
            report.fields["found"] = json!(error.found);
            report.fields["incomplete"] = json!(error.incomplete);
            if error.incomplete {
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
                        format!("`{kind}` is outside the supported syntax subset"),
                    )
                }
                UseAliasAndImports => (
                    "syntax.use_alias_imports",
                    "a use declaration cannot combine an alias and imported selectors".into(),
                ),
                ComparisonChain => (
                    "syntax.comparison_chain",
                    "write each comparison separately".into(),
                ),
                RangeChain => (
                    "syntax.range_chain",
                    "a range must have only two endpoints".into(),
                ),
                BodyResultCount { found } => {
                    report.fields = json!({"found":found});
                    (
                        "syntax.body_result_count",
                        format!("a body requires one result expression; found {found}"),
                    )
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
            "use `::` to select a namespace member",
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
