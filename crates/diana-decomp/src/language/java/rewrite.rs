use super::{
    JavaAnnotation, JavaAnnotationValue, JavaAnonymousClassBody, JavaExpr, JavaMethodBody,
    JavaStmt, JavaTypeDeclaration,
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

    fn rewrite_expression(&mut self, mut expression: JavaExpr) -> JavaExpr {
        walk_expression(&mut OwnedChildren(self), &mut expression);
        self.finish_expression(expression)
    }

    fn rewrite_statement(&mut self, mut statement: JavaStmt) -> JavaStmt {
        walk_statement(&mut OwnedChildren(self), &mut statement);
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

/// Rewriters that inspect or edit nodes by reference avoid moving every
/// unchanged expression and statement through an owning callback.
pub(crate) trait JavaAstMutator {
    fn after_expression(&mut self, _expression: &mut JavaExpr) {}
    fn after_statement(&mut self, _statement: &mut JavaStmt) {}
    fn after_statements(&mut self, _statements: &mut Vec<JavaStmt>) {}
    fn after_anonymous_body(&mut self, _body: &mut JavaAnonymousClassBody) {}
    fn after_type_declaration(&mut self, _declaration: &mut JavaTypeDeclaration) {}

    fn visit_anonymous(&mut self, body: &mut JavaAnonymousClassBody)
    where
        Self: Sized,
    {
        JavaAstRewriter::rewrite_anonymous_body(self, body);
    }
}

impl<R: JavaAstMutator> JavaAstRewriter for R {
    fn rewrite_expression(&mut self, mut expression: JavaExpr) -> JavaExpr {
        self.expression(&mut expression);
        expression
    }
    fn rewrite_statement(&mut self, mut statement: JavaStmt) -> JavaStmt {
        self.statement(&mut statement);
        statement
    }
    fn rewrite_body(&mut self, body: &mut JavaMethodBody) {
        self.statement(&mut body.root);
    }
    fn rewrite_statements(&mut self, mut values: Vec<JavaStmt>) -> Vec<JavaStmt> {
        self.statements(&mut values);
        values
    }
    fn finish_anonymous_body(&mut self, body: &mut JavaAnonymousClassBody) {
        self.after_anonymous_body(body);
    }
    fn finish_type_declaration(&mut self, declaration: &mut JavaTypeDeclaration) {
        self.after_type_declaration(declaration);
    }
}

trait Children {
    fn expression(&mut self, expression: &mut JavaExpr);
    fn statement(&mut self, statement: &mut JavaStmt);
    fn expressions(&mut self, expressions: &mut Vec<JavaExpr>);
    fn statements(&mut self, statements: &mut Vec<JavaStmt>);
    fn anonymous(&mut self, body: &mut JavaAnonymousClassBody);
    fn nested_functions(&self) -> bool;
}

impl<R: JavaAstMutator> Children for R {
    fn expression(&mut self, expression: &mut JavaExpr) {
        walk_expression(self, expression);
        self.after_expression(expression);
    }
    fn statement(&mut self, statement: &mut JavaStmt) {
        walk_statement(self, statement);
        self.after_statement(statement);
    }
    fn expressions(&mut self, expressions: &mut Vec<JavaExpr>) {
        for value in expressions {
            self.expression(value);
        }
    }
    fn statements(&mut self, statements: &mut Vec<JavaStmt>) {
        for statement in &mut *statements {
            self.statement(statement);
        }
        self.after_statements(statements);
    }
    fn anonymous(&mut self, body: &mut JavaAnonymousClassBody) {
        self.visit_anonymous(body);
    }
    fn nested_functions(&self) -> bool {
        true
    }
}

/// Keep dispatching through the public owning entry points. Implementations
/// can override individual child, list, and nested-function behavior.
struct OwnedChildren<'a, R: ?Sized>(&'a mut R);

impl<R: JavaAstRewriter + ?Sized> Children for OwnedChildren<'_, R> {
    fn expression(&mut self, expression: &mut JavaExpr) {
        *expression = self
            .0
            .rewrite_expression(std::mem::replace(expression, JavaExpr::This));
    }
    fn statement(&mut self, statement: &mut JavaStmt) {
        *statement = self
            .0
            .rewrite_statement(std::mem::replace(statement, JavaStmt::Empty));
    }
    fn expressions(&mut self, expressions: &mut Vec<JavaExpr>) {
        *expressions = self.0.rewrite_expressions(std::mem::take(expressions));
    }
    fn statements(&mut self, statements: &mut Vec<JavaStmt>) {
        *statements = self.0.rewrite_statements(std::mem::take(statements));
    }
    fn anonymous(&mut self, body: &mut JavaAnonymousClassBody) {
        self.0.rewrite_anonymous_body(body);
    }
    fn nested_functions(&self) -> bool {
        self.0.rewrite_nested_functions()
    }
}

