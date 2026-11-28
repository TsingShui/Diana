use std::collections::BTreeSet;

use crate::language::java::{
    JavaAssignOp, JavaAstRewriter, JavaBinaryOp, JavaExpr, JavaFieldDeclaration, JavaIdentifier,
    JavaLiteral, JavaMethodBody, JavaMethodDeclaration, JavaMethodDeclarationKind, JavaModifier,
    JavaNameScope, JavaPrimitiveType, JavaStmt, JavaType, JavaTypeDeclaration,
    JavaTypeDeclarationKind,
};

pub(super) struct StaticInitializerFields<'a> {
    pub owner: &'a JavaType,
    pub ambiguous: &'a BTreeSet<JavaIdentifier>,
    pub constant_capable: &'a BTreeSet<JavaIdentifier>,
    pub prepared_defaults: &'a BTreeSet<JavaIdentifier>,
    pub implicit_scalar_defaults: &'a BTreeSet<JavaIdentifier>,
}

impl StaticInitializerFields<'_> {
    fn owns(&self, owner: &JavaType, name: &JavaIdentifier) -> bool {
        self.owner == owner && !self.ambiguous.contains(name)
    }

    fn has_default_initializer(&self, field: &JavaFieldDeclaration) -> bool {
        self.prepared_defaults.contains(&field.name)
            && StaticInitializationRecovery::is_default_initializer(field)
    }
}

/// Recovers the direct-assignment prefix of `<clinit>` as field initializers.
///
/// The recovered fields are ordered by the executable schedule, not by DEX
/// field-table order. No executable statement is crossed.
pub(super) struct StaticInitializationRecovery;

impl StaticInitializationRecovery {
    pub(super) fn apply(
        declaration: &mut JavaTypeDeclaration,
        fields: &StaticInitializerFields<'_>,
    ) {
        Self::recover_assignments(declaration, fields);
        for field in &mut declaration.fields {
            if !fields.implicit_scalar_defaults.contains(&field.name)
                || !fields.has_default_initializer(field)
            {
                continue;
            }
            if let Some(value) = field.initializer.take() {
                // A prepared value still requires active initialization
                // on a static read, even when this class has no writes.
                field.initializer =
                    Some(Self::runtime_initializer(value.clone(), value, &field.ty));
            }
        }
    }

