//! Source AST for the numerical workload subset, without semantic analysis.
//!
//! All spans are half-open UTF-8 byte offsets into the original input. Unsupported
//! grammar constructs return structured errors instead of being omitted.

mod normalize;

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Function {
    pub name: Name,
    pub parameters: Vec<Parameter>,
    pub body: Expr,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Parameter {
    pub pattern: Pattern,
    pub annotation: Option<Annotation>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct While {
    pub guard: Expr,
    pub writes: Vec<Write>,
    pub body_span: Span,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Write {
    pub target: Name,
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
    If {
        condition: Box<Expr>,
        consequence: Box<Expr>,
        alternative: Box<Expr>,
    },
    Call {
        function: NameRef,
        arguments: Vec<Expr>,
    },
    Index {
        target: Box<Expr>,
        indices: Vec<Expr>,
    },
    Projection {
        key: Name,
        value: Box<Expr>,
    },
    List(Vec<Expr>),
    Range {
        start: Option<Box<Expr>>,
        end: Option<Box<Expr>>,
        inclusive: bool,
    },
    Periodic(Box<Expr>),
    Body(Body),
    For(Box<Comprehension>),
    Reduce(Box<Reduction>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnaryOperator {
    Negate,
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
}

/// Parse and normalize the supported source subset. No partial AST is returned.
pub fn parse_document(source: &str) -> Result<Document, Vec<Error>> {
    normalize::document(crate::parse_document(source))
}

/// Parse and normalize an expression without changing its source offsets.
pub fn parse_expression(source: &str) -> Result<Expr, Vec<Error>> {
    normalize::expression(crate::parse_expression(source))
}
