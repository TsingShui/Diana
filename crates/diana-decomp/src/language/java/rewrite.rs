use super::{
    JavaAnnotation, JavaAnnotationValue, JavaAnonymousClassBody, JavaCatch, JavaExpr,
    JavaMethodBody, JavaStmt, JavaTypeDeclaration,
};

/// Structural Java AST rewriting with one post-order expression hook.
///
/// Implementations describe only their semantic rewrite. Child traversal is
/// centralized here so declaration-level recovery never grows its own syntax
/// shape library.
pub trait JavaAstRewriter {
    fn rewrite_nested_functions(&self) -> bool {
        true
    }

    fn finish_expression(&mut self, expression: JavaExpr) -> JavaExpr {
        expression
    }

    fn finish_statement(&mut self, statement: JavaStmt) -> JavaStmt {
        statement
    }

    fn finish_anonymous_body(&mut self, _body: &mut JavaAnonymousClassBody) {}

    fn finish_type_declaration(&mut self, _declaration: &mut JavaTypeDeclaration) {}

    fn rewrite_expression(&mut self, expression: JavaExpr) -> JavaExpr {
        let expression = match expression {
            JavaExpr::DirectInit {
                receiver,
                target,
                args,
            } => JavaExpr::DirectInit {
                receiver: rewrite_boxed_expression(self, receiver),
                target,
                args: self.rewrite_expressions(args),
            },
            JavaExpr::Field { owner, name } => JavaExpr::Field {
                owner: rewrite_boxed_expression(self, owner),
                name,
            },
            JavaExpr::ArrayAccess { array, index } => JavaExpr::ArrayAccess {
                array: rewrite_boxed_expression(self, array),
                index: rewrite_boxed_expression(self, index),
            },
            JavaExpr::Call {
                receiver,
                owner,
                type_arguments,
                method,
                args,
            } => JavaExpr::Call {
                receiver: receiver.map(|value| rewrite_boxed_expression(self, value)),
                owner,
                type_arguments,
                method,
                args: self.rewrite_expressions(args),
            },
            JavaExpr::MethodReference { receiver, method } => JavaExpr::MethodReference {
                receiver: rewrite_boxed_expression(self, receiver),
                method,
            },
            JavaExpr::Lambda { parameters, body } if self.rewrite_nested_functions() => {
                JavaExpr::Lambda {
                    parameters,
                    body: rewrite_boxed_expression(self, body),
                }
            }
            JavaExpr::BlockLambda { parameters, body } if self.rewrite_nested_functions() => {
                JavaExpr::BlockLambda {
                    parameters,
                    body: rewrite_boxed_statement(self, body),
                }
            }
            JavaExpr::New {
                enclosing,
                ty,
                target_type,
                args,
                anonymous_body,
            } => JavaExpr::New {
                enclosing: enclosing.map(|value| rewrite_boxed_expression(self, value)),
                ty,
                target_type,
                args: self.rewrite_expressions(args),
                anonymous_body: anonymous_body.map(|mut body| {
                    self.rewrite_anonymous_body(&mut body);
                    body
                }),
            },
            JavaExpr::NewArray {
                element_type,
                dimensions,
                initializer,
            } => JavaExpr::NewArray {
                element_type,
                dimensions: self.rewrite_expressions(dimensions),
                initializer: self.rewrite_expressions(initializer),
            },
            JavaExpr::Unary { op, operand } => JavaExpr::Unary {
                op,
                operand: rewrite_boxed_expression(self, operand),
            },
            JavaExpr::Update { op, target, prefix } => JavaExpr::Update {
                op,
                target: rewrite_boxed_expression(self, target),
                prefix,
            },
            JavaExpr::Binary { left, op, right } => JavaExpr::Binary {
                left: rewrite_boxed_expression(self, left),
                op,
                right: rewrite_boxed_expression(self, right),
            },
            JavaExpr::Cast { ty, value } => JavaExpr::Cast {
                ty,
                value: rewrite_boxed_expression(self, value),
            },
            JavaExpr::InstanceOf { value, ty } => JavaExpr::InstanceOf {
                value: rewrite_boxed_expression(self, value),
                ty,
            },
            JavaExpr::Conditional {
                condition,
                when_true,
                when_false,
            } => JavaExpr::Conditional {
                condition: rewrite_boxed_expression(self, condition),
                when_true: rewrite_boxed_expression(self, when_true),
                when_false: rewrite_boxed_expression(self, when_false),
            },
            JavaExpr::Assignment { target, op, value } => JavaExpr::Assignment {
                target: rewrite_boxed_expression(self, target),
                op,
                value: rewrite_boxed_expression(self, value),
            },
            leaf => leaf,
        };
        self.finish_expression(expression)
    }