    fn recover_assignments(
        declaration: &mut JavaTypeDeclaration,
        fields: &StaticInitializerFields<'_>,
    ) {
        let Some(initializer_index) = declaration
            .methods
            .iter()
            .position(|method| method.kind == JavaMethodDeclarationKind::ClassInitializer)
        else {
            return;
        };
        let mut method_names = JavaNameScope::default();
        for name in declaration
            .methods
            .iter()
            .filter_map(|method| method.name.clone())
        {
            method_names.reserve(name);
        }
        let Some(body) = declaration.methods[initializer_index].body.as_mut() else {
            return;
        };
        let JavaStmt::Block(statements) = &mut body.root else {
            return;
        };
        let interface_snapshot = declaration
            .kind
            .is_interface()
            .then(|| (declaration.fields.clone(), statements.clone()));

        let mut assignments = Vec::new();
        let mut assigned_fields = std::collections::BTreeSet::new();
        let mut consumed = 0usize;
        while consumed < statements.len() {
            let Some(fact) =
                InitializationFact::analyze(&statements[consumed..], fields, &declaration.fields)
            else {
                break;
            };
            let Some(field_index) = declaration.fields.iter().position(|field| {
                field.name == fact.field
                    && fields.prepared_defaults.contains(&field.name)
                    && (field.initializer.is_none() || fields.has_default_initializer(field))
                    && field.modifiers.contains(&JavaModifier::Static)
            }) else {
                break;
            };
            let value = if fields.constant_capable.contains(&fact.field) {
                // Moving a runtime write into a final scalar declaration can
                // turn reads into constants and skip class initialization.
                if !declaration.kind.is_interface() {
                    break;
                }
                let Some(default) = declaration.fields[field_index].initializer.clone() else {
                    break;
                };
                Self::runtime_initializer(fact.value, default, &declaration.fields[field_index].ty)
            } else {
                fact.value
            };
            if !assigned_fields.insert(field_index) {
                break;
            }
            assignments.push((field_index, value));
            consumed += fact.consumed;
        }
        if !assignments.is_empty() {
            let insertion_index = assignments
                .iter()
                .map(|(field_index, _)| *field_index)
                .min()
                .unwrap();
            let mut recovered = assignments
                .iter()
                .map(|(field_index, value)| {
                    let mut field = declaration.fields[*field_index].clone();
                    field.initializer = Some(value.clone());
                    field
                })
                .collect::<Vec<JavaFieldDeclaration>>();
            let selected = assignments
                .into_iter()
                .map(|(field_index, _)| field_index)
                .collect::<std::collections::BTreeSet<_>>();
            declaration.fields = declaration
                .fields
                .drain(..)
                .enumerate()
                .filter_map(|(index, field)| (!selected.contains(&index)).then_some(field))
                .collect();
            declaration
                .fields
                .splice(insertion_index..insertion_index, recovered.drain(..));
            statements.drain(..consumed);
        }

        if declaration.kind.is_interface() {
            if statements.is_empty() {
                declaration.methods.remove(initializer_index);
                return;
            }
            if let Some(initialization) = InterfaceInitialization::recover(
                fields,
                &mut declaration.fields,
                statements,
                &mut method_names,
            ) {
                declaration.methods[initializer_index] = initialization;
                return;
            }
            if let Some((original_fields, original_statements)) = interface_snapshot {
                declaration.fields = original_fields;
                *statements = original_statements;
                let mut type_names = JavaNameScope::default();
                type_names.reserve(declaration.name.clone());
                for nested in &declaration.nested {
                    type_names.reserve(nested.name.clone());
                }
                let holder_name =
                    type_names.claim(JavaIdentifier::from_dex("DexdecStaticInitialization"));
                if let Some(holder) = InterfaceInitialization::recover_with_holder(
                    fields,
                    holder_name,
                    &mut declaration.fields,
                    statements,
                ) {
                    declaration.methods.remove(initializer_index);
                    declaration.nested.push(holder);
                    return;
                }
            }
        }

        let remove_initializer = statements.is_empty();
        let mut writes = StaticInitializerWrites::new(
            fields,
            declaration
                .fields
                .iter()
                .map(|field| field.name.clone())
                .collect(),
        );
        writes.rewrite_body(body);
        for field in &mut declaration.fields {
            if writes.fields.contains(&field.name)
                && field.modifiers.contains(&JavaModifier::Static)
                && field.modifiers.contains(&JavaModifier::Final)
                && fields.has_default_initializer(field)
            {
                field.initializer = None;
            }
        }
        if remove_initializer {
            declaration.methods.remove(initializer_index);
        }
    }

    fn runtime_initializer(value: JavaExpr, default: JavaExpr, field_type: &JavaType) -> JavaExpr {
        // This condition has no effects and is outside Java's constant
        // expression grammar. It always selects the original value.
        let null = || Box::new(JavaExpr::Literal(JavaLiteral::Null));
        let expression = JavaExpr::Conditional {
            condition: Box::new(JavaExpr::Binary {
                left: null(),
                op: JavaBinaryOp::Equal,
                right: null(),
            }),
            when_true: Box::new(value),
            when_false: Box::new(default),
        };
        if matches!(
            field_type,
            JavaType::Primitive(JavaPrimitiveType::Byte | JavaPrimitiveType::Short)
        ) {
            JavaExpr::Cast {
                ty: field_type.clone(),
                value: Box::new(expression),
            }
        } else {
            expression
        }
    }

    fn is_default_initializer(field: &JavaFieldDeclaration) -> bool {
        match (&field.ty, field.initializer.as_ref()) {
            (
                JavaType::Class(_) | JavaType::Array(_),
                Some(JavaExpr::Literal(JavaLiteral::Null)),
            ) => true,
            (
                JavaType::Primitive(JavaPrimitiveType::Boolean),
                Some(JavaExpr::Literal(JavaLiteral::Boolean(false))),
            ) => true,
            (
                JavaType::Primitive(
                    JavaPrimitiveType::Byte
                    | JavaPrimitiveType::Short
                    | JavaPrimitiveType::Char
                    | JavaPrimitiveType::Int,
                ),
                Some(JavaExpr::Literal(JavaLiteral::Integer(0))),
            ) => true,
            (
                JavaType::Primitive(JavaPrimitiveType::Char),
                Some(JavaExpr::Literal(JavaLiteral::Character(0))),
            ) => true,
            (
                JavaType::Primitive(JavaPrimitiveType::Long),
                Some(JavaExpr::Literal(JavaLiteral::Long(0))),
            ) => true,
            (
                JavaType::Primitive(JavaPrimitiveType::Float),
                Some(JavaExpr::Literal(JavaLiteral::Float(value))),
            ) => value.to_bits() == 0,
            (
                JavaType::Primitive(JavaPrimitiveType::Double),
                Some(JavaExpr::Literal(JavaLiteral::Double(value))),
            ) => value.to_bits() == 0,
            _ => false,
        }
    }
}

