use std::fmt::Write;

use expression::Typing;

use super::ast::{
    JavaBinaryOp, JavaCatch, JavaConstructorTarget, JavaExpr, JavaIdentifier, JavaMethodBody,
    JavaStmt, JavaSwitchCase, JavaType,
};
use super::literals::JavaLiterals;
use super::unit::{
    JavaAnnotation, JavaAnnotationValue, JavaAnonymousClassBody, JavaCompilationUnit,
    JavaFieldDeclaration, JavaMethodDeclaration, JavaMethodDeclarationKind, JavaModifier,
    JavaTypeDeclaration, JavaTypeDeclarationKind,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JavaPrintError {
    MalformedExpression,
    MalformedDeclaration,
    InvalidInlineStatement,
    Formatting,
}

impl std::fmt::Display for JavaPrintError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MalformedExpression => formatter.write_str("malformed Java expression tree"),
            Self::MalformedDeclaration => formatter.write_str("malformed Java declaration tree"),
            Self::InvalidInlineStatement => {
                formatter.write_str("for initializer is not an inline Java statement")
            }
            Self::Formatting => formatter.write_str("failed to format Java source"),
        }
    }
}

impl std::error::Error for JavaPrintError {}

impl From<std::fmt::Error> for JavaPrintError {
    fn from(_: std::fmt::Error) -> Self {
        Self::Formatting
    }
}

#[derive(Debug, Clone)]
pub struct JavaPrinter {
    indent: String,
    preserve_dex_names: bool,
}

impl Default for JavaPrinter {
    fn default() -> Self {
        Self {
            indent: "    ".to_string(),
            preserve_dex_names: false,
        }
    }
}

impl JavaPrinter {
    pub fn new(indent: impl Into<String>) -> Self {
        Self {
            indent: indent.into(),
            preserve_dex_names: false,
        }
    }

    pub fn with_preserved_dex_names(mut self, preserve: bool) -> Self {
        self.preserve_dex_names = preserve;
        self
    }

    pub fn print_method_body(&self, body: &JavaMethodBody) -> Result<String, JavaPrintError> {
        super::ast::with_preserved_dex_names(self.preserve_dex_names, || {
            let mut output = String::new();
            self.render(&mut output, vec![PrintTask::Statement(&body.root, 0)])?;
            Ok(output)
        })
    }

    pub fn print_method_body_at(
        &self,
        body: &JavaMethodBody,
        depth: usize,
    ) -> Result<String, JavaPrintError> {
        super::ast::with_preserved_dex_names(self.preserve_dex_names, || {
            let mut output = String::new();
            let tasks = match &body.root {
                JavaStmt::Block(statements) => statements
                    .iter()
                    .rev()
                    .map(|statement| PrintTask::Statement(statement, depth))
                    .collect(),
                statement => vec![PrintTask::Statement(statement, depth)],
            };
            self.render(&mut output, tasks)?;
            Ok(output)
        })
    }

    pub fn print_compilation_unit(
        &self,
        unit: &JavaCompilationUnit,
    ) -> Result<String, JavaPrintError> {
        super::ast::with_preserved_dex_names(self.preserve_dex_names, || {
            let mut output = String::new();
            if let Some(package) = &unit.package {
                writeln!(output, "package {package};\n")?;
            }
            if !unit.imports.is_empty() {
                for import in &unit.imports {
                    writeln!(output, "import {import};")?;
                }
                writeln!(output)?;
            }
            self.print_type_tree(&mut output, &unit.declaration)?;
            Ok(output)
        })
    }

    pub fn print_method_declaration(
        &self,
        declaration: &JavaMethodDeclaration,
        depth: usize,
    ) -> Result<String, JavaPrintError> {
        super::ast::with_preserved_dex_names(self.preserve_dex_names, || {
            let mut output = String::new();
            self.print_method_into(&mut output, declaration, depth)?;
            Ok(output)
        })
    }

    fn print_type_tree(
        &self,
        output: &mut String,
        root: &JavaTypeDeclaration,
    ) -> Result<(), JavaPrintError> {
        self.print_type_subtree(output, root, 0)
    }

