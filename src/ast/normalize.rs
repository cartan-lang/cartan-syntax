use super::text;
use super::*;
use crate::{Node, SyntaxTree};

type Result<T> = std::result::Result<T, Error>;

/// Every item the grammar read is normalized, those beside an item it could
/// not read included, so a document's errors are all its grammar diagnostics
/// and the first form error of each item that parsed, in source order.
pub(super) fn document(
    tree: &SyntaxTree,
) -> std::result::Result<(Option<Prose>, Vec<Item>), Vec<Error>> {
    let mut errors: Vec<Error> = tree
        .diagnostics
        .iter()
        .cloned()
        .map(Error::Grammar)
        .collect();
    if errors.is_empty() && tree.root.has_error() {
        return Err(vec![unsupported(&tree.root)]);
    }
    let reader = Reader { tree };
    let mut prose = None;
    let mut items = Vec::new();
    // Documentation waiting for the item after it.
    let mut pending: Option<Prose> = None;
    let misplaced = |prose: Prose, next_span| {
        form_at(
            prose.span,
            FormDiagnosticKind::MisplacedDocumentation { next_span },
        )
    };
    for (index, node) in children(&tree.root).enumerate() {
        if node.kind == "ERROR" {
            // its grammar diagnostic stands; whether it could carry the
            // documentation is not known
            pending = None;
            continue;
        }
        if node.kind == "doc_declaration" {
            match reader.prose(node) {
                Ok(written) if index == 0 => prose = Some(written),
                Ok(written) => {
                    if let Some(previous) = pending.replace(written) {
                        errors.push(misplaced(previous, Some(span(node))));
                    }
                }
                Err(error) => errors.push(error),
            }
            continue;
        }
        match reader.item(node) {
            Ok(mut item) => {
                if let Some(written) = pending.take() {
                    match item.kind {
                        ItemKind::Binding(_)
                        | ItemKind::Function(_)
                        | ItemKind::Fiber(_)
                        | ItemKind::Sum(_)
                        | ItemKind::Struct(_) => item.prose = Some(written),
                        ItemKind::Use(_) | ItemKind::Step(_) | ItemKind::Require(_) => {
                            errors.push(misplaced(written, Some(item.span)))
                        }
                    }
                }
                items.push(item);
            }
            // The item's own error stands; whether it could carry the
            // documentation is not known.
            Err(error) => {
                pending = None;
                errors.push(error);
            }
        }
    }
    if let Some(written) = pending {
        errors.push(misplaced(written, None));
    }
    errors.sort_by_key(|error| error.span().start);
    if errors.is_empty() {
        Ok((prose, items))
    } else {
        Err(errors)
    }
}

pub(super) fn expression(tree: SyntaxTree) -> std::result::Result<Expr, Vec<Error>> {
    check_grammar(&tree)?;
    Reader { tree: &tree }
        .expr(single(&tree.root).map_err(|error| vec![error])?)
        .map_err(|error| vec![error])
}

fn check_grammar(tree: &SyntaxTree) -> std::result::Result<(), Vec<Error>> {
    if !tree.diagnostics.is_empty() {
        return Err(tree
            .diagnostics
            .iter()
            .cloned()
            .map(Error::Grammar)
            .collect());
    }
    if tree.root.has_error() {
        return Err(vec![unsupported(&tree.root)]);
    }
    Ok(())
}

fn span(node: &Node) -> Span {
    Span {
        start: node.start,
        end: node.end,
    }
}

fn children(node: &Node) -> impl Iterator<Item = &Node> {
    node.named_children().filter(|child| !child.extra)
}

fn single(node: &Node) -> Result<&Node> {
    let mut children = children(node);
    let child = children.next().ok_or_else(|| unsupported(node))?;
    if children.next().is_some() {
        return Err(unsupported(node));
    }
    Ok(child)
}

fn field<'a>(node: &'a Node, name: &str) -> Result<&'a Node> {
    node.child(name).ok_or_else(|| unsupported(node))
}

/// Refuse any unhandled named child or field, including optional clauses.
fn shape(node: &Node, fields: &[&str], unlabelled: &[&str]) -> Result<()> {
    for child in node.children.iter().filter(|child| !child.extra) {
        if let Some(label) = &child.field {
            if !fields.contains(&label.as_str()) {
                return Err(unsupported(child));
            }
        } else if child.named && !unlabelled.contains(&child.kind.as_str()) {
            return Err(unsupported(child));
        }
    }
    Ok(())
}

fn form(node: &Node, kind: FormDiagnosticKind) -> Error {
    form_at(span(node), kind)
}

fn form_at(span: Span, kind: FormDiagnosticKind) -> Error {
    Error::Form(FormDiagnostic { kind, span })
}

fn binary_operator(written: &str) -> Option<BinaryOperator> {
    Some(match written {
        "+" => BinaryOperator::Add,
        "-" => BinaryOperator::Subtract,
        "*" => BinaryOperator::Multiply,
        "/" => BinaryOperator::Divide,
        "//" => BinaryOperator::FloorDivide,
        "^" => BinaryOperator::Power,
        "==" => BinaryOperator::Equal,
        "!=" => BinaryOperator::NotEqual,
        "<" => BinaryOperator::Less,
        "<=" => BinaryOperator::LessEqual,
        ">" => BinaryOperator::Greater,
        ">=" => BinaryOperator::GreaterEqual,
        "and" => BinaryOperator::And,
        "or" => BinaryOperator::Or,
        _ => return None,
    })
}

fn unsupported(node: &Node) -> Error {
    form(
        node,
        FormDiagnosticKind::UnsupportedConstruct {
            kind: node.kind.clone(),
        },
    )
}

enum Written<'a> {
    Value(&'a Node),
    Signature(&'a Node),
}

struct Reader<'a> {
    tree: &'a SyntaxTree,
}