struct InterfaceInitialization;

impl InterfaceInitialization {
    fn recover(
        ownership: &StaticInitializerFields<'_>,
        fields: &mut [JavaFieldDeclaration],
        statements: &mut Vec<JavaStmt>,
        method_names: &mut JavaNameScope,
    ) -> Option<JavaMethodDeclaration> {
        let (field_name, value) = Self::static_assignment(ownership, statements.last()?)?;
        let field_index = fields.iter().position(|field| {
            &field.name == field_name
                && field.modifiers.contains(&JavaModifier::Static)
                && field.modifiers.contains(&JavaModifier::Final)
                && ownership.has_default_initializer(field)
        })?;

        let mut writes = StaticInitializerWrites::new(
            ownership,
            fields.iter().map(|field| field.name.clone()).collect(),
        );
        writes.rewrite_statement(JavaStmt::Block(statements.clone()));
        if writes.fields.len() != 1 || !writes.fields.contains(field_name) {
            return None;
        }

        let field = &mut fields[field_index];
        let helper_name = method_names.claim(JavaIdentifier::from_dex("initialize"));
        let value = value.clone();
        let mut helper_statements = std::mem::take(statements);
        *helper_statements.last_mut()? = JavaStmt::Return(Some(value));
        field.initializer = Some(JavaExpr::Call {
            receiver: None,
            owner: None,
            type_arguments: Vec::new(),
            virtual_target: None,
            method: helper_name.clone(),
            args: Vec::new(),
        });

        Some(JavaMethodDeclaration {
            annotations: Vec::new(),
            annotation_default: None,
            annotation_default_issue: None,
            modifiers: vec![JavaModifier::Static],
            compiler_generated: true,
            kind: JavaMethodDeclarationKind::Method,
            type_parameters: Vec::new(),
            return_type: Some(field.ty.clone()),
            name: Some(helper_name),
            parameters: Vec::new(),
            throws: Vec::new(),
            body: Some(JavaMethodBody {
                root: JavaStmt::Block(helper_statements),
            }),
        })
    }

