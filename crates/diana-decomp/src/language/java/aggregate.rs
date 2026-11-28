use super::{
    JavaAssignOp, JavaAstMutator, JavaAstRewriter, JavaAstTransform, JavaBinaryOp, JavaExpr,
    JavaIdentifier, JavaLiteral, JavaMethodBody, JavaStmt,
};

#[derive(Debug, Default)]
pub struct AggregateInitializer {
    changed: bool,
}

impl AggregateInitializer {
    fn combine(&mut self, statements: Vec<JavaStmt>) -> Vec<JavaStmt> {
        if !statements.windows(2).any(|pair| {
            matches!(
                pair[0],
                JavaStmt::Variable {
                    value: Some(JavaExpr::NewArray { .. } | JavaExpr::Cast { .. }),
                    ..
                }
            ) && matches!(
                pair[1],
                JavaStmt::Assign {
                    op: JavaAssignOp::Assign,
                    ..
                } | JavaStmt::If {
                    else_stmt: None,
                    ..
                }
            )
        }) {
            return statements;
        }

        let mut output = Vec::with_capacity(statements.len());
        let mut pending = statements.into_iter();
        while let Some(mut statement) = pending.next() {
            if let JavaStmt::Variable {
                name,
                value: Some(value),
                ..
            } = &mut statement
            {
                self.combine_writes(name, value, &mut pending);
                if matches!(&*value, JavaExpr::NewArray { dimensions, initializer, .. }
                    if dimensions.is_empty() && !initializer.is_empty())
                {
                    if let Some(JavaStmt::Assign {
                        target: JavaExpr::StaticField { .. },
                        op: JavaAssignOp::Assign,
                        value: source,
                    }) = pending.as_mut_slice().first_mut()
                    {
                        if matches!(&*source, JavaExpr::Name(source) if source == name) {
                            *source = std::mem::replace(value, JavaExpr::This);
                            self.changed = true;
                            continue;
                        }
                    }
                }
            }
            output.push(statement);
        }
        output
    }

    fn combine_writes(
        &mut self,
        name: &super::JavaIdentifier,
        mut expression: &mut JavaExpr,
        pending: &mut std::vec::IntoIter<JavaStmt>,
    ) {
        while let JavaExpr::Cast { value, .. } = expression {
            expression = value;
        }
        let JavaExpr::NewArray {
            dimensions,
            initializer,
            ..
        } = expression
        else {
            return;
        };
        let [JavaExpr::Literal(JavaLiteral::Integer(length))] = dimensions.as_slice() else {
            return;
        };
        let Ok(length) = usize::try_from(*length) else {
            return;
        };
        if length == 0 || !initializer.is_empty() {
            return;
        }
        // A DEX dimension can be far larger than the statement list. Only
        // complete, consecutively indexed stores are an aggregate initializer.
        let after_declaration = pending.as_slice();
        let skip_guard = after_declaration
            .first()
            .is_some_and(|statement| Self::redundant_length_guard(statement, name, length));
        let first_write = usize::from(skip_guard);
        let Some(writes) = after_declaration
            .get(first_write..)
            .and_then(|following| following.get(..length))
        else {
            return;
        };
        let complete = writes.iter().enumerate().all(|(index, write)| {
            let JavaStmt::Assign {
                target: JavaExpr::ArrayAccess { array, index: subscript },
                op: JavaAssignOp::Assign, value,
            } = write else { return false };
            matches!(array.as_ref(), JavaExpr::Name(array) if array == name)
                && matches!(subscript.as_ref(), JavaExpr::Literal(JavaLiteral::Integer(i)) if usize::try_from(*i) == Ok(index))
                && !ExpressionNameUse::contains(value, name)
        });
        if complete {
            if skip_guard {
                pending.next();
            }
            dimensions.clear();
            initializer.extend(pending.take(length).map(|statement| match statement {
                JavaStmt::Assign { value, .. } => value,
                _ => unreachable!("checked consecutive array assignments"),
            }));
            self.changed = true;
        }
    }