    fn print_anonymous_body(
        &self,
        body: &JavaAnonymousClassBody,
        depth: usize,
    ) -> Result<String, JavaPrintError> {
        let mut output = String::from("{\n");
        let mut wrote_member = false;
        for field in &body.fields {
            if wrote_member {
                writeln!(output)?;
            }
            self.print_field_into(&mut output, field, depth + 1)?;
            wrote_member = true;
        }
        for method in &body.methods {
            if wrote_member {
                writeln!(output)?;
            }
            self.print_method_into(&mut output, method, depth + 1)?;
            wrote_member = true;
        }
        for declaration in &body.nested {
            if wrote_member {
                writeln!(output)?;
            }
            self.print_type_subtree(&mut output, declaration, depth + 1)?;
            wrote_member = true;
        }
        self.indent(&mut output, depth);
        output.push('}');
        Ok(output)
    }

    fn print_type_subtree(
        &self,
        output: &mut String,
        root: &JavaTypeDeclaration,
        root_depth: usize,
    ) -> Result<(), JavaPrintError> {
        let mut pending = vec![TypePrintTask::Declaration {
            declaration: root,
            depth: root_depth,
            leading_blank: false,
        }];
        while let Some(task) = pending.pop() {
            match task {
                TypePrintTask::Declaration {
                    declaration,
                    depth,
                    leading_blank,
                } => {
                    if leading_blank {
                        writeln!(output)?;
                    }
                    let has_members =
                        self.print_type_declaration_into(output, declaration, depth)?;
                    pending.push(TypePrintTask::End { depth });
                    pending.extend(declaration.nested.iter().enumerate().rev().map(
                        |(index, nested)| TypePrintTask::Declaration {
                            declaration: nested,
                            depth: depth + 1,
                            leading_blank: has_members || index != 0,
                        },
                    ));
                }
                TypePrintTask::End { depth } => {
                    self.indent(output, depth);
                    writeln!(output, "}}")?;
                }
            }
        }
        Ok(())
    }

    fn print_type_declaration_into(
        &self,
        output: &mut String,
        declaration: &JavaTypeDeclaration,
        depth: usize,
    ) -> Result<bool, JavaPrintError> {
        self.annotations(output, &declaration.annotations, depth)?;
        self.indent(output, depth);
        self.modifiers(output, &declaration.modifiers)?;
        write!(
            output,
            "{} {}{}",
            declaration.kind.token(),
            declaration.name,
            Self::type_parameters(&declaration.type_parameters)
        )?;
        if let Some(extends) = &declaration.extends {
            write!(output, " extends {extends}")?;
        }
        if !declaration.implements.is_empty() {
            let relation = if declaration.kind.is_interface() {
                "extends"
            } else {
                "implements"
            };
            write!(
                output,
                " {relation} {}",
                declaration
                    .implements
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            )?;
        }
        writeln!(output, " {{")?;

        let mut wrote_member = false;
        for (index, constant) in declaration.enum_constants.iter().enumerate() {
            if index != 0 {
                writeln!(output, ",")?;
            }
            self.annotations(output, &constant.annotations, depth + 1)?;
            self.indent(output, depth + 1);
            write!(output, "{}", constant.name)?;
            if !constant.arguments.is_empty() {
                write!(
                    output,
                    "({})",
                    constant
                        .arguments
                        .iter()
                        .map(|argument| self.target_typed_expression_at(argument, depth + 1))
                        .collect::<Result<Vec<_>, _>>()?
                        .join(", ")
                )?;
            }
            if let Some(body) = &constant.body {
                write!(output, " {}", self.print_anonymous_body(body, depth + 1)?)?;
            }
        }
        if !declaration.enum_constants.is_empty() {
            writeln!(output, ";")?;
            wrote_member = true;
        } else if declaration.kind == JavaTypeDeclarationKind::Enum
            && (!declaration.fields.is_empty()
                || !declaration.methods.is_empty()
                || !declaration.nested.is_empty())
        {
            // An enum with no recovered constants still needs the separator
            // before ordinary members.
            self.indent(output, depth + 1);
            writeln!(output, ";")?;
            wrote_member = true;
        }
        for field in &declaration.fields {
            if wrote_member {
                writeln!(output)?;
            }
            self.print_field_into(output, field, depth + 1)?;
            wrote_member = true;
        }
        for method in &declaration.methods {
            if wrote_member {
                writeln!(output)?;
            }
            self.print_method_into(output, method, depth + 1)?;
            wrote_member = true;
        }
        Ok(wrote_member)
    }