    fn recover_with_holder(
        ownership: &StaticInitializerFields<'_>,
        holder_name: JavaIdentifier,
        fields: &mut [JavaFieldDeclaration],
        statements: &mut Vec<JavaStmt>,
    ) -> Option<JavaTypeDeclaration> {
        let declared_fields = fields
            .iter()
            .map(|field| field.name.clone())
            .collect::<std::collections::BTreeSet<_>>();
        let mut writes = StaticInitializerWrites::new(ownership, declared_fields);
        let mut analyzed = JavaMethodBody {
            root: JavaStmt::Block(statements.clone()),
        };
        writes.rewrite_body(&mut analyzed);
        // A holder delays publication of the interface fields until its
        // initialization returns. An effect between the original writes can
        // reenter the interface and observe a field that is already published.
        if writes.fields.is_empty() || !writes.fields.is_disjoint(ownership.constant_capable) {
            return None;
        }
        if !Self::can_delay_publication(ownership, statements, &writes.fields) {
            return None;
        }
        let selected = fields
            .iter()
            .enumerate()
            .filter_map(|(index, field)| writes.fields.contains(&field.name).then_some(index))
            .collect::<Vec<_>>();
        if selected.len() != writes.fields.len()
            || selected.iter().any(|index| {
                let field = &fields[*index];
                !field.modifiers.contains(&JavaModifier::Static)
                    || !field.modifiers.contains(&JavaModifier::Final)
                    || !ownership.has_default_initializer(field)
                    || matches!(field.ty, JavaType::Primitive(_))
            })
        {
            return None;
        }

        let holder_type = JavaType::source_class(holder_name.as_str());
        let holder_fields = selected
            .iter()
            .map(|index| {
                let field = &fields[*index];
                JavaFieldDeclaration {
                    annotations: Vec::new(),
                    modifiers: vec![JavaModifier::Private, JavaModifier::Static],
                    ty: field.ty.clone(),
                    name: field.name.clone(),
                    initializer: None,
                }
            })
            .collect();
        for index in selected {
            let field = &mut fields[index];
            field.initializer = Some(JavaExpr::StaticField {
                owner: holder_type.clone(),
                name: field.name.clone(),
            });
        }
        let mut holder_body = JavaMethodBody {
            root: JavaStmt::Block(std::mem::take(statements)),
        };
        InterfaceStaticHolderReferences {
            ownership,
            holder: holder_type,
            fields: &writes.fields,
        }
        .rewrite_body(&mut holder_body);

        Some(JavaTypeDeclaration {
            annotations: Vec::new(),
            annotation_default_issue: None,
            modifiers: vec![
                JavaModifier::Public,
                JavaModifier::Static,
                JavaModifier::Final,
            ],
            kind: JavaTypeDeclarationKind::Class,
            name: holder_name,
            type_parameters: Vec::new(),
            extends: None,
            implements: Vec::new(),
            enum_constants: Vec::new(),
            fields: holder_fields,
            methods: vec![JavaMethodDeclaration {
                annotations: Vec::new(),
                annotation_default: None,
                annotation_default_issue: None,
                modifiers: vec![JavaModifier::Static],
                compiler_generated: true,
                kind: JavaMethodDeclarationKind::ClassInitializer,
                type_parameters: Vec::new(),
                return_type: None,
                name: None,
                parameters: Vec::new(),
                throws: Vec::new(),
                body: Some(holder_body),
            }],
            nested: Vec::new(),
        })
    }

    fn can_delay_publication(
        ownership: &StaticInitializerFields<'_>,
        statements: &[JavaStmt],
        fields: &BTreeSet<JavaIdentifier>,
    ) -> bool {
        let Some(start) = statements.len().checked_sub(fields.len()) else {
            return false;
        };
        let (setup, publication) = statements.split_at(start);
        let saved_references = setup
            .iter()
            .filter_map(|statement| match statement {
                JavaStmt::Variable { name, ty, .. } if !matches!(ty, JavaType::Primitive(_)) => {
                    Some(name)
                }
                _ => None,
            })
            .collect::<BTreeSet<_>>();
        let mut assigned = BTreeSet::new();
        for statement in publication {
            let Some((name, value)) = Self::static_assignment(ownership, statement) else {
                return false;
            };
            if !fields.contains(name)
                || !assigned.insert(name)
                || !matches!(value, JavaExpr::Name(local) if saved_references.contains(local))
                    && !matches!(value, JavaExpr::Literal(JavaLiteral::Null))
            {
                return false;
            }
        }

        let mut earlier_use = EarlierInterfaceFieldUse {
            ownership,
            fields,
            found: false,
        };
        earlier_use.rewrite_statement(JavaStmt::Block(setup.to_vec()));
        !earlier_use.found
    }

    fn static_assignment<'s>(
        ownership: &StaticInitializerFields<'_>,
        statement: &'s JavaStmt,
    ) -> Option<(&'s JavaIdentifier, &'s JavaExpr)> {
        let JavaStmt::Assign {
            target:
                JavaExpr::StaticField {
                    owner: field_owner,
                    name,
                },
            op: JavaAssignOp::Assign,
            value,
        } = statement
        else {
            return None;
        };
        ownership.owns(field_owner, name).then_some((name, value))
    }
}

struct EarlierInterfaceFieldUse<'a> {
    ownership: &'a StaticInitializerFields<'a>,
    fields: &'a BTreeSet<JavaIdentifier>,
    found: bool,
}

impl JavaAstRewriter for EarlierInterfaceFieldUse<'_> {
    fn finish_expression(&mut self, expression: JavaExpr) -> JavaExpr {
        self.found |= match &expression {
            JavaExpr::StaticField { owner, name } => {
                self.fields.contains(name) && self.ownership.owns(owner, name)
            }
            JavaExpr::Name(name) => self.fields.contains(name),
            _ => false,
        };
        expression
    }
}

