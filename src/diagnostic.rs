//! Grammar rule identities and structured source diagnostics.

use treegram::Label;

/// Half-open UTF-8 byte offsets in the source supplied to the parser.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

/// A matched grammar field captured by a stated diagnostic.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Field {
    pub name: String,
    pub span: Span,
}

/// The grammar's error facts, without a presentation sentence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DiagnosticKind {
    Expected {
        expected: Vec<String>,
    },
    Unexpected,
    Stated {
        rule: Rule,
        refused: bool,
        fields: Vec<Field>,
    },
}

/// A recoverable grammar diagnostic and its source location.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub span: Span,
    pub found: Option<String>,
    pub incomplete: bool,
    pub kind: DiagnosticKind,
}

impl Diagnostic {
    pub(crate) fn from_raw(raw: treegram::Diag) -> Self {
        let kind = match raw.said {
            treegram::Said::Expected(expected) if expected.is_empty() => DiagnosticKind::Unexpected,
            treegram::Said::Expected(expected) => DiagnosticKind::Expected { expected },
            treegram::Said::Stated {
                label,
                refused,
                fields,
            } => DiagnosticKind::Stated {
                rule: Rule::of(label).expect("the Cartan grammar uses its declared rule labels"),
                refused,
                fields: fields
                    .into_iter()
                    .map(|field| Field {
                        name: field.name,
                        span: Span {
                            start: field.start,
                            end: field.end,
                        },
                    })
                    .collect(),
            },
        };
        Self {
            span: Span {
                start: raw.pos,
                end: raw.end,
            },
            found: raw.found,
            incomplete: raw.incomplete,
            kind,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Comprehension {
    For,
    Each,
}

impl Comprehension {
    pub fn word(self) -> &'static str {
        match self {
            Comprehension::For => "for",
            Comprehension::Each => "each",
        }
    }
}

/// **A diagnostic the grammar states**, one per rule that states one:
/// the case [`treegram::Rule::Expect`] or [`treegram::Rule::Refuse`]
/// reports by its [`Label`]. Each retains its own typed rule identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Rule {
    /// A comprehension's body, braced or bare, missing.
    ComprehensionBody,
    /// A parenthesized fiber writing both `,` and `|`.
    MixedSeparators,
    /// A comprehension's header with no `in` after its binders.
    ComprehensionIn(Comprehension),
    /// More text on an item's line after the item.
    ItemEnd,
    /// A `use` with no path or namespace after it.
    UseHead,
    /// A `use … ::` with no names after it.
    UseImports,
    /// A `use … as` with no name after it.
    UseAlias,
    /// A `use … with` with no parenthesized holes after it.
    WithHoles,
    /// A `with` clause's hole with no `:` after its name.
    HoleArgument,
    /// A refused `module` item; code units are declared with `namespace`.
    ModuleSpelling,
    /// A namespace with no name.
    NamespaceName,
    /// A namespace with no parenthesized holes after its name.
    NamespaceHoles,
    /// A namespace with no brace after its holes.
    NamespaceBrace,
    /// A namespace's hole with no `:` and fiber.
    HoleFiber,
    /// A namespace body holding a `use`, a port or an update block.
    NamespaceItem,
    /// A port with no `=` and initial value.
    PortValue,
    /// A lock clause with no `when` and condition.
    LockCondition,
    /// A fiber declaration declaring no components or variants.
    FiberDeclares,
    /// Braces where a fiber's components stand.
    StructFiber,
    /// A struct declaration with no name.
    StructName,
    /// A struct declaration with no `=`.
    StructEquals,
    /// A struct declaration with no braced entries.
    StructEntries,
    /// A struct entry with no `:` and type.
    StructEntryType,
    /// A fiber's component with no `:` and type.
    ComponentType,
    /// A binding with no `=` and value.
    BindingValue,
    /// A top-level pattern with no `=` and value.
    PatternValue,
    /// A function definition with no `=` and body.
    FunctionBody,
    /// An annotation writing a second alternative after `|`.
    AnnotationAlternative,
    /// An update block with no braced writes.
    UpdateBody,
    /// A `while` block with no guard.
    WhileGuard,
    /// A conditional with no `then`.
    Then,
    /// A unary `+`, retired.
    UnaryPlus,
    /// A dot spelling a namespace step.
    NamespaceDot,
    /// A dot reading a value's component.
    ComponentDot,
    /// Parens applied to what is no named function.
    CallHead,
    /// A primed head with no argument list.
    PrimedCall,
    /// A configuration key written inside a call's parens.
    PositionalArgument,
    /// A configuration zone holding no Map, name or selection.
    ConfigAtom,
    /// A recurrence's accumulator with no `:=` and seed.
    FoldSeed,
    /// A recurrence's driver with no `in`.
    FoldIn,
    /// A recurrence with no braced body.
    FoldBody,
    /// A `match` with no brace of arms.
    MatchArms,
    /// A `match` arm with no `then`.
    ArmThen,
    /// A parenthesized binder in a header.
    HeaderParen,
    /// A parenthesized position of a bracket pattern.
    BracketParen,
    /// A comma inside grouping parens.
    ParenComma,
    /// A `;` after a List literal's entry.
    ListRepeat,
    /// A `tap` with no braced writes.
    TapBody,
    /// A Map literal's walk with no `in` after its binders.
    WalkIn,
    /// A Map literal's walk with no entry for its body.
    WalkBody,
    /// A `param` declared inside a body.
    LocalParam,
    /// A local declared with `=` inside a body.
    BodyEquals,
    /// A body opening with the retired clause comprehension's `in`.
    ClauseComprehension,
    /// A write naming a component of a port.
    ComponentWrite,
    /// A `state` local with no `:=`.
    StateLocal,
    /// A tagged literal's hole holding no expression.
    TemplateHole,
    /// A tagged literal's hole with no closing `}`.
    TemplateClose,
}

impl Rule {
    /// Every rule, each once: a rule's label is its position here.
    pub const ALL: &'static [Rule] = &[
        Rule::ComprehensionBody,
        Rule::MixedSeparators,
        Rule::ComprehensionIn(Comprehension::For),
        Rule::ComprehensionIn(Comprehension::Each),
        Rule::ItemEnd,
        Rule::UseHead,
        Rule::UseImports,
        Rule::UseAlias,
        Rule::WithHoles,
        Rule::HoleArgument,
        Rule::ModuleSpelling,
        Rule::NamespaceName,
        Rule::NamespaceHoles,
        Rule::NamespaceBrace,
        Rule::HoleFiber,
        Rule::NamespaceItem,
        Rule::PortValue,
        Rule::LockCondition,
        Rule::FiberDeclares,
        Rule::StructFiber,
        Rule::StructName,
        Rule::StructEquals,
        Rule::StructEntries,
        Rule::StructEntryType,
        Rule::ComponentType,
        Rule::BindingValue,
        Rule::PatternValue,
        Rule::FunctionBody,
        Rule::AnnotationAlternative,
        Rule::UpdateBody,
        Rule::WhileGuard,
        Rule::Then,
        Rule::UnaryPlus,
        Rule::NamespaceDot,
        Rule::ComponentDot,
        Rule::CallHead,
        Rule::PrimedCall,
        Rule::PositionalArgument,
        Rule::ConfigAtom,
        Rule::FoldSeed,
        Rule::FoldIn,
        Rule::FoldBody,
        Rule::MatchArms,
        Rule::ArmThen,
        Rule::HeaderParen,
        Rule::BracketParen,
        Rule::ParenComma,
        Rule::ListRepeat,
        Rule::TapBody,
        Rule::WalkIn,
        Rule::WalkBody,
        Rule::LocalParam,
        Rule::BodyEquals,
        Rule::ClauseComprehension,
        Rule::ComponentWrite,
        Rule::StateLocal,
        Rule::TemplateHole,
        Rule::TemplateClose,
    ];