    fn print_field_into(
        &self,
        output: &mut String,
        field: &JavaFieldDeclaration,
        depth: usize,
    ) -> Result<(), JavaPrintError> {
        self.annotations(output, &field.annotations, depth)?;
        self.indent(output, depth);
        self.modifiers(output, &field.modifiers)?;
        write!(output, "{} {}", field.ty, field.name)?;
        if let Some(initializer) = &field.initializer {
            output.push_str(" = ");
            self.write_expression(output, initializer, Typing::Target, depth)?;
        }
        writeln!(output, ";")?;
        Ok(())
    }

    fn print_method_into(
        &self,
        output: &mut String,
        method: &JavaMethodDeclaration,
        depth: usize,
    ) -> Result<(), JavaPrintError> {
        self.annotations(output, &method.annotations, depth)?;
        self.indent(output, depth);
        if method.kind == JavaMethodDeclarationKind::ClassInitializer {
            write!(output, "static")?;
        } else {
            self.modifiers(output, &method.modifiers)?;
            let parameters = method
                .parameters
                .iter()
                .map(|parameter| {
                    let annotations = parameter
                        .annotations
                        .iter()
                        .map(|annotation| self.annotation(annotation))
                        .collect::<Result<Vec<_>, _>>()?;
                    let prefix = if annotations.is_empty() {
                        String::new()
                    } else {
                        format!("{} ", annotations.join(" "))
                    };
                    let ty = if parameter.varargs {
                        match &parameter.ty {
                            JavaType::Array(element) => format!("{element}..."),
                            ty => ty.to_string(),
                        }
                    } else {
                        parameter.ty.to_string()
                    };
                    Ok(format!("{prefix}{ty} {}", parameter.name))
                })
                .collect::<Result<Vec<_>, JavaPrintError>>()?
                .join(", ");
            match method.kind {
                JavaMethodDeclarationKind::Method => {
                    let return_type = method
                        .return_type
                        .as_ref()
                        .ok_or(JavaPrintError::MalformedDeclaration)?;
                    let name = method
                        .name
                        .as_ref()
                        .ok_or(JavaPrintError::MalformedDeclaration)?;
                    write!(
                        output,
                        "{}{} {name}({parameters})",
                        Self::type_parameters_prefix(&method.type_parameters),
                        return_type,
                    )?;
                }
                JavaMethodDeclarationKind::Constructor => {
                    let name = method
                        .name
                        .as_ref()
                        .ok_or(JavaPrintError::MalformedDeclaration)?;
                    write!(
                        output,
                        "{}{name}({parameters})",
                        Self::type_parameters_prefix(&method.type_parameters)
                    )?;
                }
                JavaMethodDeclarationKind::ClassInitializer => {
                    return Err(JavaPrintError::MalformedDeclaration);
                }
            }
            if !method.throws.is_empty() {
                write!(
                    output,
                    " throws {}",
                    method
                        .throws
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                )?;
            }
        }
        let Some(body) = &method.body else {
            writeln!(output, ";")?;
            return Ok(());
        };
        writeln!(output, " {{")?;
        output.push_str(&self.print_method_body_at(body, depth + 1)?);
        self.indent(output, depth);
        writeln!(output, "}}")?;
        Ok(())
    }

    fn annotations(
        &self,
        output: &mut String,
        annotations: &[JavaAnnotation],
        depth: usize,
    ) -> Result<(), JavaPrintError> {
        for annotation in annotations {
            self.indent(output, depth);
            writeln!(output, "{}", self.annotation(annotation)?)?;
        }
        Ok(())
    }

