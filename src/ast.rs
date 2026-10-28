//! Source AST for the numerical workload subset, without semantic analysis.
//!
//! All spans are half-open UTF-8 byte offsets into the original input. Unsupported
//! grammar constructs return structured errors instead of being omitted.

mod normalize;
mod text;

use crate::{Diagnostic, Span};

/// A complete source document, preserving declaration order and source text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Document {
    pub source: String,
    pub items: Vec<Item>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Name {
    pub text: String,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    pub kind: ItemKind,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ItemKind {
    Use(Use),
    Binding(Binding),
    Fiber(Fiber),
    Sum(Sum),
    Struct(Struct),
    Function(Function),
    While(While),
}

/// A written import, without path lookup, name binding or export resolution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Use {
    pub source: UseSource,
    pub selection: Option<ImportSelection>,
    pub alias: Option<Name>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UseSource {
    Namespace(Name),
    File(PathLiteral),
}

/// A quoted file path; its span and raw spelling include both quotes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PathLiteral {
    pub raw: String,
    pub decoded: String,
    pub span: Span,
}

/// A selector's span includes `::` and its star, single name or braced list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportSelection {
    pub kind: Imports,
    pub span: Span,
}

/// An explicit namespace followed by one or more member names.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QualifiedName {
    pub namespace: Name,
    pub members: Vec<Name>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NameRef {
    Local(Name),
    Qualified(QualifiedName),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Imports {
    All { span: Span },
    Names(Vec<Name>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BindingKind {
    Ordinary,
    Parameter,
    State,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Binding {
    pub kind: BindingKind,
    pub pattern: Pattern,
    pub annotation: Option<Annotation>,
    pub value: Expr,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fiber {
    pub name: Name,
    pub algebra: Option<Name>,
    pub components: Vec<Component>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Component {
    pub name: Name,
    pub annotation: Annotation,
    pub span: Span,
}

/// `fiber Shape = (A: Real | B)`: a sum fiber's roster under an alias name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sum {
    pub name: Name,
    pub variants: Vec<Variant>,
}

/// One variant of a sum roster, in declared order. Duplicate names are kept;
/// graph owns roster checks. A unit variant has no payload.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Variant {
    pub name: Name,
    pub payload: Option<Annotation>,
    pub span: Span,
}

/// A named struct shape. Entries keep source order and duplicate keys; graph
/// owns duplicate, empty-struct and kind diagnostics.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Struct {
    pub name: Name,
    pub entries: Vec<StructEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StructEntry {
    pub name: Name,
    pub annotation: Annotation,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Function {
    pub name: Name,
    pub parameters: Vec<Parameter>,
    pub body: Expr,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Parameter {
    pub pattern: Pattern,
    pub annotation: Option<ParameterAnnotation>,
    pub span: Span,
}

/// A parameter's written type: an ordinary value type, or the signature of a
/// statically named function the parameter receives.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParameterAnnotation {
    Value(Annotation),
    Signature(Signature),
}

/// `(T1, …, Tn) -> R`, or `T -> R` for one argument; `span` covers what was
/// written, through the result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Signature {
    pub arguments: Vec<Annotation>,
    pub result: Annotation,
    pub arrow_span: Span,
    pub span: Span,
}

/// Where a function signature was written outside a parameter annotation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SignaturePosition {
    /// A document binding, including `param` and `state` declarations.
    Binding,
    /// A body local.
    Local,
    /// A generic type argument such as `List<(Real) -> Real>`.
    TypeArgument,
    /// An argument slot of another signature.
    SignatureArgument,
    /// The result slot of another signature.
    SignatureResult,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct While {
    pub guard: Expr,
    pub writes: Vec<Write>,
    pub body_span: Span,
}

/// `p <- e`, or the keyed `u[k, …] <- e` writing only the selected sites.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Write {
    pub target: Name,
    /// The subscript's selectors in order; empty for a whole-state write.
    pub keys: Vec<Expr>,
    pub value: Expr,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Annotation {
    pub kind: AnnotationKind,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnnotationKind {
    Named(Name),
    Apply {
        name: Name,
        arguments: Vec<TypeArgument>,
    },
    /// An inline struct type `{a: Real, b: Int}`, possibly empty; the
    /// annotation span covers the braces.
    Struct(Vec<StructEntry>),
    /// Fixed positional annotations, `[Int, Real]`, in source order.
    Tuple(Vec<Annotation>),
    /// `[T; width]`, preserving the width without expanding the entries.
    TupleRepeat {
        item: Box<Annotation>,
        width: TupleWidth,
    },
    /// An inline sum roster `(A: Real | B)`; the span covers the parentheses.
    Sum(Vec<Variant>),
    /// `T|None`, the only union the type language admits.
    Optional {
        value: Box<Annotation>,
        /// The `None` alternative.
        none_span: Span,
    },
}

/// A tuple repetition width as written; analysis resolves names and validates
/// numeric widths. A name `_` remains distinct from every numeric width.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TupleWidth {
    Number(NumberLiteral),
    Name(Name),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TypeArgument {
    Type(Annotation),
    Number { text: String, span: Span },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pattern {
    pub kind: PatternKind,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PatternKind {
    Name(Name),
    Wildcard,
    /// Both `a, b` and `[a, b]` normalize to this variant.
    List(Vec<Pattern>),
    /// Positional product destructuring, distinct from a list pattern.
    Site(Vec<Pattern>),
    /// `Head(p, …)`: a named product's components in declared order.
    Constructor {
        head: Name,
        items: Vec<Pattern>,
    },
    /// `(k: p, …)`: components by name or position, in any order.
    Components(Vec<ComponentPattern>),
    /// `{ k, k2: p }`: struct fields or Map entries by key.
    Entries(Vec<EntryPattern>),
}

/// One `k: p` of a named-components pattern; `k` is a name or position.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComponentPattern {
    pub key: ComponentKey,
    pub pattern: Pattern,
    pub span: Span,
}

/// One entry of a brace pattern. A bare key (`pattern: None`) binds the
/// key's own name; `k: p` reads key `k` into `p`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EntryPattern {
    pub key: Name,
    pub pattern: Option<Pattern>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExprKind {
    Number(String),
    Bool(bool),
    String(StringLiteral),
    Template(Template),
    Name(Name),
    QualifiedName(QualifiedName),
    Unary {
        operator: UnaryOperator,
        operand: Box<Expr>,
    },
    Binary {
        operator: BinaryOperator,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    /// `if c then a else b`; without `else` the alternative is absent.
    If {
        condition: Box<Expr>,
        consequence: Box<Expr>,
        alternative: Option<Box<Expr>>,
    },
    /// The literal `none`.
    None,
    /// `value ?? fallback`, the presence default.
    Default {
        value: Box<Expr>,
        fallback: Box<Expr>,
        /// The `??` token alone.
        operator_span: Span,
    },
    /// `value?`, the absence-lifting mark.
    Lift {
        value: Box<Expr>,
        /// The `?` alone.
        mark_span: Span,
    },
    Call {
        function: NameRef,
        arguments: Vec<Expr>,
    },
    /// A sum constructor naming its variant, `Shape(A: 1.0)` or
    /// `(A: Real | B)(B)`. A unit variant has no payload. A named head applied
    /// to a bare name, `Mode(Binary)`, stays a [`ExprKind::Call`], since only
    /// name resolution can tell it from a function call.
    Construct {
        head: ConstructorHead,
        variant: Name,
        payload: Option<Box<Expr>>,
    },
    Index {
        target: Box<Expr>,
        indices: Vec<Expr>,
    },
    /// `key of value`, reading one component.
    Projection {
        key: ComponentKey,
        value: Box<Expr>,
    },
    /// A product value or partial product `(D: 1.0, 0: x)`, entries in source
    /// order with duplicate keys kept for analysis to diagnose.
    Product(Vec<ProductEntry>),
    /// `base but overrides`: a new value with the named components replaced.
    Update {
        base: Box<Expr>,
        overrides: Box<Expr>,
        /// The `but` keyword alone.
        operator_span: Span,
    },
    List(Vec<Expr>),
    Map(Vec<MapEntry>),
    MapJoin {
        left: Box<Expr>,
        right: Box<Expr>,
        operator_span: Span,
    },
    Configurable {
        /// Nonempty, in source order; every atom has its own introducing colon.
        settings: Vec<SettingsAtom>,
        body: Body,
    },
    Configure {
        kind: ConfigureKind,
        target: Box<Expr>,
        settings: Box<Expr>,
        operator_span: Span,
    },
    Range {
        start: Option<Box<Expr>>,
        end: Option<Box<Expr>>,
        inclusive: bool,
    },
    Periodic(Box<Expr>),
    Body(Body),
    For(Box<Comprehension>),
    /// A value-producing ordered walk, `each x in xs { value }`.
    /// Collection eligibility and result element types belong to analysis.
    Each(Box<Comprehension>),
    Reduce(Box<Reduction>),
    Match(Box<Match>),
}

/// The written head of a sum constructor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConstructorHead {
    /// An alias name, possibly qualified.
    Named(NameRef),
    /// An inline roster; the span covers its parentheses.
    Inline { variants: Vec<Variant>, span: Span },
}

/// The literal key of a component read or product entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ComponentKey {
    /// A named component.
    Name(Name),
    /// A position, by its written spelling; analysis checks that it is an
    /// in-range integer.
    Position(NumberLiteral),
    /// `(row, column)` for a two-index fiber; the span covers the parentheses.
    Pair {
        row: NumberLiteral,
        column: NumberLiteral,
        span: Span,
    },
}

/// A numeric literal kept as written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NumberLiteral {
    pub text: String,
    pub span: Span,
}

/// One `key: value` entry of a product value or partial product.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProductEntry {
    pub key: ComponentKey,
    pub value: Expr,
    pub span: Span,
}

/// An ordinary named-map entry, preserving order and duplicate keys.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MapEntry {
    pub key: Name,
    pub value: Expr,
    pub span: Span,
}

/// Whether a string or template was written with one or three quotation marks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StringForm {
    Quoted,
    Triple,
}

/// A string expression. `raw` is the written spelling including delimiters and
/// `value` the decoded text. Import paths use [`PathLiteral`] instead.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StringLiteral {
    pub form: StringForm,
    pub raw: String,
    pub value: String,
}