fn walk_expression(visitor: &mut impl Children, expression: &mut JavaExpr) {
    match expression {
        JavaExpr::DirectInit { receiver, args, .. } => {
            visitor.expression(receiver);
            visitor.expressions(args);
        }
        JavaExpr::Field { owner, .. }
        | JavaExpr::MethodReference {
            receiver: owner, ..
        } => visitor.expression(owner),
        JavaExpr::ArrayAccess { array, index } => {
            visitor.expression(array);
            visitor.expression(index);
        }
        JavaExpr::Call { receiver, args, .. } => {
            if let Some(receiver) = receiver {
                visitor.expression(receiver);
            }
            visitor.expressions(args);
        }
        JavaExpr::Lambda { body, .. } if visitor.nested_functions() => visitor.expression(body),
        JavaExpr::BlockLambda { body, .. } if visitor.nested_functions() => visitor.statement(body),
        JavaExpr::New {
            enclosing,
            args,
            anonymous_body,
            ..
        } => {
            if let Some(enclosing) = enclosing {
                visitor.expression(enclosing);
            }
            visitor.expressions(args);
            if let Some(body) = anonymous_body {
                visitor.anonymous(body);
            }
        }
        JavaExpr::NewArray {
            dimensions,
            initializer,
            ..
        } => {
            visitor.expressions(dimensions);
            visitor.expressions(initializer);
        }
        JavaExpr::Unary { operand, .. }
        | JavaExpr::Update {
            target: operand, ..
        }
        | JavaExpr::Cast { value: operand, .. }
        | JavaExpr::InstanceOf { value: operand, .. } => visitor.expression(operand),
        JavaExpr::Binary { left, right, .. }
        | JavaExpr::Assignment {
            target: left,
            value: right,
            ..
        } => {
            visitor.expression(left);
            visitor.expression(right);
        }
        JavaExpr::Conditional {
            condition,
            when_true,
            when_false,
        } => {
            visitor.expression(condition);
            visitor.expression(when_true);
            visitor.expression(when_false);
        }
        JavaExpr::This
        | JavaExpr::QualifiedThis(_)
        | JavaExpr::Super
        | JavaExpr::Name(_)
        | JavaExpr::Literal(_)
        | JavaExpr::ClassLiteral(_)
        | JavaExpr::Allocate(_)
        | JavaExpr::StaticField { .. }
        | JavaExpr::Lambda { .. }
        | JavaExpr::BlockLambda { .. } => {}
    }
}

fn walk_statement(visitor: &mut impl Children, statement: &mut JavaStmt) {
    match statement {
        JavaStmt::Block(statements) => visitor.statements(statements),
        JavaStmt::Labeled { body, .. } => visitor.statement(body),
        JavaStmt::Variable { value, .. } | JavaStmt::Return(value) => {
            if let Some(value) = value {
                visitor.expression(value);
            }
        }
        JavaStmt::Expression(value) | JavaStmt::Throw(value) => visitor.expression(value),
        JavaStmt::ConstructorInvocation { args, .. } => visitor.expressions(args),
        JavaStmt::Assign { target, value, .. } => {
            visitor.expression(target);
            visitor.expression(value);
        }
        JavaStmt::If {
            condition,
            then_stmt,
            else_stmt,
        } => {
            visitor.expression(condition);
            visitor.statement(then_stmt);
            if let Some(other) = else_stmt {
                visitor.statement(other);
            }
        }
        JavaStmt::While {
            condition, body, ..
        } => {
            visitor.expression(condition);
            visitor.statement(body);
        }
        JavaStmt::DoWhile {
            condition, body, ..
        } => {
            visitor.statement(body);
            visitor.expression(condition);
        }
        JavaStmt::For {
            init,
            condition,
            update,
            body,
            ..
        } => {
            visitor.statements(init);
            if let Some(condition) = condition {
                visitor.expression(condition);
            }
            visitor.expressions(update);
            visitor.statement(body);
        }
        JavaStmt::ForEach { iterable, body, .. } => {
            visitor.expression(iterable);
            visitor.statement(body);
        }
        JavaStmt::Switch {
            selector, cases, ..
        } => {
            for case in cases {
                visitor.expressions(&mut case.labels);
                visitor.statements(&mut case.body);
            }
            visitor.expression(selector);
        }
        JavaStmt::Try {
            body,
            catches,
            finally,
        } => {
            for catch in catches {
                visitor.statement(&mut catch.body);
            }
            visitor.statement(body);
            if let Some(finally) = finally {
                visitor.statement(finally);
            }
        }
        JavaStmt::Synchronized { lock, body, .. } => {
            visitor.expression(lock);
            visitor.statement(body);
        }
        JavaStmt::Empty
        | JavaStmt::RecoveryFailure(_)
        | JavaStmt::Break(_)
        | JavaStmt::Continue(_) => {}
    }
}

#[cfg(test)]
#[path = "../../../tests/internal/java_rewriter_callbacks.rs"]
mod callback_tests;