struct InterfaceStaticHolderReferences<'a> {
    ownership: &'a StaticInitializerFields<'a>,
    holder: JavaType,
    fields: &'a std::collections::BTreeSet<JavaIdentifier>,
}

impl JavaAstRewriter for InterfaceStaticHolderReferences<'_> {
    fn finish_expression(&mut self, expression: JavaExpr) -> JavaExpr {
        match expression {
            JavaExpr::StaticField { owner, name }
                if self.fields.contains(&name) && self.ownership.owns(&owner, &name) =>
            {
                JavaExpr::StaticField {
                    owner: self.holder.clone(),
                    name,
                }
            }
            expression => expression,
        }
    }
}

struct InitializationFact {
    field: crate::language::java::JavaIdentifier,
    value: JavaExpr,
    consumed: usize,
}

impl InitializationFact {
    fn analyze(
        statements: &[JavaStmt],
        fields: &StaticInitializerFields<'_>,
        declarations: &[JavaFieldDeclaration],
    ) -> Option<Self> {
        Self::direct(statements, fields)
            .or_else(|| Self::first_evaluation(statements, fields, declarations))
    }

    fn direct(statements: &[JavaStmt], fields: &StaticInitializerFields<'_>) -> Option<Self> {
        let JavaStmt::Assign {
            target: JavaExpr::StaticField { owner, name },
            op: JavaAssignOp::Assign,
            value,
        } = statements.first()?
        else {
            return None;
        };
        fields.owns(owner, name).then_some(())?;
        Some(Self {
            field: name.clone(),
            value: value.clone(),
            consumed: 1,
        })
    }

    fn first_evaluation(
        statements: &[JavaStmt],
        fields: &StaticInitializerFields<'_>,
        declarations: &[JavaFieldDeclaration],
    ) -> Option<Self> {
        let [JavaStmt::Variable {
            ty,
            name,
            value: Some(saved),
            ..
        }, JavaStmt::Assign {
            target: JavaExpr::StaticField { owner, name: field },
            op: JavaAssignOp::Assign,
            value,
        }, trailing @ ..] = statements
        else {
            return None;
        };
        if !fields.owns(owner, field)
            || ExpressionNames::collect(value)
                .iter()
                .filter(|candidate| *candidate == name)
                .count()
                != 1
            || trailing
                .iter()
                .flat_map(ExpressionNames::collect_statement)
                .any(|candidate| &candidate == name)
        {
            return None;
        }

        let mut value = value.clone();
        // A direct store to the same type keeps the assignment context,
        // including target typing and conversions of the initializer.
        let same_assignment = matches!(&value, JavaExpr::Name(candidate) if candidate == name)
            && declarations
                .iter()
                .any(|declaration| &declaration.name == field && &declaration.ty == ty);
        let replacement = if same_assignment {
            saved.clone()
        } else {
            Self::preserve_binding_type(saved, ty)?
        };
        if !value.bind_first_evaluation(name, replacement) {
            return None;
        }
        Some(Self {
            field: field.clone(),
            value,
            consumed: 2,
        })
    }

    fn preserve_binding_type(value: &JavaExpr, ty: &JavaType) -> Option<JavaExpr> {
        match value {
            JavaExpr::Cast { ty: source, .. } if source == ty => Some(value.clone()),
            JavaExpr::Literal(literal)
                if Self::primitive_literal_type(literal).as_ref() == Some(ty) =>
            {
                Some(value.clone())
            }
            _ => None,
        }
    }

    fn primitive_literal_type(literal: &JavaLiteral) -> Option<JavaType> {
        use JavaPrimitiveType::{Boolean, Char, Double, Float, Int, Long};

        let primitive = match literal {
            JavaLiteral::Boolean(_) => Boolean,
            JavaLiteral::Character(_) => Char,
            JavaLiteral::Integer(_) => Int,
            JavaLiteral::Long(_) => Long,
            JavaLiteral::Float(_) => Float,
            JavaLiteral::Double(_) => Double,
            _ => return None,
        };
        Some(JavaType::Primitive(primitive))
    }
}