    fn rewrite_statement(&mut self, statement: JavaStmt) -> JavaStmt {
        let statement = match statement {
            JavaStmt::Block(statements) => JavaStmt::Block(self.rewrite_statements(statements)),
            JavaStmt::Labeled { label, body } => JavaStmt::Labeled {
                label,
                body: rewrite_boxed_statement(self, body),
            },
            JavaStmt::Variable { ty, name, value } => JavaStmt::Variable {
                ty,
                name,
                value: value.map(|value| self.rewrite_expression(value)),
            },
            JavaStmt::Expression(expression) => {
                JavaStmt::Expression(self.rewrite_expression(expression))
            }
            JavaStmt::ConstructorInvocation { target, args } => JavaStmt::ConstructorInvocation {
                target,
                args: self.rewrite_expressions(args),
            },
            JavaStmt::Assign { target, op, value } => JavaStmt::Assign {
                target: self.rewrite_expression(target),
                op,
                value: self.rewrite_expression(value),
            },
            JavaStmt::If {
                condition,
                then_stmt,
                else_stmt,
            } => JavaStmt::If {
                condition: self.rewrite_expression(condition),
                then_stmt: rewrite_boxed_statement(self, then_stmt),
                else_stmt: else_stmt.map(|statement| rewrite_boxed_statement(self, statement)),
            },
            JavaStmt::While {
                label,
                condition,
                body,
            } => JavaStmt::While {
                label,
                condition: self.rewrite_expression(condition),
                body: rewrite_boxed_statement(self, body),
            },
            JavaStmt::DoWhile {
                label,
                body,
                condition,
            } => JavaStmt::DoWhile {
                label,
                body: rewrite_boxed_statement(self, body),
                condition: self.rewrite_expression(condition),
            },
            JavaStmt::For {
                label,
                init,
                condition,
                update,
                body,
            } => JavaStmt::For {
                label,
                init: self.rewrite_statements(init),
                condition: condition.map(|value| self.rewrite_expression(value)),
                update: self.rewrite_expressions(update),
                body: rewrite_boxed_statement(self, body),
            },
            JavaStmt::ForEach {
                label,
                ty,
                variable,
                iterable,
                body,
            } => JavaStmt::ForEach {
                label,
                ty,
                variable,
                iterable: self.rewrite_expression(iterable),
                body: rewrite_boxed_statement(self, body),
            },
            JavaStmt::Switch {
                label,
                selector,
                mut cases,
            } => {
                for case in &mut cases {
                    case.labels = self.rewrite_expressions(std::mem::take(&mut case.labels));
                    case.body = self.rewrite_statements(std::mem::take(&mut case.body));
                }
                JavaStmt::Switch {
                    label,
                    selector: self.rewrite_expression(selector),
                    cases,
                }
            }
            JavaStmt::Try {
                body,
                mut catches,
                finally,
            } => {
                for JavaCatch { body, .. } in &mut catches {
                    *body = self.rewrite_statement(std::mem::replace(body, JavaStmt::Empty));
                }
                JavaStmt::Try {
                    body: rewrite_boxed_statement(self, body),
                    catches,
                    finally: finally.map(|body| rewrite_boxed_statement(self, body)),
                }
            }
            JavaStmt::Synchronized { lock, body } => JavaStmt::Synchronized {
                lock: self.rewrite_expression(lock),
                body: rewrite_boxed_statement(self, body),
            },
            JavaStmt::Return(value) => {
                JavaStmt::Return(value.map(|value| self.rewrite_expression(value)))
            }
            JavaStmt::Throw(value) => JavaStmt::Throw(self.rewrite_expression(value)),
            leaf => leaf,
        };
        self.finish_statement(statement)
    }