impl Reader<'_> {
    fn comparison(&self, node: &Node) -> bool {
        node.kind == "binary_expression"
            && node.child("operator").is_some_and(|operator| {
                matches!(
                    self.text(operator).trim(),
                    "==" | "!=" | "<" | "<=" | ">" | ">="
                )
            })
    }

    fn text<'a>(&'a self, node: &Node) -> &'a str {
        // Nodes come exclusively from this parser invocation.
        &self.tree.source[node.start..node.end]
    }

    fn number(&self, node: &Node) -> Result<NumberLiteral> {
        if node.kind != "number" {
            return Err(unsupported(node));
        }
        Ok(NumberLiteral {
            text: self.text(node).into(),
            span: span(node),
        })
    }

    /// A name, a number, or a parenthesized run of numbers (`component_key`
    /// or `position_pair`); only a run of two forms a pair.
    fn component_key(&self, node: &Node) -> Result<ComponentKey> {
        match node.kind.as_str() {
            "identifier" => Ok(ComponentKey::Name(self.name(node)?)),
            "number" => Ok(ComponentKey::Position(self.number(node)?)),
            "component_key" | "position_pair" => {
                let numbers: Vec<_> = children(node).collect();
                match numbers.as_slice() {
                    [row, column] => Ok(ComponentKey::Pair {
                        row: self.number(row)?,
                        column: self.number(column)?,
                        span: span(node),
                    }),
                    _ => Err(form(
                        node,
                        FormDiagnosticKind::ComponentKeyPositions {
                            found: numbers.len(),
                        },
                    )),
                }
            }
            _ => Err(unsupported(node)),
        }
    }

    fn name(&self, node: &Node) -> Result<Name> {
        if node.kind != "identifier" {
            return Err(unsupported(node));
        }
        Ok(Name {
            text: self.text(node).into(),
            span: span(node),
        })
    }

    fn qualified_name(&self, node: &Node) -> Result<QualifiedName> {
        shape(node, &["namespace", "name"], &[])?;
        let namespace = self.name(field(node, "namespace")?)?;
        let members: Vec<_> = node
            .children_named("name")
            .map(|member| self.name(member))
            .collect::<Result<_>>()?;
        if members.is_empty() {
            return Err(unsupported(node));
        }
        Ok(QualifiedName {
            namespace,
            members,
            span: span(node),
        })
    }

    fn name_ref(&self, node: &Node) -> Result<NameRef> {
        match node.kind.as_str() {
            "identifier" => Ok(NameRef::Local(self.name(node)?)),
            "qualified_identifier" => Ok(NameRef::Qualified(self.qualified_name(node)?)),
            _ => Err(unsupported(node)),
        }
    }

    fn quoted(&self, node: &Node) -> Result<String> {
        self.text(node)
            .strip_prefix('"')
            .and_then(|text| text.strip_suffix('"'))
            .and_then(text::quoted)
            .ok_or_else(|| unsupported(node))
    }

    fn path_literal(&self, node: &Node) -> Result<PathLiteral> {
        if node.kind != "string" {
            return Err(unsupported(node));
        }
        Ok(PathLiteral {
            raw: self.text(node).into(),
            decoded: self.quoted(node)?,
            span: span(node),
        })
    }

    fn string_literal(&self, node: &Node) -> Result<StringLiteral> {
        let (form, value) = match node.kind.as_str() {
            "string" => (StringForm::Quoted, self.quoted(node)?),
            "triple_string" => {
                shape(node, &["text"], &[])?;
                let value = node
                    .child("text")
                    .map_or_else(String::new, |content| text::triple(self.text(content)));
                (StringForm::Triple, value)
            }
            _ => return Err(unsupported(node)),
        };
        Ok(StringLiteral {
            form,
            raw: self.text(node).into(),
            value,
        })
    }

    fn template(&self, node: &Node) -> Result<Template> {
        shape(
            node,
            &[],
            &[
                "chunk",
                "raw_chunk",
                "raw_quote",
                "escape",
                "brace_escape",
                "substitution",
            ],
        )?;
        let form = if self.text(node).starts_with("f\"\"\"") {
            StringForm::Triple
        } else {
            StringForm::Quoted
        };
        // Text runs alternate with holes: one before each hole and one after the last.
        let mut runs: Vec<(Option<Span>, String)> = vec![(None, String::new())];
        let mut holes = Vec::new();
        for child in children(node) {
            if child.kind == "substitution" {
                holes.push(self.hole(child)?);
                runs.push((None, String::new()));
                continue;
            }
            let decoded = match (child.kind.as_str(), self.text(child)) {
                ("chunk" | "raw_chunk" | "raw_quote", written) => written.to_owned(),
                ("escape" | "brace_escape", "{{") => "{".to_owned(),
                ("escape" | "brace_escape", "}}") => "}".to_owned(),
                ("escape", written) => text::quoted(written).ok_or_else(|| unsupported(child))?,
                _ => return Err(unsupported(child)),
            };
            let (run_span, value) = runs.last_mut().expect("runs start nonempty");
            *run_span = Some(match run_span {
                Some(run_span) => Span {
                    start: run_span.start,
                    end: child.end,
                },
                None => span(child),
            });
            value.push_str(&decoded);
        }
        let mut values: Vec<String> = runs.iter().map(|(_, value)| value.clone()).collect();
        if form == StringForm::Triple {
            values = text::triple_pieces(&values);
        }
        let mut holes = holes.into_iter();
        let mut pieces = Vec::new();
        for ((run_span, _), value) in runs.into_iter().zip(values) {
            match run_span {
                Some(run_span) if !value.is_empty() => pieces.push(Piece::Text(TextPiece {
                    raw: self.tree.source[run_span.start..run_span.end].into(),
                    value,
                    span: run_span,
                })),
                _ => {}
            }
            pieces.extend(holes.next().map(Piece::Hole));
        }
        Ok(Template { form, pieces })
    }

    fn hole(&self, node: &Node) -> Result<Hole> {
        shape(node, &["expr", "spec"], &[])?;
        Ok(Hole {
            expr: self.expr(field(node, "expr")?)?,
            format: node
                .child("spec")
                .map(|spec| self.format_spec(spec))
                .transpose()?,
            span: span(node),
        })
    }

    /// The grammar admits only ASCII format characters, so byte offsets are safe.
    fn format_spec(&self, node: &Node) -> Result<FormatSpec> {
        let raw = self
            .text(node)
            .strip_prefix(':')
            .ok_or_else(|| unsupported(node))?;
        let start = node.start + 1;
        let (precision, rest) = match raw.strip_prefix('.') {
            None => (None, 0),
            Some(after) => {
                let end = 1 + after
                    .find(|ch: char| !ch.is_ascii_digit())
                    .unwrap_or(after.len());
                let precision_span = Span {
                    start,
                    end: start + end,
                };
                let digits = raw[1..end].parse::<u32>().map_err(|_| {
                    form_at(
                        precision_span,
                        FormDiagnosticKind::FormatPrecision {
                            written: raw[..end].into(),
                        },
                    )
                })?;
                let precision = Precision {
                    digits,
                    span: precision_span,
                };
                (Some(precision), end)
            }
        };
        let notation = match &raw[rest..] {
            "" => None,
            "sci" => Some(Notation::Sci),
            "eng" => Some(Notation::Eng),
            written => {
                return Err(form_at(
                    Span {
                        start: start + rest,
                        end: node.end,
                    },
                    FormDiagnosticKind::FormatNotation {
                        written: written.into(),
                    },
                ))
            }
        };
        Ok(FormatSpec {
            precision,
            notation,
            raw: raw.into(),
            span: span(node),
        })
    }

    fn import_selection(&self, node: &Node, imports: &Node) -> Result<ImportSelection> {
        shape(imports, &["name"], &[])?;
        let kind = if let Some(star) = imports.children.iter().find(|child| child.kind == "*") {
            Imports::All { span: span(star) }
        } else {
            Imports::Names(
                imports
                    .children_named("name")
                    .map(|name| self.name(name))
                    .collect::<Result<_>>()?,
            )
        };
        let separator = node
            .children
            .iter()
            .find(|child| child.kind == "::")
            .ok_or_else(|| unsupported(imports))?;
        Ok(ImportSelection {
            kind,
            span: Span {
                start: separator.start,
                end: imports.end,
            },
        })
    }

    fn optional_annotation(
        &self,
        node: &Node,
        position: SignaturePosition,
    ) -> Result<Option<Annotation>> {
        node.child("type")
            .map(|child| self.annotation(child, position))
            .transpose()
    }

    /// `(locked when condition)`; the grammar states a clause without `when`
    /// and a condition, so a `lock_clause` always holds both.
    fn lock(&self, node: &Node) -> Result<Lock> {
        shape(node, &["condition"], &[])?;
        Ok(Lock {
            condition: Box::new(self.expr(field(node, "condition")?)?),
            span: span(node),
        })
    }

    fn item(&self, node: &Node) -> Result<Item> {
        let kind = match node.kind.as_str() {
            "use_declaration" => {
                shape(node, &["namespace", "path", "imports", "alias"], &[])?;
                if node.child("alias").is_some() && node.child("imports").is_some() {
                    return Err(form(node, FormDiagnosticKind::UseAliasAndImports));
                }
                let source = if let Some(path) = node.child("path") {
                    UseSource::File(self.path_literal(path)?)
                } else {
                    UseSource::Namespace(self.name(field(node, "namespace")?)?)
                };
                let selection = node
                    .child("imports")
                    .map(|imports| self.import_selection(node, imports))
                    .transpose()?;
                let alias = node
                    .child("alias")
                    .map(|alias| self.name(alias))
                    .transpose()?;
                ItemKind::Use(Use {
                    source,
                    selection,
                    alias,
                })
            }
            "binding" | "pattern_binding" | "port_declaration" => {
                shape(
                    node,
                    &["name", "pattern", "type", "kind", "value", "lock"],
                    &[],
                )?;
                let pattern = self.typed_pattern(
                    node.child("pattern")
                        .or_else(|| node.child("name"))
                        .ok_or_else(|| unsupported(node))?,
                    node.child("type"),
                )?;
                let kind = match node.child("kind").map(|kind| self.text(kind)) {
                    None => BindingKind::Ordinary,
                    Some("param") => BindingKind::Parameter,
                    Some("state") => BindingKind::State,
                    _ => return Err(unsupported(node)),
                };
                let annotation = self.optional_annotation(node, SignaturePosition::Binding)?;
                let value = self.expr(field(node, "value")?)?;
                let lock = node.child("lock").map(|lock| self.lock(lock)).transpose()?;
                ItemKind::Binding(Binding {
                    kind,
                    pattern,
                    annotation,
                    value,
                    lock,
                })
            }
            "fiber_declaration" if node.child("variant").is_some() => {
                shape(node, &["name", "variant"], &[])?;
                ItemKind::Sum(Sum {
                    name: self.name(field(node, "name")?)?,
                    variants: self.variants(node)?,
                })
            }
            "fiber_declaration" => {
                shape(node, &["name", "component"], &[])?;
                ItemKind::Fiber(Fiber {
                    name: self.name(field(node, "name")?)?,
                    components: self.components(node)?,
                })
            }
            "struct_declaration" => {
                shape(node, &["name", "entries"], &[])?;
                let entries = self.struct_entries(field(node, "entries")?)?;
                ItemKind::Struct(Struct {
                    name: self.name(field(node, "name")?)?,
                    entries,
                })
            }
            "function_definition" => {
                shape(node, &["name", "parameters", "body"], &[])?;
                ItemKind::Function(Function {
                    name: self.name(field(node, "name")?)?,
                    parameters: self.parameters(field(node, "parameters")?)?,
                    body: self.expr(field(node, "body")?)?,
                })
            }
            "step_block" => {
                shape(node, &["body"], &[])?;
                let body = field(node, "body")?;
                let writes = children(body)
                    .map(|write| {
                        if write.kind != "write" {
                            return Err(unsupported(write));
                        }
                        shape(write, &["target", "value"], &[])?;
                        Ok(Write {
                            target: self.name(field(write, "target")?)?,
                            value: self.expr(field(write, "value")?)?,
                            span: span(write),
                        })
                    })
                    .collect::<Result<_>>()?;
                ItemKind::Step(Step {
                    writes,
                    body_span: span(body),
                })
            }
            "require_statement" => ItemKind::Require(self.require(node)?),
            _ => return Err(unsupported(node)),
        };
        Ok(Item {
            kind,
            prose: None,
            span: span(node),
        })
    }

    /// A `doc_declaration`'s text; see [`Prose`].
    fn prose(&self, node: &Node) -> Result<Prose> {
        shape(node, &["text"], &[])?;
        let literal = field(node, "text")?;
        shape(literal, &["text"], &[])?;
        Ok(Prose {
            text: literal
                .child("text")
                .map_or_else(String::new, |content| text::prose(self.text(content))),
            span: span(node),
        })
    }

    fn require(&self, node: &Node) -> Result<Require> {
        shape(node, &["condition", "message"], &[])?;
        Ok(Require {
            condition: self.expr(field(node, "condition")?)?,
            message: node
                .child("message")
                .map(|message| self.expr(message))
                .transpose()?,
            span: span(node),
        })
    }

    /// The written type inside a `type_annotation`: a value type, a parenthesized
    /// `signature_type`, or the annotation itself for the unary `T -> R` spelling.
    fn written_type<'a>(&self, node: &'a Node) -> Result<Written<'a>> {
        if node.kind != "type_annotation" {
            return Ok(Written::Value(node));
        }
        if node.child("arrow").is_some() {
            return Ok(Written::Signature(node));
        }
        if node.child("alternative").is_some() {
            // `T|None`: the annotation itself carries the alternative.
            return Ok(Written::Value(node));
        }
        shape(node, &[], &["fiber_type", "signature_type"])?;
        let written = single(node)?;
        Ok(if written.kind == "signature_type" {
            Written::Signature(written)
        } else {
            Written::Value(written)
        })
    }

    /// Token nodes include leading whitespace; the arrow is the last two bytes.
    fn arrow_span(&self, arrow: &Node) -> Span {
        Span {
            start: arrow.end - 2,
            end: arrow.end,
        }
    }

    /// A value type, refusing a signature written at `position`.
    fn annotation(&self, node: &Node, position: SignaturePosition) -> Result<Annotation> {
        match self.written_type(node)? {
            Written::Value(written) => self.value_type(written),
            Written::Signature(written) => Err(form_at(
                self.arrow_span(field(written, "arrow")?),
                FormDiagnosticKind::SignatureOutsideParameter {
                    position,
                    signature_span: span(written),
                },
            )),
        }
    }

    fn parameter_annotation(&self, node: &Node) -> Result<ParameterAnnotation> {
        let written = match self.written_type(node)? {
            Written::Value(written) => {
                return self.value_type(written).map(ParameterAnnotation::Value)
            }
            Written::Signature(written) => written,
        };
        let arguments = if written.kind == "signature_type" {
            shape(written, &["argument", "arrow", "result"], &[])?;
            written
                .children_named("argument")
                .map(|argument| self.annotation(argument, SignaturePosition::SignatureArgument))
                .collect::<Result<_>>()?
        } else {
            // `T -> R`: the single argument is the annotation's own value type,
            // optional when an alternative precedes the arrow.
            shape(
                written,
                &["arrow", "result", "alternative"],
                &["fiber_type"],
            )?;
            let argument = children(written)
                .find(|child| child.field.is_none())
                .ok_or_else(|| unsupported(written))?;
            // Before an arrow the parens are the argument list, so a name and
            // a colon there is a named formal, not an inline product.
            if let Some(component) = argument.child("component") {
                return Err(unsupported(component));
            }
            vec![match written.child("alternative") {
                Some(alternative) => self.optional_type(
                    argument,
                    alternative,
                    Span {
                        start: argument.start,
                        end: alternative.end,
                    },
                )?,
                None => self.value_type(argument)?,
            }]
        };
        Ok(ParameterAnnotation::Signature(Signature {
            arguments,
            result: self.annotation(
                field(written, "result")?,
                SignaturePosition::SignatureResult,
            )?,
            arrow_span: self.arrow_span(field(written, "arrow")?),
            span: span(written),
        }))
    }

    /// `T|None`; any other alternative is refused, since this is the only union.
    fn optional_type(&self, value: &Node, alternative: &Node, whole: Span) -> Result<Annotation> {
        let none = self.value_type(alternative)?;
        if !matches!(&none.kind, AnnotationKind::Named(NameRef::Local(name)) if name.text == "None")
        {
            return Err(form(alternative, FormDiagnosticKind::UnionAlternative));
        }
        Ok(Annotation {
            kind: AnnotationKind::Optional {
                value: Box::new(self.value_type(value)?),
                none_span: none.span,
            },
            span: whole,
        })
    }

    /// A product's components in declared order, keeping duplicates.
    fn components(&self, node: &Node) -> Result<Vec<Component>> {
        node.children_named("component")
            .map(|component| {
                shape(component, &["name", "type"], &[])?;
                Ok(Component {
                    name: self.name(field(component, "name")?)?,
                    annotation: self.value_type(field(component, "type")?)?,
                    span: span(component),
                })
            })
            .collect()
    }

    /// A roster's variants in declared order, keeping duplicates.
    fn variants(&self, node: &Node) -> Result<Vec<Variant>> {
        node.children_named("variant")
            .map(|variant| {
                shape(variant, &["name", "type"], &[])?;
                Ok(Variant {
                    name: self.name(field(variant, "name")?)?,
                    payload: variant
                        .child("type")
                        .map(|payload| self.value_type(payload))
                        .transpose()?,
                    span: span(variant),
                })
            })
            .collect()
    }

    /// `Shape(A: 1.0)`, `(A: Real | B)(A: 1.0)` and `(A: Real | B)(B)`.
    fn construct(&self, node: &Node) -> Result<ExprKind> {
        shape(
            node,
            &["function", "described", "variants", "arguments"],
            &[],
        )?;
        let head = match (node.child("function"), node.child("described")) {
            (Some(function), None) => ConstructorHead::Named(self.name_ref(function)?),
            (None, Some(described)) if described.child("variant").is_some() => {
                shape(described, &["variant"], &[])?;
                ConstructorHead::Inline {
                    variants: self.variants(described)?,
                    span: span(described),
                }
            }
            // An inline product description as the head stays unsupported.
            (None, Some(described)) => return Err(unsupported(described)),
            _ => return Err(unsupported(node)),
        };
        if let Some(arguments) = node.child("variants") {
            let written: Vec<_> = children(arguments).collect();
            let [argument] = written.as_slice() else {
                return Err(form(
                    arguments,
                    FormDiagnosticKind::ConstructorVariantCount {
                        found: written.len(),
                    },
                ));
            };
            shape(argument, &["name", "value"], &[])?;
            return Ok(ExprKind::Construct {
                head,
                variant: self.name(field(argument, "name")?)?,
                payload: Some(Box::new(self.expr(field(argument, "value")?)?)),
            });
        }
        // Only an inline head reaches here with positional arguments.
        let arguments = field(node, "arguments")?;
        match children(arguments).collect::<Vec<_>>().as_slice() {
            [variant] if variant.kind == "identifier" && self.text(variant) != "_" => {
                Ok(ExprKind::Construct {
                    head,
                    variant: self.name(variant)?,
                    payload: None,
                })
            }
            _ => Err(form(
                arguments,
                FormDiagnosticKind::ConstructorWithoutVariant,
            )),
        }
    }

    /// Struct entries in source order, keeping duplicates and an empty list.
    fn struct_entries(&self, node: &Node) -> Result<Vec<StructEntry>> {
        shape(node, &["entry"], &[])?;
        node.children_named("entry")
            .map(|entry| {
                shape(entry, &["name", "type", "alternative"], &[])?;
                let written = field(entry, "type")?;
                let annotation = match entry.child("alternative") {
                    Some(alternative) => self.optional_type(
                        written,
                        alternative,
                        Span {
                            start: written.start,
                            end: alternative.end,
                        },
                    )?,
                    None => self.value_type(written)?,
                };
                Ok(StructEntry {
                    name: self.name(field(entry, "name")?)?,
                    annotation,
                    span: span(entry),
                })
            })
            .collect()
    }

    /// A named type, generic application or inline struct; struct entries and
    /// fiber components take this form directly.
    fn value_type(&self, node: &Node) -> Result<Annotation> {
        if node.kind == "type_annotation" {
            shape(node, &["alternative"], &["fiber_type"])?;
            let value = children(node)
                .find(|child| child.field.is_none())
                .ok_or_else(|| unsupported(node))?;
            return self.optional_type(value, field(node, "alternative")?, span(node));
        }
        if node.kind != "fiber_type" {
            return Err(unsupported(node));
        }
        if node.child("variant").is_some() {
            // An inline sum roster; its span covers the parentheses.
            shape(node, &["variant"], &[])?;
            return Ok(Annotation {
                kind: AnnotationKind::Sum(self.variants(node)?),
                span: span(node),
            });
        }
        if node.child("component").is_some() {
            // An inline product; its span covers the parentheses.
            shape(node, &["component"], &[])?;
            return Ok(Annotation {
                kind: AnnotationKind::Product(self.components(node)?),
                span: span(node),
            });
        }
        if let Some(entries) = node.child("entries") {
            // An inline struct; its span covers the braces.
            shape(node, &["entries"], &[])?;
            return Ok(Annotation {
                kind: AnnotationKind::Struct(self.struct_entries(entries)?),
                span: span(node),
            });
        }
        if let Some(tuple) = node.child("list") {
            shape(node, &["list"], &[])?;
            shape(tuple, &["item", "width"], &[])?;
            let items = tuple.children_named("item");
            let kind = if let Some(width) = tuple.child("width") {
                let items: Vec<_> = items.collect();
                let [item] = items.as_slice() else {
                    return Err(unsupported(tuple));
                };
                AnnotationKind::TupleRepeat {
                    item: Box::new(self.value_type(item)?),
                    width: if width.kind == "number" {
                        TupleWidth::Number(self.number(width)?)
                    } else {
                        TupleWidth::Name(self.name(width)?)
                    },
                }
            } else {
                AnnotationKind::Tuple(
                    items
                        .map(|item| self.value_type(item))
                        .collect::<Result<_>>()?,
                )
            };
            return Ok(Annotation {
                kind,
                span: span(node),
            });
        }
        shape(node, &["name"], &["type_annotation", "number"])?;
        let name = self.name_ref(field(node, "name")?)?;
        let kind = if node.children.iter().any(|child| child.kind == "<") {
            let arguments = children(node)
                .filter(|child| child.field.is_none())
                .map(|argument| {
                    if argument.kind == "number" {
                        Ok(TypeArgument::Number {
                            text: self.text(argument).into(),
                            span: span(argument),
                        })
                    } else {
                        self.annotation(argument, SignaturePosition::TypeArgument)
                            .map(TypeArgument::Type)
                    }
                })
                .collect::<Result<_>>()?;
            AnnotationKind::Apply { name, arguments }
        } else {
            AnnotationKind::Named(name)
        };
        Ok(Annotation {
            kind,
            span: span(node),
        })
    }

    fn parameters(&self, list: &Node) -> Result<Vec<Parameter>> {
        shape(list, &[], &["parameter"])?;
        children(list)
            .map(|parameter| {
                shape(parameter, &["name", "type"], &[])?;
                let written_type = parameter.child("type");
                Ok(Parameter {
                    pattern: self.typed_pattern(field(parameter, "name")?, written_type)?,
                    annotation: written_type
                        .map(|written| self.parameter_annotation(written))
                        .transpose()?,
                    span: span(parameter),
                })
            })
            .collect()
    }

    /// A pattern with an optional stated type, which only a name, a discard or
    /// a positional tuple may carry.
    fn typed_pattern(&self, node: &Node, written_type: Option<&Node>) -> Result<Pattern> {
        let pattern = self.pattern(node)?;
        match (&pattern.kind, written_type) {
            (PatternKind::Name(_) | PatternKind::Wildcard | PatternKind::Site(_), _)
            | (_, None) => Ok(pattern),
            (_, Some(written_type)) => Err(form(
                written_type,
                FormDiagnosticKind::PatternTypeAnnotation {
                    pattern_span: pattern.span,
                },
            )),
        }
    }

    fn pattern(&self, node: &Node) -> Result<Pattern> {
        let kind = match node.kind.as_str() {
            "binder_list" => return self.pattern(single(node)?),
            "identifier" if self.text(node) == "_" => PatternKind::Wildcard,
            "identifier" => PatternKind::Name(self.name(node)?),
            "bare_list_pattern" | "list_pattern" => PatternKind::List(
                children(node)
                    .map(|node| self.pattern(node))
                    .collect::<Result<_>>()?,
            ),
            "site_pattern" => PatternKind::Site(
                children(node)
                    .map(|node| self.pattern(node))
                    .collect::<Result<_>>()?,
            ),
            "constructor_pattern" => {
                // An inline product description as the head stays unsupported.
                if let Some(described) = node.child("described") {
                    return Err(unsupported(described));
                }
                PatternKind::Constructor {
                    head: self.name(field(node, "head")?)?,
                    items: children(node)
                        .filter(|child| child.field.is_none())
                        .map(|item| self.pattern(item))
                        .collect::<Result<_>>()?,
                }
            }
            "named_pattern" => PatternKind::Components(
                children(node)
                    .map(|component| self.component_pattern(component))
                    .collect::<Result<_>>()?,
            ),
            "map_pattern" => PatternKind::Entries(
                children(node)
                    .map(|entry| {
                        if entry.kind != "map_sub_pattern" {
                            return Err(unsupported(entry));
                        }
                        shape(entry, &["key", "pattern"], &[])?;
                        Ok(EntryPattern {
                            key: self.name(field(entry, "key")?)?,
                            pattern: entry
                                .child("pattern")
                                .map(|pattern| self.pattern(pattern))
                                .transpose()?,
                            span: span(entry),
                        })
                    })
                    .collect::<Result<_>>()?,
            ),
            _ => return Err(unsupported(node)),
        };
        Ok(Pattern {
            kind,
            span: span(node),
        })
    }

    fn component_pattern(&self, node: &Node) -> Result<ComponentPattern> {
        if node.kind != "named_sub_pattern" {
            return Err(unsupported(node));
        }
        shape(node, &["name", "pattern"], &[])?;
        Ok(ComponentPattern {
            key: self.component_key(field(node, "name")?)?,
            pattern: self.pattern(field(node, "pattern")?)?,
            span: span(node),
        })
    }

    fn body(&self, node: &Node) -> Result<Body> {
        if node.kind != "body" {
            return Err(unsupported(node));
        }
        let mut items = Vec::new();
        let mut results = 0;
        for child in children(node) {
            if child.kind == "local" {
                shape(child, &["pattern", "type", "value"], &[])?;
                items.push(BodyItem::Local(Local {
                    pattern: self.typed_pattern(field(child, "pattern")?, child.child("type"))?,
                    annotation: self.optional_annotation(child, SignaturePosition::Local)?,
                    value: self.expr(field(child, "value")?)?,
                    span: span(child),
                }));
            } else if child.kind == "require_statement" {
                items.push(BodyItem::Require(self.require(child)?));
            } else {
                items.push(BodyItem::Result(self.expr(child)?));
                results += 1;
            }
        }
        if results != 1 {
            return Err(form(
                node,
                FormDiagnosticKind::BodyResultCount { found: results },
            ));
        }
        Ok(Body {
            items,
            span: span(node),
        })
    }

    /// **Every level of an expression's nesting passes through here**, so
    /// this frame is kept to a dispatch: each form is read by a method of
    /// its own, whose frame stands on the stack only while that form is
    /// read, and the arms share one result.
    fn expr(&self, node: &Node) -> Result<Expr> {
        let kind = match node.kind.as_str() {
            "parenthesized_expression" | "selector" => return self.grouped(node),
            "binary_expression" => return self.binary(node),
            "number" | "boolean" | "identifier" | "none" => self.word(node),
            "string" | "triple_string" => self.string_literal(node).map(ExprKind::String),
            "template_string" => self.template(node).map(ExprKind::Template),
            "qualified_identifier" => self.qualified_name(node).map(ExprKind::QualifiedName),
            "unary_expression" => self.unary(node),
            "conditional_expression" => self.conditional(node),
            "function_expression" => {
                shape(node, &["parameters", "body"], &[])?;
                Ok(ExprKind::Function {
                    parameters: self.parameters(field(node, "parameters")?)?,
                    body: Box::new(self.expr(field(node, "body")?)?),
                })
            }
            "absent_expression" => self.lift(node),
            "call_expression"
                if node.child("variants").is_some() || node.child("described").is_some() =>
            {
                self.construct(node)
            }
            "call_expression" => self.call(node),
            "index_expression" => self.index(node),
            "component_expression" => self.projection(node),
            "product" => self.product(node),
            "but_expression" => self.update(node),
            "list" => self.list(node),
            "range_expression" | "open_range" | "header_open_range" | "range_from" | "range_to"
            | "full_range" => self.range(node),
            "periodic_expression" => self.periodic(node),
            "periodic_range" => self.periodic_range(node),
            "map" => self.map(node),
            "configurable_expression" => self.configurable(node),
            "config_expression" | "configure_dot_expression" => self.configure(node),
            "body" => self.body(node).map(ExprKind::Body),
            "for_expression" | "each_expression" => self.comprehension(node),
            "fold_expression" => self.reduction(node),
            "match_expression" => self
                .match_expression(node)
                .map(|m| ExprKind::Match(Box::new(m))),
            _ => Err(unsupported(node)),
        };
        Ok(Expr {
            kind: kind?,
            span: span(node),
        })
    }

    /// A literal number, Boolean or `none`, or a name.
    fn word(&self, node: &Node) -> Result<ExprKind> {
        Ok(match node.kind.as_str() {
            "number" => ExprKind::Number(self.text(node).into()),
            "boolean" => ExprKind::Bool(self.text(node) == "true"),
            "none" => ExprKind::None,
            _ => ExprKind::Name(self.name(node)?),
        })
    }

    /// Grouping parentheses normalize away, the span covering them.
    fn grouped(&self, node: &Node) -> Result<Expr> {
        let mut expr = self.expr(single(node)?)?;
        expr.span = span(node);
        Ok(expr)
    }

    fn unary(&self, node: &Node) -> Result<ExprKind> {
        shape(node, &["operator", "operand"], &[])?;
        let operator = field(node, "operator")?;
        let operator = match self.text(operator).trim() {
            "-" => UnaryOperator::Negate,
            "+" => UnaryOperator::Plus,
            "not" => UnaryOperator::Not,
            _ => return Err(unsupported(operator)),
        };
        Ok(ExprKind::Unary {
            operator,
            operand: Box::new(self.expr(field(node, "operand")?)?),
        })
    }

    fn binary(&self, node: &Node) -> Result<Expr> {
        shape(node, &["operator", "left", "right"], &[])?;
        let left = field(node, "left")?;
        if self.comparison(node) && self.comparison(left) {
            return Err(form(node, FormDiagnosticKind::ComparisonChain));
        }
        let operator = field(node, "operator")?;
        let kind = match self.text(operator).trim() {
            "??" => {
                // Continuation tokens carry surrounding whitespace.
                let start = operator.start
                    + self
                        .text(operator)
                        .find("??")
                        .ok_or_else(|| unsupported(operator))?;
                ExprKind::Default {
                    value: Box::new(self.expr(left)?),
                    fallback: Box::new(self.expr(field(node, "right")?)?),
                    operator_span: Span {
                        start,
                        end: start + 2,
                    },
                }
            }
            "<>" => ExprKind::MapJoin {
                left: Box::new(self.expr(left)?),
                right: Box::new(self.expr(field(node, "right")?)?),
                operator_span: Span {
                    start: operator.end - 2,
                    end: operator.end,
                },
            },
            written => ExprKind::Binary {
                operator: binary_operator(written).ok_or_else(|| unsupported(operator))?,
                left: Box::new(self.expr(left)?),
                right: Box::new(self.expr(field(node, "right")?)?),
            },
        };
        Ok(Expr {
            kind,
            span: span(node),
        })
    }

    fn conditional(&self, node: &Node) -> Result<ExprKind> {
        shape(node, &["condition", "consequence", "alternative"], &[])?;
        Ok(ExprKind::If {
            condition: Box::new(self.expr(field(node, "condition")?)?),
            consequence: Box::new(self.branch(field(node, "consequence")?)?),
            alternative: node
                .child("alternative")
                .map(|alternative| self.branch(alternative).map(Box::new))
                .transpose()?,
        })
    }

    /// A conditional's branch or a `match` arm's braced body: its result
    /// alone where the body holds nothing else, and the body otherwise. An
    /// `else if` alternative is the inner conditional.
    fn branch(&self, node: &Node) -> Result<Expr> {
        if node.kind != "body" {
            return self.expr(node);
        }
        let mut body = self.body(node)?;
        // A body holds exactly one result, so a body of one item is that result.
        if body.items.len() == 1 {
            if let Some(BodyItem::Result(result)) = body.items.pop() {
                return Ok(result);
            }
        }
        Ok(Expr {
            span: body.span,
            kind: ExprKind::Body(body),
        })
    }

    fn lift(&self, node: &Node) -> Result<ExprKind> {
        shape(node, &["value"], &[])?;
        let mark = node
            .children
            .iter()
            .rev()
            .find(|child| child.kind == "?")
            .ok_or_else(|| unsupported(node))?;
        Ok(ExprKind::Lift {
            value: Box::new(self.expr(field(node, "value")?)?),
            mark_span: Span {
                start: mark.end - 1,
                end: mark.end,
            },
        })
    }

    fn call(&self, node: &Node) -> Result<ExprKind> {
        shape(node, &["function", "arguments"], &[])?;
        Ok(ExprKind::Call {
            function: self.name_ref(field(node, "function")?)?,
            arguments: self.exprs(field(node, "arguments")?)?,
        })
    }

    /// The expressions a node holds, in order.
    fn exprs(&self, node: &Node) -> Result<Vec<Expr>> {
        children(node).map(|node| self.expr(node)).collect()
    }

    fn index(&self, node: &Node) -> Result<ExprKind> {
        shape(node, &["target", "index"], &[])?;
        Ok(ExprKind::Index {
            target: Box::new(self.expr(field(node, "target")?)?),
            indices: self.exprs(field(node, "index")?)?,
        })
    }

    fn projection(&self, node: &Node) -> Result<ExprKind> {
        shape(node, &["key", "value"], &[])?;
        let key = field(node, "key")?;
        // A component key wraps a name, a number or a parenthesized run.
        let key = match children(key).collect::<Vec<_>>().as_slice() {
            [single] => self.component_key(single)?,
            _ => self.component_key(key)?,
        };
        Ok(ExprKind::Projection {
            key,
            value: Box::new(self.expr(field(node, "value")?)?),
        })
    }

    fn product(&self, node: &Node) -> Result<ExprKind> {
        children(node)
            .map(|entry| {
                if entry.kind != "product_component" {
                    return Err(unsupported(entry));
                }
                shape(entry, &["name", "value"], &[])?;
                Ok(ProductEntry {
                    key: self.component_key(field(entry, "name")?)?,
                    value: self.expr(field(entry, "value")?)?,
                    span: span(entry),
                })
            })
            .collect::<Result<_>>()
            .map(ExprKind::Product)
    }

    fn update(&self, node: &Node) -> Result<ExprKind> {
        shape(node, &["target", "override"], &[])?;
        let keyword = node
            .children
            .iter()
            .find(|child| child.kind == "but")
            .ok_or_else(|| unsupported(node))?;
        // Continuation keyword tokens carry surrounding whitespace.
        let start = keyword.start
            + self
                .text(keyword)
                .find("but")
                .ok_or_else(|| unsupported(keyword))?;
        Ok(ExprKind::Update {
            base: Box::new(self.expr(field(node, "target")?)?),
            overrides: Box::new(self.expr(field(node, "override")?)?),
            operator_span: Span {
                start,
                end: start + 3,
            },
        })
    }

    fn list(&self, node: &Node) -> Result<ExprKind> {
        self.exprs(node).map(ExprKind::List)
    }

    fn range(&self, node: &Node) -> Result<ExprKind> {
        shape(node, &["left", "right", "start", "end", "operator"], &[])?;
        let start = node.child("left").or_else(|| node.child("start"));
        let end = node.child("right").or_else(|| node.child("end"));
        if start.into_iter().chain(end).any(|child| {
            matches!(
                child.kind.as_str(),
                "range_expression" | "range_from" | "range_to" | "full_range" | "periodic_range"
            )
        }) {
            return Err(form(node, FormDiagnosticKind::RangeChain));
        }
        Ok(ExprKind::Range {
            start: start
                .map(|node| self.expr(node).map(Box::new))
                .transpose()?,
            end: end.map(|node| self.expr(node).map(Box::new)).transpose()?,
            inclusive: self.text(field(node, "operator")?).trim() == "..=",
        })
    }

    fn periodic(&self, node: &Node) -> Result<ExprKind> {
        shape(node, &["axis"], &[])?;
        let axis = field(node, "axis")?;
        if axis.kind == "periodic_range" {
            return Err(form(node, FormDiagnosticKind::PeriodicTwice));
        }
        Ok(ExprKind::Periodic(Box::new(self.expr(axis)?)))
    }

    fn periodic_range(&self, node: &Node) -> Result<ExprKind> {
        shape(node, &["operator", "axis"], &[])?;
        Ok(ExprKind::Periodic(Box::new(
            self.expr(field(node, "axis")?)?,
        )))
    }

    fn map(&self, node: &Node) -> Result<ExprKind> {
        children(node)
            .map(|entry| self.map_entry(entry))
            .collect::<Result<_>>()
            .map(ExprKind::Map)
    }

    fn map_entry(&self, entry: &Node) -> Result<MapEntry> {
        if entry.kind != "map_entry" {
            return Err(unsupported(entry));
        }
        shape(entry, &["key", "value", "separator"], &[])?;
        let separator = field(entry, "separator")?;
        let written = self.text(separator).trim();
        let key = field(entry, "key")?;
        let key = match written {
            ":" => MapKey::Static(self.name(key)?),
            "=>" => MapKey::Computed(self.expr(key)?),
            _ => return Err(unsupported(separator)),
        };
        Ok(MapEntry {
            key,
            value: self.expr(field(entry, "value")?)?,
            span: span(entry),
            separator_span: Span {
                start: separator.end - written.len(),
                end: separator.end,
            },
        })
    }

    fn configurable(&self, node: &Node) -> Result<ExprKind> {
        shape(node, &["operator", "config", "body"], &[])?;
        let values: Vec<_> = node.children_named("config").collect();
        let operators: Vec<_> = node.children_named("operator").collect();
        if values.is_empty() || values.len() != operators.len() {
            return Err(unsupported(node));
        }
        let settings = values
            .into_iter()
            .zip(operators)
            .map(|(value, operator)| {
                Ok(SettingsAtom {
                    value: self.expr(value)?,
                    operator_span: Span {
                        start: operator.end - 1,
                        end: operator.end,
                    },
                })
            })
            .collect::<Result<_>>()?;
        Ok(ExprKind::Configurable {
            settings,
            body: self.body(field(node, "body")?)?,
        })
    }

    fn configure(&self, node: &Node) -> Result<ExprKind> {
        shape(node, &["target", "config", "operator"], &[])?;
        let operator = field(node, "operator")?;
        Ok(ExprKind::Configure {
            kind: if node.kind == "config_expression" {
                ConfigureKind::Colon
            } else {
                ConfigureKind::Dot
            },
            target: Box::new(self.expr(field(node, "target")?)?),
            settings: Box::new(self.expr(field(node, "config")?)?),
            operator_span: Span {
                start: operator.end - 1,
                end: operator.end,
            },
        })
    }

    /// `for` and `each`, which share their header.
    fn comprehension(&self, node: &Node) -> Result<ExprKind> {
        let collection = if node.kind == "for_expression" {
            "space"
        } else {
            "collection"
        };
        shape(node, &["binders", collection, "body"], &[])?;
        let comprehension = Box::new(Comprehension {
            binder: self.pattern(field(node, "binders")?)?,
            collection: self.expr(field(node, collection)?)?,
            body: self.expr(field(node, "body")?)?,
        });
        Ok(if node.kind == "for_expression" {
            ExprKind::For(comprehension)
        } else {
            ExprKind::Each(comprehension)
        })
    }

    fn reduction(&self, node: &Node) -> Result<ExprKind> {
        shape(
            node,
            &[
                "keyword",
                "accumulator",
                "seed",
                "element",
                "driver",
                "body",
            ],
            &[],
        )?;
        if self.text(field(node, "keyword")?) != "reduce" {
            return Err(unsupported(node));
        }
        Ok(ExprKind::Reduce(Box::new(Reduction {
            accumulator: self.name(field(node, "accumulator")?)?,
            seed: self.expr(field(node, "seed")?)?,
            binder: self.pattern(field(node, "element")?)?,
            collection: self.expr(field(node, "driver")?)?,
            body: self.body(field(node, "body")?)?,
        })))
    }

    fn match_expression(&self, node: &Node) -> Result<Match> {
        shape(node, &["scrutinee"], &["match_arm", "else_arm"])?;
        let mut arms = Vec::new();
        let mut otherwise: Option<ElseArm> = None;
        for arm in children(node).filter(|child| child.field.is_none()) {
            let arm_span = self.arm_span(arm)?;
            if let Some(first) = &otherwise {
                let kind = if arm.kind == "else_arm" {
                    FormDiagnosticKind::DuplicateElseArm {
                        first_span: first.span,
                    }
                } else {
                    FormDiagnosticKind::ArmAfterElse {
                        else_span: first.span,
                    }
                };
                return Err(form_at(arm_span, kind));
            }
            if arm.kind == "else_arm" {
                shape(arm, &["body"], &[])?;
                otherwise = Some(ElseArm {
                    body: self.branch(field(arm, "body")?)?,
                    span: arm_span,
                });
                continue;
            }
            shape(arm, &["variant", "pattern", "body"], &["named_sub_pattern"])?;
            let components: Vec<_> = children(arm)
                .filter(|child| child.kind == "named_sub_pattern")
                .collect();
            let payload = match (arm.child("pattern"), components.as_slice()) {
                (Some(pattern), []) => Some(self.pattern(pattern)?),
                (None, []) => None,
                // `B(X: x, Y: y)` stands for `B((X: x, Y: y))`; the span covers
                // the components inside the arm's parentheses.
                (None, [first, ..]) => Some(Pattern {
                    kind: PatternKind::Components(
                        components
                            .iter()
                            .map(|component| self.component_pattern(component))
                            .collect::<Result<_>>()?,
                    ),
                    span: Span {
                        start: first.start,
                        end: components[components.len() - 1].end,
                    },
                }),
                (Some(_), [extra, ..]) => return Err(unsupported(extra)),
            };
            arms.push(MatchArm {
                variant: self.name(field(arm, "variant")?)?,
                payload,
                body: self.branch(field(arm, "body")?)?,
                span: arm_span,
            });
        }
        Ok(Match {
            scrutinee: self.expr(field(node, "scrutinee")?)?,
            arms,
            otherwise,
        })
    }

    /// An arm's span from its first word: an `else` opening a line is one
    /// token carrying the newlines before it.
    fn arm_span(&self, arm: &Node) -> Result<Span> {
        let keyword = arm
            .children
            .first()
            .filter(|keyword| keyword.kind == "else" && !keyword.named);
        let start = match keyword {
            Some(keyword) => {
                keyword.start
                    + self
                        .text(keyword)
                        .find("else")
                        .ok_or_else(|| unsupported(keyword))?
            }
            None => arm.start,
        };
        Ok(Span {
            start,
            end: arm.end,
        })
    }
}
