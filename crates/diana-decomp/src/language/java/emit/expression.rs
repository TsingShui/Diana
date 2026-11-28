use super::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Typing {
    Standalone,
    Target,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Precedence {
    Assignment,
    Conditional,
    LogicalOr,
    LogicalAnd,
    BitOr,
    BitXor,
    BitAnd,
    Equality,
    Relational,
    Shift,
    Additive,
    Multiplicative,
    Unary,
    Primary,
}

impl Precedence {
    fn binary(op: JavaBinaryOp) -> Self {
        match op {
            JavaBinaryOp::Multiply | JavaBinaryOp::Divide | JavaBinaryOp::Remainder => {
                Self::Multiplicative
            }
            JavaBinaryOp::Add | JavaBinaryOp::StringConcat | JavaBinaryOp::Subtract => {
                Self::Additive
            }
            JavaBinaryOp::ShiftLeft
            | JavaBinaryOp::ShiftRight
            | JavaBinaryOp::UnsignedShiftRight => Self::Shift,
            JavaBinaryOp::Less
            | JavaBinaryOp::GreaterEqual
            | JavaBinaryOp::Greater
            | JavaBinaryOp::LessEqual
            | JavaBinaryOp::IntegralLess
            | JavaBinaryOp::IntegralGreaterEqual
            | JavaBinaryOp::IntegralGreater
            | JavaBinaryOp::IntegralLessEqual => Self::Relational,
            JavaBinaryOp::Equal | JavaBinaryOp::NotEqual => Self::Equality,
            JavaBinaryOp::BitAnd => Self::BitAnd,
            JavaBinaryOp::BitXor => Self::BitXor,
            JavaBinaryOp::BitOr => Self::BitOr,
            JavaBinaryOp::LogicalAnd => Self::LogicalAnd,
            JavaBinaryOp::LogicalOr => Self::LogicalOr,
        }
    }
    fn of(expression: &JavaExpr) -> Self {
        match expression {
            JavaExpr::Assignment { .. }
            | JavaExpr::Lambda { .. }
            | JavaExpr::BlockLambda { .. } => Self::Assignment,
            JavaExpr::Conditional { .. } => Self::Conditional,
            JavaExpr::InstanceOf { .. } => Self::Relational,
            JavaExpr::Binary { op, .. } => Self::binary(*op),
            JavaExpr::Unary { .. } | JavaExpr::Cast { .. } | JavaExpr::Update { .. } => Self::Unary,
            _ => Self::Primary,
        }
    }
}

#[derive(Clone, Copy)]
enum Requirement {
    Any,
    AtLeast(Precedence),
    Above(Precedence),
    BinaryRight(JavaBinaryOp),
}

impl Requirement {
    fn parentheses(self, expression: &JavaExpr) -> bool {
        match self {
            Self::Any => false,
            Self::AtLeast(min) => Precedence::of(expression) < min,
            Self::Above(min) => Precedence::of(expression) <= min,
            Self::BinaryRight(parent) => {
                let p = Precedence::of(expression);
                let threshold = Precedence::binary(parent);
                p < threshold
                    || (p == threshold
                        && !matches!(expression, JavaExpr::Binary { op, .. }
                    if *op == parent && matches!(parent, JavaBinaryOp::BitAnd | JavaBinaryOp::BitOr |
                        JavaBinaryOp::BitXor | JavaBinaryOp::LogicalAnd | JavaBinaryOp::LogicalOr)))
            }
        }
    }
}

/// The parent operator fixes the required precedence before the child is printed.
/// Each task appends directly to the expression's one output buffer.
enum Token<'a> {
    Visit(&'a JavaExpr, Requirement, Typing),
    Text(&'static str),
    Infix(&'static str),
    Name(&'a JavaIdentifier),
    Type(&'a JavaType),
    Target(&'a crate::ir::MethodReference),
    Method(&'a [JavaType], &'a JavaIdentifier),
    NewType(&'a JavaType, bool, bool),
    ArrayRank(usize),
    Body(String),
    InvalidConstruction,
}

type Pending<'a> = smallvec::SmallVec<[Token<'a>; 8]>;

fn sequence<'a>(pending: &mut Pending<'a>, values: &'a [JavaExpr], typing: Typing) {
    for (index, value) in values.iter().enumerate().rev() {
        pending.push(Token::Visit(value, Requirement::Any, typing));
        if index != 0 {
            pending.push(Token::Text(", "));
        }
    }
}

/// Inlining and declaration recovery can expose constant operands. A field
/// can name a constant variable even when the input lacks its declaration.
fn may_be_constant(left: &JavaExpr, right: &JavaExpr) -> bool {
    let mut pending = smallvec::SmallVec::<[&JavaExpr; 8]>::from_slice(&[left, right]);
    while let Some(expression) = pending.pop() {
        match expression {
            JavaExpr::Literal(_) | JavaExpr::StaticField { .. } => {}
            JavaExpr::Name(name) if !name.is_local_reference() => {}
            JavaExpr::Field { owner, .. } => {
                // A field reached through an expression such as `this` is
                // distinct from a qualified name of a constant variable.
                pending.push(owner);
            }
            JavaExpr::Unary { operand, .. } | JavaExpr::Cast { value: operand, .. } => {
                pending.push(operand);
            }
            JavaExpr::Binary { left, op, right } if *op != JavaBinaryOp::StringConcat => {
                pending.extend([left.as_ref(), right.as_ref()]);
            }
            JavaExpr::Conditional {
                condition,
                when_true,
                when_false,
            } => {
                pending.extend([condition.as_ref(), when_true.as_ref(), when_false.as_ref()]);
            }
            _ => return false,
        }
    }
    true
}

impl JavaPrinter {
    pub(super) fn expression_at(
        &self,
        expression: &JavaExpr,
        depth: usize,
    ) -> Result<String, JavaPrintError> {
        self.render_expression(expression, Typing::Standalone, depth)
    }
    pub(super) fn target_typed_expression_at(
        &self,
        expression: &JavaExpr,
        depth: usize,
    ) -> Result<String, JavaPrintError> {
        self.render_expression(expression, Typing::Target, depth)
    }
    fn render_expression(
        &self,
        expression: &JavaExpr,
        typing: Typing,
        depth: usize,
    ) -> Result<String, JavaPrintError> {
        let mut output = String::new();
        self.write_expression(&mut output, expression, typing, depth)?;
        Ok(output)
    }
    pub(super) fn write_expression(
        &self,
        output: &mut String,
        expression: &JavaExpr,
        typing: Typing,
        depth: usize,
    ) -> Result<(), JavaPrintError> {
        use Requirement::{Above, Any, AtLeast, BinaryRight};
        use Token::{Text, Visit};
        use Typing::{Standalone, Target};
        let mut pending: Pending = smallvec::smallvec![Visit(expression, Any, typing)];
        while let Some(task) = pending.pop() {
            let (expr, requirement, typing) = match task {
                Visit(expr, req, typing) => (expr, req, typing),
                Text(text) => {
                    output.push_str(text);
                    continue;
                }
                Token::Infix(op) => {
                    output.push(' ');
                    output.push_str(op);
                    output.push(' ');
                    continue;
                }
                Token::Name(name) => {
                    write!(output, "{name}")?;
                    continue;
                }
                Token::Type(ty) => {
                    write!(output, "{ty}")?;
                    continue;
                }
                Token::Target(method) => {
                    write!(output, "{method}")?;
                    continue;
                }
                Token::Method(types, name) => {
                    if !types.is_empty() {
                        output.push('<');
                        for (i, ty) in types.iter().enumerate() {
                            if i > 0 {
                                output.push_str(", ");
                            }
                            write!(output, "{ty}")?;
                        }
                        output.push('>');
                    }
                    write!(output, "{name}(")?;
                    continue;
                }
                Token::NewType(ty, enclosed, diamond) => {
                    if enclosed || diamond {
                        if let JavaType::Class(class) = ty {
                            if let Some((member, parents)) = class.segments.split_last() {
                                output.push_str(if enclosed { ".new " } else { "new " });
                                if !enclosed {
                                    for parent in parents {
                                        write!(output, "{parent}.")?;
                                    }
                                }
                                if diamond {
                                    write!(output, "{}<>", member.name)?;
                                } else {
                                    write!(output, "{member}")?;
                                }
                            }
                        }
                    } else {
                        write!(output, "new {ty}")?;
                    }
                    output.push('(');
                    continue;
                }
                Token::ArrayRank(rank) => {
                    for _ in 0..rank {
                        output.push_str("[]");
                    }
                    continue;
                }
                Token::Body(body) => {
                    output.push(' ');
                    output.push_str(&body);
                    continue;
                }
                Token::InvalidConstruction => return Err(JavaPrintError::MalformedExpression),
            };
            if requirement.parentheses(expr) {
                output.push('(');
                pending.push(Text(")"));
            }
            match expr {
                JavaExpr::This => output.push_str("this"),
                JavaExpr::Super => output.push_str("super"),
                JavaExpr::QualifiedThis(ty) => write!(output, "{ty}.this")?,
                JavaExpr::Name(name) => write!(output, "{name}")?,
                JavaExpr::Literal(value) => output.push_str(&JavaLiterals::render(value)),
                JavaExpr::ClassLiteral(ty) => write!(output, "{ty}.class")?,
                JavaExpr::Allocate(ty) => write!(output, "allocate-instance {ty}")?,
                JavaExpr::Unavailable { reason, operands } => {
                    let reason = reason.escape_debug().to_string().replace("*/", "* /");
                    write!(output, "/* Diana: {reason} */ <unavailable>(")?;
                    pending.push(Text(")"));
                    sequence(&mut pending, operands, Standalone);
                }
                JavaExpr::RecordOperation { site, operands } => {
                    write!(
                        output,
                        "{}<{}>[",
                        site.operation.mnemonic(),
                        site.owner.to_descriptor().escape_debug()
                    )?;
                    for (index, component) in site.components.iter().enumerate() {
                        if index != 0 {
                            output.push_str(", ");
                        }
                        write!(output, "getfield {}", component.to_string().escape_debug())?;
                    }
                    output.push_str("](");
                    pending.push(Text(")"));
                    sequence(&mut pending, operands, Standalone);
                }
                JavaExpr::DirectInit {
                    receiver,
                    target,
                    args,
                } => {
                    output.push_str("init-direct ");
                    pending.push(Text("]"));
                    sequence(&mut pending, args, Standalone);
                    pending.push(Text(", ["));
                    pending.push(Token::Target(target));
                    pending.push(Text(", "));
                    pending.push(Visit(receiver, Any, Standalone));
                }
                JavaExpr::StaticField { owner, name } => {
                    Self::write_raw_type(output, owner)?;
                    write!(output, ".{name}")?;
                }
                JavaExpr::Field { owner, name } => {
                    pending.push(Token::Name(name));
                    pending.push(Text("."));
                    pending.push(Visit(owner, AtLeast(Precedence::Primary), Standalone));
                }
                JavaExpr::ArrayAccess { array, index } => {
                    pending.push(Text("]"));
                    pending.push(Visit(index, Any, Standalone));
                    pending.push(Text("["));
                    pending.push(Visit(array, AtLeast(Precedence::Primary), Standalone));
                }
                JavaExpr::Call {
                    receiver,
                    owner,
                    type_arguments,
                    method,
                    args,
                } => {
                    pending.push(Text(")"));
                    sequence(&mut pending, args, Target);
                    pending.push(Token::Method(type_arguments, method));
                    if let Some(receiver) = receiver {
                        pending.push(Text("."));
                        pending.push(Visit(receiver, AtLeast(Precedence::Primary), Standalone));
                    } else if let Some(owner) = owner {
                        Self::write_raw_type(output, owner)?;
                        output.push('.');
                    }
                }
                JavaExpr::MethodReference { receiver, method } => {
                    pending.push(Token::Name(method));
                    pending.push(Text("::"));
                    pending.push(Visit(receiver, AtLeast(Precedence::Primary), Standalone));
                }
                JavaExpr::Lambda { parameters, body } => {
                    output.push_str(&Self::lambda_parameters(parameters));
                    output.push_str(" -> ");
                    pending.push(Visit(body, Any, Target));
                }
                JavaExpr::BlockLambda { parameters, body } => {
                    output.push_str(&Self::lambda_parameters(parameters));
                    output.push_str(" -> ");
                    let mut nested = String::new();
                    self.render(&mut nested, vec![PrintTask::Statement(body, depth)])?;
                    output.push_str(nested.trim_end());
                }
                JavaExpr::New {
                    enclosing,
                    ty,
                    target_type,
                    args,
                    anonymous_body,
                } => {
                    let member = match ty {
                        JavaType::Class(class) => class.segments.last(),
                        _ => None,
                    };
                    let diamond = typing == Target
                        && target_type.is_some()
                        && anonymous_body.is_none()
                        && member.is_some_and(|segment| !segment.arguments.is_empty());
                    // Anonymous declaration recovery precedes argument recovery. The
                    // qualified type check follows it, as in declaration lowering.
                    if let Some(body) = anonymous_body {
                        pending.push(Token::Body(self.print_anonymous_body(body, depth)?));
                    }
                    if enclosing.is_some() && member.is_none() {
                        pending.push(Token::InvalidConstruction);
                    }
                    pending.push(Text(")"));
                    sequence(&mut pending, args, Target);
                    pending.push(Token::NewType(ty, enclosing.is_some(), diamond));
                    if let Some(owner) = enclosing {
                        pending.push(Visit(owner, AtLeast(Precedence::Primary), Standalone));
                    }
                }
                JavaExpr::NewArray {
                    element_type,
                    dimensions,
                    initializer,
                } => {
                    let (base, rank) = element_type.array_shape();
                    write!(output, "new {base}")?;
                    if !initializer.is_empty() {
                        pending.push(Text(" }"));
                        sequence(&mut pending, initializer, Target);
                        pending.push(Text(" { "));
                    }
                    pending.push(Token::ArrayRank(rank));
                    if dimensions.is_empty() {
                        output.push_str("[]");
                    }
                    for dim in dimensions.iter().rev() {
                        pending.push(Text("]"));
                        pending.push(Visit(dim, Any, Standalone));
                        pending.push(Text("["));
                    }
                }
                JavaExpr::Unary { op, operand } => {
                    output.push_str(op.token());
                    pending.push(Visit(operand, Above(Precedence::Unary), Standalone));
                }
                JavaExpr::Update { op, target, prefix } => {
                    if *prefix {
                        output.push_str(op.token());
                    } else {
                        pending.push(Text(op.token()));
                    }
                    pending.push(Visit(target, AtLeast(Precedence::Primary), Standalone));
                }
                JavaExpr::Binary { left, op, right } => {
                    pending.push(Visit(right, BinaryRight(*op), Standalone));
                    pending.push(Token::Infix(op.token()));
                    if *op == JavaBinaryOp::StringConcat && may_be_constant(left, right) {
                        let mut first = left.as_ref();
                        while let JavaExpr::Binary {
                            left,
                            op: JavaBinaryOp::Add,
                            right,
                        } = first
                        {
                            pending.push(Visit(right, BinaryRight(JavaBinaryOp::Add), Standalone));
                            pending.push(Token::Infix("+"));
                            first = left;
                        }
                        // The first term is a String. These casts preserve its
                        // value and evaluation point while preventing interning.
                        output.push_str("(java.lang.String) (java.lang.Object) ");
                        pending.push(Visit(first, AtLeast(Precedence::Unary), Standalone));
                    } else {
                        pending.push(Visit(left, AtLeast(Precedence::binary(*op)), Standalone));
                    }
                }
                JavaExpr::Cast { ty, value } => {
                    write!(output, "({ty}) ")?;
                    pending.push(Visit(value, AtLeast(Precedence::Unary), Standalone));
                }
                JavaExpr::InstanceOf { value, ty } => {
                    pending.push(Token::Type(ty));
                    pending.push(Text(" instanceof "));
                    pending.push(Visit(value, Above(Precedence::Relational), Standalone));
                }
                JavaExpr::Conditional {
                    condition,
                    when_true,
                    when_false,
                } => {
                    pending.push(Visit(when_false, AtLeast(Precedence::Conditional), typing));
                    pending.push(Text(" : "));
                    pending.push(Visit(when_true, Any, typing));
                    pending.push(Text(" ? "));
                    pending.push(Visit(condition, Above(Precedence::Conditional), Standalone));
                }
                JavaExpr::Assignment { target, op, value } => {
                    pending.push(Visit(value, Any, Target));
                    pending.push(Token::Infix(op.token()));
                    pending.push(Visit(target, Any, Standalone));
                }
            }
        }
        Ok(())
    }
}