#[derive(Default)]
struct ExpressionNames {
    names: Vec<crate::language::java::JavaIdentifier>,
}

impl ExpressionNames {
    fn collect(expression: &JavaExpr) -> Vec<crate::language::java::JavaIdentifier> {
        let mut collector = Self::default();
        collector.rewrite_expression(expression.clone());
        collector.names
    }

    fn collect_statement(statement: &JavaStmt) -> Vec<crate::language::java::JavaIdentifier> {
        let mut collector = Self::default();
        collector.rewrite_statement(statement.clone());
        collector.names
    }
}

impl JavaAstRewriter for ExpressionNames {
    fn finish_expression(&mut self, expression: JavaExpr) -> JavaExpr {
        if let JavaExpr::Name(name) = &expression {
            self.names.push(name.clone());
        }
        expression
    }
}

struct StaticInitializerWrites<'a> {
    ownership: &'a StaticInitializerFields<'a>,
    declared_fields: BTreeSet<JavaIdentifier>,
    fields: BTreeSet<JavaIdentifier>,
}

impl<'a> StaticInitializerWrites<'a> {
    fn new(
        ownership: &'a StaticInitializerFields<'a>,
        declared_fields: BTreeSet<JavaIdentifier>,
    ) -> Self {
        Self {
            ownership,
            declared_fields,
            fields: BTreeSet::new(),
        }
    }

    fn is_own_field(&self, owner: &JavaType, name: &JavaIdentifier) -> bool {
        self.declared_fields.contains(name) && self.ownership.owns(owner, name)
    }
}

impl JavaAstRewriter for StaticInitializerWrites<'_> {
    fn finish_statement(&mut self, statement: JavaStmt) -> JavaStmt {
        if let JavaStmt::Assign {
            target: JavaExpr::StaticField { owner, name },
            op,
            value,
        } = statement
        {
            if !self.is_own_field(&owner, &name) {
                return JavaStmt::Assign {
                    target: JavaExpr::StaticField { owner, name },
                    op,
                    value,
                };
            }
            self.fields.insert(name.clone());
            return JavaStmt::Assign {
                target: JavaExpr::Name(name),
                op,
                value,
            };
        }
        statement
    }
}

#[cfg(test)]
mod tests {
    use super::{BTreeSet, StaticInitializationRecovery, StaticInitializerFields};
    use crate::language::java::{
        JavaAssignOp, JavaExpr, JavaFieldDeclaration, JavaIdentifier, JavaLiteral, JavaMethodBody,
        JavaMethodDeclaration, JavaMethodDeclarationKind, JavaModifier, JavaStmt, JavaType,
        JavaTypeDeclaration, JavaTypeDeclarationKind,
    };

    fn recover(declaration: &mut JavaTypeDeclaration) {
        let prepared_defaults = declaration
            .fields
            .iter()
            .map(|field| field.name.clone())
            .collect();
        StaticInitializationRecovery::apply(
            declaration,
            &StaticInitializerFields {
                owner: &JavaType::source_class("Settings"),
                ambiguous: &BTreeSet::new(),
                constant_capable: &BTreeSet::new(),
                prepared_defaults: &prepared_defaults,
                implicit_scalar_defaults: &BTreeSet::new(),
            },
        );
    }

    fn interface_with_shared_value(value: JavaExpr) -> JavaTypeDeclaration {
        let owner = JavaIdentifier::from_dex("Settings");
        let shared = JavaIdentifier::from_dex("shared");
        let first = JavaIdentifier::from_dex("FIRST");
        let second = JavaIdentifier::from_dex("SECOND");
        let field = |name| JavaFieldDeclaration {
            annotations: Vec::new(),
            modifiers: vec![JavaModifier::Static, JavaModifier::Final],
            ty: JavaType::source_class("java.lang.Object"),
            name,
            initializer: Some(JavaExpr::Literal(JavaLiteral::Null)),
        };
        let assignment = |name| JavaStmt::Assign {
            target: JavaExpr::StaticField {
                owner: JavaType::source_class("Settings"),
                name,
            },
            op: JavaAssignOp::Assign,
            value: JavaExpr::Name(shared.clone()),
        };
        JavaTypeDeclaration {
            annotations: Vec::new(),
            annotation_default_issue: None,
            modifiers: vec![JavaModifier::Public],
            kind: JavaTypeDeclarationKind::Interface,
            name: owner,
            type_parameters: Vec::new(),
            extends: None,
            implements: Vec::new(),
            enum_constants: Vec::new(),
            fields: vec![field(first.clone()), field(second.clone())],
            methods: vec![JavaMethodDeclaration {
                annotations: Vec::new(),
                annotation_default: None,
                annotation_default_issue: None,
                modifiers: vec![JavaModifier::Static],
                compiler_generated: true,
                kind: JavaMethodDeclarationKind::ClassInitializer,
                type_parameters: Vec::new(),
                return_type: None,
                name: None,
                parameters: Vec::new(),
                throws: Vec::new(),
                body: Some(JavaMethodBody {
                    root: JavaStmt::Block(vec![
                        JavaStmt::Variable {
                            ty: JavaType::source_class("java.lang.Object"),
                            name: shared.clone(),
                            value: Some(value),
                        },
                        assignment(first),
                        assignment(second),
                    ]),
                }),
            }],
            nested: Vec::new(),
        }
    }