    fn annotation(&self, annotation: &JavaAnnotation) -> Result<String, JavaPrintError> {
        let mut output = format!("@{}", annotation.ty);
        if annotation.elements.is_empty() {
            return Ok(output);
        }
        let implicit_value =
            annotation.elements.len() == 1 && annotation.elements[0].name.as_str() == "value";
        let elements = annotation
            .elements
            .iter()
            .map(|element| {
                let value = self.annotation_value(&element.value)?;
                Ok(if implicit_value {
                    value
                } else {
                    format!("{} = {value}", element.name)
                })
            })
            .collect::<Result<Vec<_>, JavaPrintError>>()?;
        write!(output, "({})", elements.join(", "))?;
        Ok(output)
    }

    fn annotation_value(&self, value: &JavaAnnotationValue) -> Result<String, JavaPrintError> {
        match value {
            JavaAnnotationValue::Expression(expression) => {
                self.target_typed_expression_at(expression, 0)
            }
            JavaAnnotationValue::Annotation(annotation) => self.annotation(annotation),
            JavaAnnotationValue::Array(values) => Ok(format!(
                "{{{}}}",
                values
                    .iter()
                    .map(|value| self.annotation_value(value))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ")
            )),
        }
    }

    fn modifiers(
        &self,
        output: &mut String,
        modifiers: &[JavaModifier],
    ) -> Result<(), JavaPrintError> {
        for modifier in modifiers {
            write!(output, "{} ", modifier.token())?;
        }
        Ok(())
    }

    fn type_parameters(parameters: &[super::JavaTypeParameter]) -> String {
        if parameters.is_empty() {
            return String::new();
        }
        format!(
            "<{}>",
            parameters
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        )
    }

    fn type_parameters_prefix(parameters: &[super::JavaTypeParameter]) -> String {
        let parameters = Self::type_parameters(parameters);
        if parameters.is_empty() {
            String::new()
        } else {
            format!("{parameters} ")
        }
    }