    /// The label the grammar gives this rule: its position in
    /// [`Rule::ALL`].
    pub fn label(self) -> Label {
        let at = Rule::ALL
            .iter()
            .position(|r| *r == self)
            .expect("Rule::ALL lists every rule");
        Label(at as u32)
    }

    /// The rule a label names, where the grammar gave it one.
    pub fn of(label: Label) -> Option<Rule> {
        Rule::ALL.get(label.0 as usize).copied()
    }

    /// **The fields the rule's diagnostic reads**, by the grammar's
    /// field names, `found` being the text standing where it fired.
    pub fn reads(self) -> &'static [&'static str] {
        match self {
            Rule::ItemEnd
            | Rule::UseHead
            | Rule::UseImports
            | Rule::UseAlias
            | Rule::WithHoles
            | Rule::NamespaceName
            | Rule::StructName
            | Rule::Then
            | Rule::ConfigAtom
            | Rule::TemplateClose => &["found"],
            Rule::HoleArgument
            | Rule::NamespaceHoles
            | Rule::NamespaceBrace
            | Rule::HoleFiber
            | Rule::FiberDeclares
            | Rule::StructEquals
            | Rule::StructEntries
            | Rule::StructEntryType
            | Rule::ComponentType
            | Rule::BodyEquals
            | Rule::StateLocal => &["name"],
            Rule::PortValue => &["name", "kind"],
            Rule::BindingValue | Rule::FunctionBody => &["name", "found"],
            Rule::PrimedCall => &["function"],
            Rule::PositionalArgument => &["function", "key"],
            Rule::FoldSeed => &["keyword"],
            Rule::FoldBody => &["keyword", "found"],
            Rule::ArmThen => &["variant"],
            Rule::ComponentWrite => &["head", "tail"],
            Rule::ComprehensionBody
            | Rule::MixedSeparators
            | Rule::ComprehensionIn(_)
            | Rule::ModuleSpelling
            | Rule::NamespaceItem
            | Rule::LockCondition
            | Rule::StructFiber
            | Rule::PatternValue
            | Rule::AnnotationAlternative
            | Rule::UpdateBody
            | Rule::WhileGuard
            | Rule::UnaryPlus
            | Rule::NamespaceDot
            | Rule::ComponentDot
            | Rule::CallHead
            | Rule::FoldIn
            | Rule::MatchArms
            | Rule::HeaderParen
            | Rule::BracketParen
            | Rule::ParenComma
            | Rule::ListRepeat
            | Rule::TapBody
            | Rule::WalkIn
            | Rule::WalkBody
            | Rule::LocalParam
            | Rule::ClauseComprehension
            | Rule::TemplateHole => &[],
        }
    }
}
