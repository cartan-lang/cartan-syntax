use super::*;
use crate::{Node, SyntaxTree};

type Result<T> = std::result::Result<T, Error>;

pub(super) fn document(tree: SyntaxTree) -> std::result::Result<Document, Vec<Error>> {
    check_grammar(&tree)?;
    let reader = Reader { tree: &tree };
    let mut items = Vec::new();
    let mut errors = Vec::new();
    for node in children(&tree.root) {
        match reader.item(node) {
            Ok(item) => items.push(item),
            Err(error) => errors.push(error),
        }
    }
    if errors.is_empty() {
        Ok(Document {
            source: tree.source,
            items,
        })
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
    Error::Form(FormDiagnostic {
        kind,
        span: span(node),
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

    fn path_literal(&self, node: &Node) -> Result<PathLiteral> {
        if node.kind != "string" {
            return Err(unsupported(node));
        }
        let raw = self.text(node);
        let content = raw
            .strip_prefix('"')
            .and_then(|text| text.strip_suffix('"'))
            .ok_or_else(|| unsupported(node))?;
        let mut decoded = String::new();
        let mut chars = content.chars();
        while let Some(ch) = chars.next() {
            decoded.push(if ch == '\\' {
                match chars.next().ok_or_else(|| unsupported(node))? {
                    'n' => '\n',
                    't' => '\t',
                    escaped => escaped,
                }
            } else {
                ch
            });
        }
        Ok(PathLiteral {
            raw: raw.into(),
            decoded,
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

    fn optional_annotation(&self, node: &Node) -> Result<Option<Annotation>> {
        node.child("type")
            .map(|child| self.annotation(child))
            .transpose()
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
                shape(node, &["name", "pattern", "type", "kind", "value"], &[])?;
                let pattern = self.pattern(
                    node.child("pattern")
                        .or_else(|| node.child("name"))
                        .ok_or_else(|| unsupported(node))?,
                )?;
                let kind = match node.child("kind").map(|kind| self.text(kind)) {
                    None => BindingKind::Ordinary,
                    Some("param") => BindingKind::Parameter,
                    Some("state") => BindingKind::State,
                    _ => return Err(unsupported(node)),
                };
                ItemKind::Binding(Binding {
                    kind,
                    pattern,
                    annotation: self.optional_annotation(node)?,
                    value: self.expr(field(node, "value")?)?,
                })
            }
            "fiber_declaration" => {
                shape(node, &["name", "algebra", "component"], &[])?;
                let components = node
                    .children_named("component")
                    .map(|component| {
                        shape(component, &["name", "type"], &[])?;
                        Ok(Component {
                            name: self.name(field(component, "name")?)?,
                            annotation: self.annotation(field(component, "type")?)?,
                            span: span(component),
                        })
                    })
                    .collect::<Result<_>>()?;
                ItemKind::Fiber(Fiber {
                    name: self.name(field(node, "name")?)?,
                    algebra: node
                        .child("algebra")
                        .map(|name| self.name(name))
                        .transpose()?,
                    components,
                })
            }
            "function_definition" => {
                shape(node, &["name", "parameters", "body"], &[])?;
                let parameters = field(node, "parameters")?;
                shape(parameters, &[], &["parameter"])?;
                let parameters = children(parameters)
                    .map(|parameter| {
                        shape(parameter, &["name", "type"], &[])?;
                        Ok(Parameter {
                            pattern: self.pattern(field(parameter, "name")?)?,
                            annotation: self.optional_annotation(parameter)?,
                            span: span(parameter),
                        })
                    })
                    .collect::<Result<_>>()?;
                ItemKind::Function(Function {
                    name: self.name(field(node, "name")?)?,
                    parameters,
                    body: self.expr(field(node, "body")?)?,
                })
            }
            "agent" => {
                shape(node, &["guard", "body"], &[])?;
                // `every` without a guard must not become a `while`.
                if !node.children.iter().any(|child| child.kind == "while") {
                    return Err(unsupported(node));
                }
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
                ItemKind::While(While {
                    guard: self.expr(field(node, "guard")?)?,
                    writes,
                    body_span: span(body),
                })
            }
            _ => return Err(unsupported(node)),
        };
        Ok(Item {
            kind,
            span: span(node),
        })
    }

    fn annotation(&self, node: &Node) -> Result<Annotation> {
        if node.kind == "type_annotation" {
            shape(node, &[], &["fiber_type"])?;
            return self.annotation(single(node)?);
        }
        if node.kind != "fiber_type" {
            return Err(unsupported(node));
        }
        shape(node, &["name"], &["type_annotation", "number"])?;
        let name = self.name(field(node, "name")?)?;
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
                        self.annotation(argument).map(TypeArgument::Type)
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
            _ => return Err(unsupported(node)),
        };
        Ok(Pattern {
            kind,
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
                    pattern: self.pattern(field(child, "pattern")?)?,
                    annotation: self.optional_annotation(child)?,
                    value: self.expr(field(child, "value")?)?,
                    span: span(child),
                }));
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

    fn expr(&self, node: &Node) -> Result<Expr> {
        let kind = match node.kind.as_str() {
            "number" => ExprKind::Number(self.text(node).into()),
            "boolean" => ExprKind::Bool(self.text(node) == "true"),
            "identifier" => ExprKind::Name(self.name(node)?),
            "qualified_identifier" => ExprKind::QualifiedName(self.qualified_name(node)?),
            "parenthesized_expression" | "selector" => {
                let mut expr = self.expr(single(node)?)?;
                expr.span = span(node);
                return Ok(expr);
            }
            "unary_expression" => {
                shape(node, &["operator", "operand"], &[])?;
                let operator = field(node, "operator")?;
                let operator = match self.text(operator).trim() {
                    "-" => UnaryOperator::Negate,
                    "not" => UnaryOperator::Not,
                    _ => return Err(unsupported(operator)),
                };
                ExprKind::Unary {
                    operator,
                    operand: Box::new(self.expr(field(node, "operand")?)?),
                }
            }
            "binary_expression" => {
                shape(node, &["operator", "left", "right"], &[])?;
                let left = field(node, "left")?;
                if self.comparison(node) && self.comparison(left) {
                    return Err(form(node, FormDiagnosticKind::ComparisonChain));
                }
                let operator = field(node, "operator")?;
                let operator = match self.text(operator).trim() {
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
                    _ => return Err(unsupported(operator)),
                };
                ExprKind::Binary {
                    operator,
                    left: Box::new(self.expr(left)?),
                    right: Box::new(self.expr(field(node, "right")?)?),
                }
            }
            "conditional_expression" => {
                shape(node, &["condition", "consequence", "alternative"], &[])?;
                ExprKind::If {
                    condition: Box::new(self.expr(field(node, "condition")?)?),
                    consequence: Box::new(self.expr(field(node, "consequence")?)?),
                    alternative: Box::new(self.expr(field(node, "alternative")?)?),
                }
            }
            "call_expression" => {
                shape(node, &["function", "arguments"], &[])?;
                ExprKind::Call {
                    function: self.name_ref(field(node, "function")?)?,
                    arguments: children(field(node, "arguments")?)
                        .map(|node| self.expr(node))
                        .collect::<Result<_>>()?,
                }
            }
            "index_expression" => {
                shape(node, &["target", "index"], &[])?;
                ExprKind::Index {
                    target: Box::new(self.expr(field(node, "target")?)?),
                    indices: children(field(node, "index")?)
                        .map(|node| self.expr(node))
                        .collect::<Result<_>>()?,
                }
            }
            "component_expression" => {
                shape(node, &["key", "value"], &[])?;
                ExprKind::Projection {
                    key: self.name(single(field(node, "key")?)?)?,
                    value: Box::new(self.expr(field(node, "value")?)?),
                }
            }
            "list" => ExprKind::List(
                children(node)
                    .map(|node| self.expr(node))
                    .collect::<Result<_>>()?,
            ),
            "range_expression" | "open_range" | "header_open_range" => {
                shape(node, &["left", "right", "start", "end", "operator"], &[])?;
                let start = node.child("left").or_else(|| node.child("start"));
                let end = node.child("right").or_else(|| node.child("end"));
                if start
                    .into_iter()
                    .chain(end)
                    .any(|child| child.kind == "range_expression")
                {
                    return Err(form(node, FormDiagnosticKind::RangeChain));
                }
                ExprKind::Range {
                    start: start
                        .map(|node| self.expr(node).map(Box::new))
                        .transpose()?,
                    end: end.map(|node| self.expr(node).map(Box::new)).transpose()?,
                    inclusive: self.text(field(node, "operator")?).trim() == "..=",
                }
            }
            "periodic_expression" => {
                shape(node, &["axis"], &[])?;
                ExprKind::Periodic(Box::new(self.expr(field(node, "axis")?)?))
            }
            "body" => ExprKind::Body(self.body(node)?),
            "for_expression" => {
                shape(node, &["binders", "space", "body"], &[])?;
                ExprKind::For(Box::new(Comprehension {
                    binder: self.pattern(field(node, "binders")?)?,
                    collection: self.expr(field(node, "space")?)?,
                    body: self.expr(field(node, "body")?)?,
                }))
            }
            "fold_expression" => {
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
                ExprKind::Reduce(Box::new(Reduction {
                    accumulator: self.name(field(node, "accumulator")?)?,
                    seed: self.expr(field(node, "seed")?)?,
                    binder: self.pattern(field(node, "element")?)?,
                    collection: self.expr(field(node, "driver")?)?,
                    body: self.body(field(node, "body")?)?,
                }))
            }
            _ => return Err(unsupported(node)),
        };
        Ok(Expr {
            kind,
            span: span(node),
        })
    }
}