    fn render<'a>(
        &self,
        output: &mut String,
        mut pending: Vec<PrintTask<'a>>,
    ) -> Result<(), JavaPrintError> {
        while let Some(task) = pending.pop() {
            match task {
                PrintTask::Statement(statement, depth) => {
                    self.schedule_statement(output, statement, depth, &mut pending)?
                }
                PrintTask::ControlBody(body, depth) => {
                    if matches!(body, JavaStmt::Block(_)) {
                        pending.push(PrintTask::Statement(body, depth));
                    } else {
                        writeln!(output, "{{")?;
                        pending.push(PrintTask::CloseBrace(depth));
                        pending.push(PrintTask::Statement(body, depth + 1));
                    }
                }
                PrintTask::CloseBrace(depth) => {
                    self.indent(output, depth);
                    writeln!(output, "}}")?;
                }
                PrintTask::Else(body, depth) => {
                    Self::continue_clause(output, "else ")?;
                    pending.push(PrintTask::ControlBody(body, depth));
                }
                PrintTask::DoWhileTail(condition, depth) => {
                    Self::continue_clause(output, "")?;
                    output.push_str("while (");
                    self.write_expression(output, condition, Typing::Standalone, depth)?;
                    output.push_str(");\n");
                }
                PrintTask::Catch(catch, depth) => {
                    let types = catch
                        .types
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(" | ");
                    Self::continue_clause(output, &format!("catch ({types} {}) ", catch.variable))?;
                    pending.push(PrintTask::ControlBody(&catch.body, depth));
                }
                PrintTask::Finally(body, depth) => {
                    Self::continue_clause(output, "finally ")?;
                    pending.push(PrintTask::ControlBody(body, depth));
                }
                PrintTask::SwitchCase(case, depth) => {
                    for label in &case.labels {
                        self.indent(output, depth);
                        output.push_str("case ");
                        self.write_expression(output, label, Typing::Standalone, depth)?;
                        output.push_str(":\n");
                    }
                    if case.is_default {
                        self.indent(output, depth);
                        writeln!(output, "default:")?;
                    }
                    pending.extend(
                        case.body
                            .iter()
                            .rev()
                            .map(|statement| PrintTask::Statement(statement, depth + 1)),
                    );
                }
            }
        }
        Ok(())
    }

    fn continue_clause(output: &mut String, clause: &str) -> Result<(), JavaPrintError> {
        if output.ends_with('\n') {
            output.pop();
        }
        write!(output, " {clause}")?;
        Ok(())
    }

    fn schedule_statement<'a>(
        &self,
        output: &mut String,
        statement: &'a JavaStmt,
        depth: usize,
        pending: &mut Vec<PrintTask<'a>>,
    ) -> Result<(), JavaPrintError> {
        match statement {
            JavaStmt::Empty => {}
            JavaStmt::RecoveryFailure(reason) => {
                self.indent(output, depth);
                writeln!(
                    output,
                    "// Diana: WARNING: Failed to decompile this method. Please report an issue at https://github.com/TsingShui/Diana/issues"
                )?;
                self.indent(output, depth);
                writeln!(output, "// Diana: {}", reason.escape_debug())?;
            }
            JavaStmt::Block(statements) => {
                writeln!(output, "{{")?;
                pending.push(PrintTask::CloseBrace(depth));
                pending.extend(
                    statements
                        .iter()
                        .rev()
                        .map(|statement| PrintTask::Statement(statement, depth + 1)),
                );
            }
            JavaStmt::Labeled { label, body } => {
                self.indent(output, depth);
                write!(output, "{label}: ")?;
                match body.as_ref() {
                    JavaStmt::Block(statements) => {
                        writeln!(output, "{{")?;
                        pending.push(PrintTask::CloseBrace(depth));
                        pending.extend(
                            statements
                                .iter()
                                .rev()
                                .map(|statement| PrintTask::Statement(statement, depth + 1)),
                        );
                    }
                    statement => {
                        writeln!(output)?;
                        pending.push(PrintTask::Statement(statement, depth + 1));
                    }
                }
            }
            JavaStmt::Variable { ty, name, value } => {
                self.indent(output, depth);
                write!(output, "{ty} {name}")?;
                if let Some(value) = value {
                    output.push_str(" = ");
                    self.write_expression(output, value, Typing::Target, depth)?;
                }
                writeln!(output, ";")?;
            }
            JavaStmt::Expression(expression) => {
                self.indent(output, depth);
                self.write_expression(output, expression, Typing::Standalone, depth)?;
                output.push_str(";\n");
            }
            JavaStmt::ConstructorInvocation { target, args } => {
                self.indent(output, depth);
                let target = match target {
                    JavaConstructorTarget::This => "this",
                    JavaConstructorTarget::Super => "super",
                };
                write!(output, "{target}(")?;
                for (index, argument) in args.iter().enumerate() {
                    if index > 0 {
                        output.push_str(", ");
                    }
                    self.write_expression(output, argument, Typing::Target, depth)?;
                }
                output.push_str(");\n");
            }
            JavaStmt::Assign { target, op, value } => {
                self.indent(output, depth);
                self.write_expression(output, target, Typing::Standalone, depth)?;
                write!(output, " {} ", op.token())?;
                self.write_expression(output, value, Typing::Target, depth)?;
                output.push_str(";\n");
            }
            JavaStmt::If {
                condition,
                then_stmt,
                else_stmt,
            } => {
                self.indent(output, depth);
                output.push_str("if (");
                self.write_expression(output, condition, Typing::Standalone, depth)?;
                output.push_str(") ");
                if let Some(else_stmt) = else_stmt {
                    pending.push(PrintTask::Else(else_stmt, depth));
                }
                pending.push(PrintTask::ControlBody(then_stmt, depth));
            }
            JavaStmt::While {
                label,
                condition,
                body,
            } => {
                self.label(output, label, depth)?;
                self.indent(output, depth);
                output.push_str("while (");
                self.write_expression(output, condition, Typing::Standalone, depth)?;
                output.push_str(") ");
                pending.push(PrintTask::ControlBody(body, depth));
            }
            JavaStmt::DoWhile {
                label,
                body,
                condition,
            } => {
                self.label(output, label, depth)?;
                self.indent(output, depth);
                write!(output, "do ")?;
                pending.push(PrintTask::DoWhileTail(condition, depth));
                pending.push(PrintTask::ControlBody(body, depth));
            }
            JavaStmt::For {
                label,
                init,
                condition,
                update,
                body,
            } => {
                self.label(output, label, depth)?;
                self.indent(output, depth);
                let init = init
                    .iter()
                    .map(|statement| self.inline_statement(statement, depth))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ");
                let condition = condition
                    .as_ref()
                    .map(|condition| self.expression_at(condition, depth))
                    .transpose()?
                    .unwrap_or_default();
                let update = update
                    .iter()
                    .map(|expression| self.expression_at(expression, depth))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ");
                write!(output, "for ({init}; {condition}; {update}) ")?;
                pending.push(PrintTask::ControlBody(body, depth));
            }
            JavaStmt::ForEach {
                label,
                ty,
                variable,
                iterable,
                body,
            } => {
                self.label(output, label, depth)?;
                self.indent(output, depth);
                write!(
                    output,
                    "for ({} {} : {}) ",
                    ty,
                    variable,
                    self.expression_at(iterable, depth)?
                )?;
                pending.push(PrintTask::ControlBody(body, depth));
            }
            JavaStmt::Switch {
                label,
                selector,
                cases,
            } => {
                self.label(output, label, depth)?;
                self.indent(output, depth);
                writeln!(
                    output,
                    "switch ({}) {{",
                    self.expression_at(selector, depth)?
                )?;
                pending.push(PrintTask::CloseBrace(depth));
                pending.extend(
                    cases
                        .iter()
                        .rev()
                        .map(|case| PrintTask::SwitchCase(case, depth + 1)),
                );
            }
            JavaStmt::Try {
                body,
                catches,
                finally,
            } => {
                self.indent(output, depth);
                write!(output, "try ")?;
                if let Some(finally) = finally {
                    pending.push(PrintTask::Finally(finally, depth));
                }
                pending.extend(
                    catches
                        .iter()
                        .rev()
                        .map(|catch| PrintTask::Catch(catch, depth)),
                );
                pending.push(PrintTask::ControlBody(body, depth));
            }
            JavaStmt::Synchronized { lock, body } => {
                self.indent(output, depth);
                output.push_str("synchronized (");
                self.write_expression(output, lock, Typing::Standalone, depth)?;
                output.push_str(") ");
                pending.push(PrintTask::ControlBody(body, depth));
            }
            JavaStmt::Return(value) => {
                self.indent(output, depth);
                match value {
                    Some(value) => {
                        output.push_str("return ");
                        self.write_expression(output, value, Typing::Target, depth)?;
                        output.push_str(";\n");
                    }
                    None => writeln!(output, "return;")?,
                }
            }
            JavaStmt::Throw(value) => {
                self.indent(output, depth);
                output.push_str("throw ");
                self.write_expression(output, value, Typing::Standalone, depth)?;
                output.push_str(";\n");
            }
            JavaStmt::Break(label) => {
                self.indent(output, depth);
                Self::control(output, "break", label)?;
            }
            JavaStmt::Continue(label) => {
                self.indent(output, depth);
                Self::control(output, "continue", label)?;
            }
        }
        Ok(())
    }

    fn write_raw_type(output: &mut String, ty: &JavaType) -> std::fmt::Result {
        let (base, dimensions) = ty.array_shape();
        if let JavaType::Class(class) = base {
            for (index, segment) in class.segments.iter().enumerate() {
                if index != 0 {
                    output.push('.');
                }
                write!(output, "{}", segment.name)?;
            }
        } else {
            write!(output, "{base}")?;
        }
        for _ in 0..dimensions {
            output.push_str("[]");
        }
        Ok(())
    }

    fn label(
        &self,
        output: &mut String,
        label: &Option<JavaIdentifier>,
        depth: usize,
    ) -> Result<(), JavaPrintError> {
        if let Some(label) = label {
            self.indent(output, depth);
            writeln!(output, "{label}:")?;
        }
        Ok(())
    }

    fn control(
        output: &mut String,
        keyword: &str,
        label: &Option<JavaIdentifier>,
    ) -> Result<(), JavaPrintError> {
        match label {
            Some(label) => writeln!(output, "{keyword} {label};")?,
            None => writeln!(output, "{keyword};")?,
        }
        Ok(())
    }

    fn inline_statement(
        &self,
        statement: &JavaStmt,
        depth: usize,
    ) -> Result<String, JavaPrintError> {
        Ok(match statement {
            JavaStmt::Variable { ty, name, value } => match value {
                Some(value) => format!(
                    "{ty} {name} = {}",
                    self.target_typed_expression_at(value, depth)?
                ),
                None => format!("{ty} {name}"),
            },
            JavaStmt::Assign { target, op, value } => format!(
                "{} {} {}",
                self.expression_at(target, depth)?,
                op.token(),
                self.target_typed_expression_at(value, depth)?
            ),
            JavaStmt::Expression(expression) => self.expression_at(expression, depth)?,
            _ => return Err(JavaPrintError::InvalidInlineStatement),
        })
    }

    fn indent(&self, output: &mut String, depth: usize) {
        for _ in 0..depth {
            output.push_str(&self.indent);
        }
    }

    fn lambda_parameters(parameters: &[JavaIdentifier]) -> String {
        match parameters {
            [] => "()".to_string(),
            [parameter] => parameter.to_string(),
            parameters => format!(
                "({})",
                parameters
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }
}

enum TypePrintTask<'a> {
    Declaration {
        declaration: &'a JavaTypeDeclaration,
        depth: usize,
        leading_blank: bool,
    },
    End {
        depth: usize,
    },
}