/// A tagged `f"…"` or `f"""…"""` literal, without desugaring or formatting.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Template {
    pub form: StringForm,
    /// Source order. Adjacent text is one piece and no text piece is empty.
    pub pieces: Vec<Piece>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Piece {
    Text(TextPiece),
    Hole(Hole),
}

/// Text between holes; `span` and `raw` cover its written chunks and escapes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextPiece {
    pub raw: String,
    pub value: String,
    pub span: Span,
}

/// One `{expr}` or `{expr:format}` hole; `span` includes both braces.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hole {
    pub expr: Expr,
    pub format: Option<FormatSpec>,
    pub span: Span,
}

/// A hole's written format: digits after the point and notation. Consumers
/// validate the precision range and which values a format applies to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormatSpec {
    pub precision: Option<Precision>,
    pub notation: Option<Notation>,
    /// The spelling after the colon.
    pub raw: String,
    /// Includes the colon.
    pub span: Span,
}

/// `.3`; the span covers the point and its digits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Precision {
    pub digits: u32,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Notation {
    Sci,
    Eng,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingsAtom {
    pub value: Expr,
    /// The introducing colon alone, excluding continuation whitespace.
    pub operator_span: Span,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConfigureKind {
    Colon,
    Dot,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnaryOperator {
    Negate,
    Plus,
    Not,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    FloorDivide,
    Power,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    And,
    Or,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Body {
    /// Source order, with exactly one result. A result need not be last.
    pub items: Vec<BodyItem>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BodyItem {
    Local(Local),
    Result(Expr),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Local {
    pub pattern: Pattern,
    pub annotation: Option<Annotation>,
    pub value: Expr,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Comprehension {
    pub binder: Pattern,
    pub collection: Expr,
    pub body: Expr,
}

/// `match s { A(x) then e, Unit then e, else e }`. Arms keep source order and
/// duplicates; graph owns roster coverage and arm agreement.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Match {
    pub scrutinee: Expr,
    pub arms: Vec<MatchArm>,
    /// The final `else` arm, when written.
    pub otherwise: Option<ElseArm>,
}

/// One variant's arm. A bare unit arm has no payload pattern; `B(X: x, Y: y)`
/// has the same `PatternKind::Components` payload as `B((X: x, Y: y))`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatchArm {
    pub variant: Name,
    pub payload: Option<Pattern>,
    pub body: Expr,
    pub span: Span,
}

/// `else e`; the span starts at the keyword.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ElseArm {
    pub body: Expr,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reduction {
    pub accumulator: Name,
    pub seed: Expr,
    pub binder: Pattern,
    pub collection: Expr,
    pub body: Body,
}

/// Syntax failures retain their complete grammar or normalization facts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    Grammar(Diagnostic),
    Form(FormDiagnostic),
}

impl Error {
    pub fn span(&self) -> Span {
        match self {
            Self::Grammar(diagnostic) => diagnostic.span,
            Self::Form(diagnostic) => diagnostic.span,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormDiagnostic {
    pub kind: FormDiagnosticKind,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FormDiagnosticKind {
    /// A valid grammar construct outside this AST's supported subset.
    UnsupportedConstruct {
        kind: String,
    },
    /// A use declaration cannot combine an alias with imported selectors.
    UseAliasAndImports,
    ComparisonChain,
    RangeChain,
    BodyResultCount {
        found: usize,
    },
    /// A format's point is not followed by a digit count that fits `u32`.
    FormatPrecision {
        written: String,
    },
    /// A format's text after any precision is neither `sci` nor `eng`.
    FormatNotation {
        written: String,
    },
    /// A function signature written where only a value type is admitted. The
    /// diagnostic span is the offending arrow.
    SignatureOutsideParameter {
        position: SignaturePosition,
        signature_span: Span,
    },
    /// The removed spatial replacement `base but key: value`. The diagnostic
    /// span is the colon after the key; `key_span` covers the key.
    RemovedSpatialUpdate {
        key_span: Span,
    },
    /// A parenthesized component key with other than two positions.
    ComponentKeyPositions {
        found: usize,
    },
    /// A type alternative other than `None`, as in `Real|Int`; the span is the
    /// alternative.
    UnionAlternative,
    /// A stated type on a pattern other than a name, discard or positional tuple. The
    /// diagnostic span is the type; `pattern_span` covers the pattern.
    PatternTypeAnnotation {
        pattern_span: Span,
    },
    /// An algebra word on a sum fiber declaration; the span is the word.
    SumAlgebra,
    /// A sum constructor naming other than one variant; the span covers the
    /// parenthesized arguments.
    ConstructorVariantCount {
        found: usize,
    },
    /// An inline sum head applied to something other than one bare unit-variant
    /// name, as in `(A: Real | B)(1.0)`; a bare payload never chooses a
    /// variant. The span covers the parenthesized arguments.
    ConstructorWithoutVariant,
    /// A second `else` arm in one `match`; the span is the second arm and
    /// `first_span` the first.
    DuplicateElseArm {
        first_span: Span,
    },
    /// A variant arm written after the `else` arm; the span is that arm and
    /// `else_span` the `else` arm.
    ArmAfterElse {
        else_span: Span,
    },
    /// A conditional in a `match` arm body, outside brackets, taking an `else`
    /// that begins a line. The diagnostic span is `else_span`, the keyword
    /// alone; `conditional_span` covers the conditional.
    LineLeadingArmElse {
        else_span: Span,
        conditional_span: Span,
    },
}

/// Parse and normalize the supported source subset. No partial AST is returned.
pub fn parse_document(source: &str) -> Result<Document, Vec<Error>> {
    normalize::document(crate::parse_document(source))
}

/// Parse and normalize an expression without changing its source offsets.
pub fn parse_expression(source: &str) -> Result<Expr, Vec<Error>> {
    normalize::expression(crate::parse_expression(source))
}