    #[test]
    fn interface_shared_static_reads_and_boxing_calls_are_evaluated_once() {
        let boolean_field = JavaExpr::StaticField {
            owner: JavaType::source_class("java.lang.Boolean"),
            name: JavaIdentifier::from_dex("TRUE"),
        };
        let boxing_call = JavaExpr::Call {
            receiver: None,
            owner: Some(JavaType::source_class("java.lang.Integer")),
            type_arguments: Vec::new(),
            virtual_target: None,
            method: JavaIdentifier::from_dex("valueOf"),
            args: vec![JavaExpr::Literal(JavaLiteral::Integer(0))],
        };

        for value in [boolean_field, boxing_call] {
            let mut declaration = interface_with_shared_value(value.clone());
            recover(&mut declaration);

            assert!(declaration.methods.is_empty());
            let [holder] = declaration.nested.as_slice() else {
                panic!("expected one static holder");
            };
            let root = &holder.methods[0].body.as_ref().unwrap().root;
            assert!(matches!(
                root,
                JavaStmt::Block(statements)
                    if matches!(statements.as_slice(),
                        [JavaStmt::Variable { name, value: Some(initializer), .. },
                         JavaStmt::Assign { value: JavaExpr::Name(first), .. },
                         JavaStmt::Assign { value: JavaExpr::Name(second), .. }]
                        if initializer == &value && first == name && second == name)
            ));
        }
    }

    #[test]
    fn interface_allocated_aliases_use_a_static_holder() {
        let value = JavaExpr::New {
            enclosing: None,
            ty: JavaType::source_class("java.lang.Object"),
            target_type: None,
            args: Vec::new(),
            anonymous_body: None,
        };
        let mut declaration = interface_with_shared_value(value);

        recover(&mut declaration);

        assert!(declaration.methods.is_empty());
        assert_eq!(declaration.nested.len(), 1);
        assert_eq!(declaration.nested[0].fields.len(), 2);
        assert!(declaration
            .fields
            .iter()
            .all(|field| matches!(field.initializer, Some(JavaExpr::StaticField { .. }))));
        assert!(matches!(
            &declaration.nested[0].methods[0].body.as_ref().unwrap().root,
            JavaStmt::Block(statements)
                if matches!(
                    statements.first(),
                    Some(JavaStmt::Variable {
                        value: Some(JavaExpr::New { .. }),
                        ..
                    })
                )
        ));
    }

    #[test]
    fn interface_mutable_static_aliases_use_a_static_holder() {
        let value = JavaExpr::StaticField {
            owner: JavaType::source_class("com.example.State"),
            name: JavaIdentifier::from_dex("CURRENT"),
        };
        let mut declaration = interface_with_shared_value(value);

        recover(&mut declaration);

        assert!(declaration.methods.is_empty());
        assert_eq!(declaration.nested.len(), 1);
        assert_eq!(declaration.nested[0].fields.len(), 2);
        assert!(matches!(
            &declaration.nested[0].methods[0].body.as_ref().unwrap().root,
            JavaStmt::Block(statements)
                if matches!(
                    statements.first(),
                    Some(JavaStmt::Variable {
                        value: Some(JavaExpr::StaticField { .. }),
                        ..
                    })
                )
        ));
    }
}