    fn redundant_length_guard(statement: &JavaStmt, array: &JavaIdentifier, length: usize) -> bool {
        let JavaStmt::If {
            condition:
                JavaExpr::Binary {
                    left,
                    op: JavaBinaryOp::Less,
                    right,
                },
            else_stmt: None,
            ..
        } = statement
        else {
            return false;
        };
        matches!(
            (left.as_ref(), right.as_ref()),
            (JavaExpr::Field { owner, name }, JavaExpr::Literal(JavaLiteral::Integer(required)))
                if name.as_str() == "length"
                    && matches!(owner.as_ref(), JavaExpr::Name(name) if name == array)
                    && usize::try_from(*required).is_ok_and(|required| required <= length)
        )
    }
}

struct ExpressionNameUse<'a> {
    target: &'a super::JavaIdentifier,
    found: bool,
}

impl ExpressionNameUse<'_> {
    fn contains(expression: &JavaExpr, target: &super::JavaIdentifier) -> bool {
        let mut query = ExpressionNameUse {
            target,
            found: false,
        };
        query.rewrite_expression(expression.clone());
        query.found
    }
}

impl JavaAstRewriter for ExpressionNameUse<'_> {
    fn finish_expression(&mut self, expression: JavaExpr) -> JavaExpr {
        if matches!(&expression, JavaExpr::Name(name) if name == self.target) {
            self.found = true;
        }
        expression
    }
}

impl JavaAstMutator for AggregateInitializer {
    fn after_statements(&mut self, statements: &mut Vec<JavaStmt>) {
        *statements = self.combine(std::mem::take(statements));
    }
}

impl JavaAstTransform for AggregateInitializer {
    type Error = super::JavaStructuralError;

    fn apply(&mut self, body: &mut JavaMethodBody) -> Result<bool, Self::Error> {
        self.changed = false;
        self.rewrite_body(body);
        Ok(self.changed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::language::java::{JavaIdentifier, JavaPrimitiveType, JavaType};

    #[test]
    fn combines_complete_array_writes() {
        let name = JavaIdentifier::from_dex("values");
        let array = || JavaExpr::ArrayAccess {
            array: Box::new(JavaExpr::Name(name.clone())),
            index: Box::new(JavaExpr::Literal(JavaLiteral::Integer(0))),
        };
        let mut body = JavaMethodBody {
            root: JavaStmt::Block(vec![
                JavaStmt::Variable {
                    ty: JavaType::array(JavaType::Primitive(JavaPrimitiveType::Int)),
                    name: name.clone(),
                    value: Some(JavaExpr::NewArray {
                        element_type: JavaType::Primitive(JavaPrimitiveType::Int),
                        dimensions: vec![JavaExpr::Literal(JavaLiteral::Integer(1))],
                        initializer: Vec::new(),
                    }),
                },
                JavaStmt::Assign {
                    target: array(),
                    op: JavaAssignOp::Assign,
                    value: JavaExpr::Literal(JavaLiteral::Integer(4)),
                },
            ]),
        };

        assert!(AggregateInitializer::default().apply(&mut body).unwrap());
        let JavaStmt::Block(statements) = body.root else {
            panic!("expected block");
        };
        assert!(matches!(
            statements.as_slice(),
            [JavaStmt::Variable {
                value: Some(JavaExpr::NewArray {
                    dimensions,
                    initializer,
                    ..
                }),
                ..
            }] if dimensions.is_empty() && initializer.len() == 1
        ));
    }

    #[test]
    fn preserves_self_referential_array_writes() {
        let name = JavaIdentifier::from_dex("values");
        let array_type = JavaType::array(JavaType::source_class("java.lang.Object"));
        let mut body = JavaMethodBody {
            root: JavaStmt::Block(vec![
                JavaStmt::Variable {
                    ty: array_type,
                    name: name.clone(),
                    value: Some(JavaExpr::NewArray {
                        element_type: JavaType::source_class("java.lang.Object"),
                        dimensions: vec![JavaExpr::Literal(JavaLiteral::Integer(1))],
                        initializer: Vec::new(),
                    }),
                },
                JavaStmt::Assign {
                    target: JavaExpr::ArrayAccess {
                        array: Box::new(JavaExpr::Name(name.clone())),
                        index: Box::new(JavaExpr::Literal(JavaLiteral::Integer(0))),
                    },
                    op: JavaAssignOp::Assign,
                    value: JavaExpr::Name(name),
                },
            ]),
        };

        assert!(!AggregateInitializer::default().apply(&mut body).unwrap());
        assert!(matches!(body.root, JavaStmt::Block(ref statements) if statements.len() == 2));
    }
}