    fn rewrite_body(&mut self, body: &mut JavaMethodBody) {
        body.root = self.rewrite_statement(std::mem::replace(&mut body.root, JavaStmt::Empty));
    }

    fn rewrite_anonymous_body(&mut self, body: &mut JavaAnonymousClassBody) {
        for field in &mut body.fields {
            self.rewrite_annotations(&mut field.annotations);
            field.initializer = field
                .initializer
                .take()
                .map(|value| self.rewrite_expression(value));
        }
        for method in &mut body.methods {
            self.rewrite_annotations(&mut method.annotations);
            for parameter in &mut method.parameters {
                self.rewrite_annotations(&mut parameter.annotations);
            }
            if let Some(body) = &mut method.body {
                self.rewrite_body(body);
            }
        }
        for nested in &mut body.nested {
            self.rewrite_type_declaration(nested);
        }
        self.finish_anonymous_body(body);
    }

    fn rewrite_type_declaration(&mut self, declaration: &mut JavaTypeDeclaration) {
        self.rewrite_annotations(&mut declaration.annotations);
        for constant in &mut declaration.enum_constants {
            self.rewrite_annotations(&mut constant.annotations);
            constant.arguments = self.rewrite_expressions(std::mem::take(&mut constant.arguments));
            if let Some(body) = &mut constant.body {
                self.rewrite_anonymous_body(body);
            }
        }
        for field in &mut declaration.fields {
            self.rewrite_annotations(&mut field.annotations);
            field.initializer = field
                .initializer
                .take()
                .map(|value| self.rewrite_expression(value));
        }
        for method in &mut declaration.methods {
            self.rewrite_annotations(&mut method.annotations);
            for parameter in &mut method.parameters {
                self.rewrite_annotations(&mut parameter.annotations);
            }
            if let Some(body) = &mut method.body {
                self.rewrite_body(body);
            }
        }
        for nested in &mut declaration.nested {
            self.rewrite_type_declaration(nested);
        }
        self.finish_type_declaration(declaration);
    }

    fn rewrite_annotation_value(&mut self, value: JavaAnnotationValue) -> JavaAnnotationValue {
        match value {
            JavaAnnotationValue::Expression(expression) => {
                JavaAnnotationValue::Expression(self.rewrite_expression(expression))
            }
            JavaAnnotationValue::Annotation(mut annotation) => {
                for element in &mut annotation.elements {
                    element.value = self.rewrite_annotation_value(std::mem::replace(
                        &mut element.value,
                        JavaAnnotationValue::Array(Vec::new()),
                    ));
                }
                JavaAnnotationValue::Annotation(annotation)
            }
            JavaAnnotationValue::Array(values) => JavaAnnotationValue::Array(
                values
                    .into_iter()
                    .map(|value| self.rewrite_annotation_value(value))
                    .collect(),
            ),
        }
    }

    fn rewrite_annotations(&mut self, annotations: &mut [JavaAnnotation]) {
        for annotation in annotations {
            for element in &mut annotation.elements {
                element.value = self.rewrite_annotation_value(std::mem::replace(
                    &mut element.value,
                    JavaAnnotationValue::Array(Vec::new()),
                ));
            }
        }
    }

    fn rewrite_expressions(&mut self, expressions: Vec<JavaExpr>) -> Vec<JavaExpr> {
        expressions
            .into_iter()
            .map(|expression| self.rewrite_expression(expression))
            .collect()
    }

    fn rewrite_statements(&mut self, statements: Vec<JavaStmt>) -> Vec<JavaStmt> {
        statements
            .into_iter()
            .map(|statement| self.rewrite_statement(statement))
            .collect()
    }
}

fn rewrite_boxed_expression<R: JavaAstRewriter + ?Sized>(
    rewriter: &mut R,
    mut child: Box<JavaExpr>,
) -> Box<JavaExpr> {
    *child = rewriter.rewrite_expression(std::mem::replace(&mut *child, JavaExpr::This));
    child
}

fn rewrite_boxed_statement<R: JavaAstRewriter + ?Sized>(
    rewriter: &mut R,
    mut child: Box<JavaStmt>,
) -> Box<JavaStmt> {
    *child = rewriter.rewrite_statement(std::mem::replace(&mut *child, JavaStmt::Empty));
    child
}