enum PrintTask<'a> {
    Statement(&'a JavaStmt, usize),
    ControlBody(&'a JavaStmt, usize),
    CloseBrace(usize),
    Else(&'a JavaStmt, usize),
    DoWhileTail(&'a JavaExpr, usize),
    Catch(&'a JavaCatch, usize),
    Finally(&'a JavaStmt, usize),
    SwitchCase(&'a JavaSwitchCase, usize),
}

#[path = "emit/expression.rs"]
mod expression;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::language::java::JavaPrimitiveType;
    use crate::language::java::JavaTypeArgument;

    #[test]
    fn allocated_dimensions_precede_unallocated_array_rank() {
        let element_type = JavaType::array(JavaType::Primitive(JavaPrimitiveType::Byte));
        let expression = JavaPrinter::default()
            .expression_at(
                &JavaExpr::NewArray {
                    element_type,
                    dimensions: vec![JavaExpr::Literal(super::super::JavaLiteral::Integer(3))],
                    initializer: vec![],
                },
                0,
            )
            .expect("array expression");

        assert_eq!(expression, "new byte[3][]");
    }

    #[test]
    fn array_initializer_preserves_the_complete_rank() {
        let element_type = JavaType::array(JavaType::Primitive(JavaPrimitiveType::Byte));
        let expression = JavaPrinter::default()
            .expression_at(
                &JavaExpr::NewArray {
                    element_type,
                    dimensions: vec![],
                    initializer: vec![JavaExpr::Name(JavaIdentifier::from_hint("row"))],
                },
                0,
            )
            .expect("array expression");

        assert_eq!(expression, "new byte[][] { row }");
    }

    #[test]
    fn qualified_class_creation_uses_the_member_type_name() {
        let ty = JavaType::source_class("example.Outer.Inner");
        let expression = JavaPrinter::default()
            .expression_at(
                &JavaExpr::New {
                    enclosing: Some(Box::new(JavaExpr::Name(JavaIdentifier::from_hint("owner")))),
                    ty,
                    target_type: None,
                    args: vec![],
                    anonymous_body: None,
                },
                0,
            )
            .expect("qualified class creation");

        assert_eq!(expression, "owner.new Inner()");
    }

    #[test]
    fn target_typed_construction_uses_diamond_syntax() {
        let mut ty = match JavaType::source_class("java.util.ArrayList") {
            JavaType::Class(ty) => ty,
            _ => unreachable!(),
        };
        ty.segments_mut().last_mut().unwrap().arguments = vec![JavaTypeArgument::Exact(
            JavaType::source_class("java.lang.String"),
        )];
        let ty = JavaType::Class(ty);
        let expression = JavaExpr::New {
            enclosing: None,
            ty: ty.clone(),
            target_type: Some(ty),
            args: Vec::new(),
            anonymous_body: None,
        };
        let expression = JavaPrinter::default()
            .target_typed_expression_at(&expression, 0)
            .expect("diamond construction");

        assert_eq!(expression, "new java.util.ArrayList<>()");
    }

    #[test]
    fn static_method_owner_does_not_emit_class_type_arguments() {
        let mut owner = match JavaType::source_class("example.Owner") {
            JavaType::Class(owner) => owner,
            _ => unreachable!(),
        };
        owner.segments_mut().last_mut().unwrap().arguments = vec![JavaTypeArgument::Exact(
            JavaType::source_class("java.lang.String"),
        )];
        let expression = JavaExpr::Call {
            receiver: None,
            owner: Some(JavaType::Class(owner)),
            type_arguments: vec![JavaType::source_class("java.lang.Integer")],
            method: JavaIdentifier::from_dex("select"),
            args: Vec::new(),
        };

        let expression = JavaPrinter::default()
            .expression_at(&expression, 0)
            .expect("static method call");

        assert_eq!(expression, "example.Owner.<java.lang.Integer>select()");
    }

    #[test]
    fn static_field_owner_does_not_emit_class_type_arguments() {
        let mut owner = match JavaType::source_class("example.Owner") {
            JavaType::Class(owner) => owner,
            _ => unreachable!(),
        };
        owner.segments_mut().last_mut().unwrap().arguments = vec![JavaTypeArgument::Exact(
            JavaType::source_class("java.lang.String"),
        )];
        let expression = JavaExpr::StaticField {
            owner: JavaType::Class(owner),
            name: JavaIdentifier::from_dex("INSTANCE"),
        };

        let expression = JavaPrinter::default()
            .expression_at(&expression, 0)
            .expect("static field access");

        assert_eq!(expression, "example.Owner.INSTANCE");
    }

    #[test]
    fn cast_operand_does_not_inherit_target_typing() {
        let mut class = match JavaType::source_class("java.util.ArrayList") {
            JavaType::Class(class) => class,
            _ => unreachable!(),
        };
        class.segments_mut().last_mut().unwrap().arguments = vec![JavaTypeArgument::Exact(
            JavaType::source_class("java.lang.String"),
        )];
        let ty = JavaType::Class(class);
        let expression = JavaExpr::Cast {
            ty: JavaType::source_class("java.lang.Object"),
            value: Box::new(JavaExpr::New {
                enclosing: None,
                ty: ty.clone(),
                target_type: Some(ty),
                args: Vec::new(),
                anonymous_body: None,
            }),
        };
        let expression = JavaPrinter::default()
            .target_typed_expression_at(&expression, 0)
            .expect("cast construction");

        assert_eq!(
            expression,
            "(java.lang.Object) new java.util.ArrayList<java.lang.String>()"
        );
    }

    #[test]
    fn empty_enum_constant_list_is_separated_from_members() {
        fn declaration(kind: JavaTypeDeclarationKind, name: &str) -> JavaTypeDeclaration {
            JavaTypeDeclaration {
                annotations: Vec::new(),
                modifiers: Vec::new(),
                kind,
                name: JavaIdentifier::from_dex(name),
                type_parameters: Vec::new(),
                extends: None,
                implements: Vec::new(),
                enum_constants: Vec::new(),
                fields: Vec::new(),
                methods: Vec::new(),
                nested: Vec::new(),
            }
        }

        let mut root = declaration(JavaTypeDeclarationKind::Enum, "State");
        root.nested
            .push(declaration(JavaTypeDeclarationKind::Class, "Member"));
        let source = JavaPrinter::default()
            .print_compilation_unit(&JavaCompilationUnit {
                package: None,
                imports: Vec::new(),
                declaration: root,
            })
            .expect("enum source");

        assert!(source.contains("enum State {\n    ;\n\n    class Member"));
    }
}
