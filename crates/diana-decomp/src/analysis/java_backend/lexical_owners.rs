use std::collections::{BTreeMap, BTreeSet};

use crate::language::java::{
    JavaAnonymousClassBody, JavaAstMutator, JavaAstRewriter, JavaClassName, JavaClassType,
    JavaClassTypeSegment, JavaExpr, JavaIdentifier, JavaMethodDeclarationKind, JavaModifier,
    JavaStmt, JavaType, JavaTypeDeclaration,
};

/// Makes references to enclosing declarations immune to inherited member-type
/// shadowing while retaining short names for unrelated types.
pub(super) struct LexicalOwners;

impl LexicalOwners {
    pub(super) fn recover_outer_aliases(
        root: &mut JavaTypeDeclaration,
        known: impl IntoIterator<Item = (JavaType, JavaIdentifier, JavaType)>,
    ) {
        let mut aliases = known
            .into_iter()
            .map(|(owner, field, outer)| (LexicalField::new(&owner, field), outer.into_raw()))
            .collect::<BTreeMap<_, _>>();
        let root_path = vec![root.name.clone()];
        Self::collect_aliases(root, root_path.clone(), true, &mut aliases);
        if !aliases.is_empty() {
            Self::rewrite_aliases(root, root_path, true, &aliases);
        }
    }

    pub(super) fn qualify(root: &mut JavaTypeDeclaration) {
        Self::declaration(root, &mut Vec::new());
    }

    fn declaration(declaration: &mut JavaTypeDeclaration, path: &mut Vec<JavaIdentifier>) {
        path.push(declaration.name.clone());
        let mut qualifier = EnclosingOwnerQualifier { path };
        for field in &mut declaration.fields {
            field.initializer = field
                .initializer
                .take()
                .map(|value| qualifier.rewrite_expression(value));
        }
        for method in &mut declaration.methods {
            if let Some(body) = &mut method.body {
                qualifier.rewrite_body(body);
            }
        }
        for nested in &mut declaration.nested {
            Self::declaration(nested, path);
        }
        path.pop();
    }

    fn collect_aliases(
        declaration: &mut JavaTypeDeclaration,
        owner_path: Vec<JavaIdentifier>,
        root: bool,
        aliases: &mut BTreeMap<LexicalField, JavaType>,
    ) {
        let owner = source_type(&owner_path);
        aliases.extend(LexicalAliasAnalysis::analyze(declaration, &owner));
        for nested in &mut declaration.nested {
            let mut nested_path = if root { Vec::new() } else { owner_path.clone() };
            nested_path.push(nested.name.clone());
            Self::collect_aliases(nested, nested_path, false, aliases);
        }
    }

    fn rewrite_aliases(
        declaration: &mut JavaTypeDeclaration,
        owner_path: Vec<JavaIdentifier>,
        root: bool,
        aliases: &BTreeMap<LexicalField, JavaType>,
    ) {
        let owner = source_type(&owner_path);
        let owner_key = owner.clone().into_raw().to_string();
        let owned = aliases
            .keys()
            .filter(|field| field.owner == owner_key)
            .map(|field| field.name.clone())
            .collect::<BTreeSet<_>>();
        declaration
            .fields
            .retain(|field| !owned.contains(&field.name));
        for method in &mut declaration.methods {
            if let Some(body) = &mut method.body {
                AliasStoreRemoval {
                    owner: &owner,
                    aliases,
                }
                .rewrite_body(body);
                AliasLoadRewriter {
                    owner: &owner,
                    aliases,
                }
                .rewrite_body(body);
            }
        }
        for nested in &mut declaration.nested {
            let mut nested_path = if root { Vec::new() } else { owner_path.clone() };
            nested_path.push(nested.name.clone());
            Self::rewrite_aliases(nested, nested_path, false, aliases);
        }
    }
}

fn source_type(path: &[JavaIdentifier]) -> JavaType {
    JavaType::Class(JavaClassType::raw(JavaClassName::from_identifiers(
        path.iter().cloned(),
    )))
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct LexicalField {
    owner: String,
    name: JavaIdentifier,
}

impl LexicalField {
    fn new(owner: &JavaType, name: JavaIdentifier) -> Self {
        Self {
            owner: owner.clone().into_raw().to_string(),
            name,
        }
    }
}

struct LexicalAliasAnalysis;

impl LexicalAliasAnalysis {
    fn analyze(
        declaration: &mut JavaTypeDeclaration,
        owner: &JavaType,
    ) -> BTreeMap<LexicalField, JavaType> {
        let mut candidates = declaration
            .fields
            .iter()
            .filter(|field| {
                field.initializer.is_none() && field.modifiers.contains(&JavaModifier::Final)
            })
            .map(|field| (field.name.clone(), (field.ty.clone().into_raw(), None)))
            .collect::<BTreeMap<_, _>>();
        let mut saw_constructor = false;
        for method in declaration
            .methods
            .iter_mut()
            .filter(|method| method.kind == JavaMethodDeclarationKind::Constructor)
        {
            if candidates.is_empty() {
                return BTreeMap::new();
            }
            saw_constructor = true;
            let mut writes = AliasWriteCollector::default();
            if let Some(body) = &mut method.body {
                writes.rewrite_body(body);
            }
            candidates.retain(|name, (field_type, first_outer)| {
                let Some(values) = writes.fields.get_mut(name) else {
                    return false;
                };
                let [Some(outer)] = values.as_slice() else {
                    return false;
                };
                if outer != field_type || first_outer.as_ref().is_some_and(|first| first != outer) {
                    return false;
                }
                if first_outer.is_none() {
                    *first_outer = values.pop().flatten();
                }
                true
            });
        }
        if !saw_constructor {
            return BTreeMap::new();
        }
        for method in declaration
            .methods
            .iter_mut()
            .filter(|method| method.kind != JavaMethodDeclarationKind::Constructor)
        {
            if candidates.is_empty() {
                break;
            }
            let mut writes = AliasWriteCollector::default();
            if let Some(body) = &mut method.body {
                writes.rewrite_body(body);
            }
            candidates.retain(|name, _| !writes.fields.contains_key(name));
        }
        candidates
            .into_iter()
            .filter_map(|(name, (_, outer))| Some((LexicalField::new(owner, name), outer?)))
            .collect()
    }
}

#[derive(Default)]
struct AliasWriteCollector {
    fields: BTreeMap<JavaIdentifier, Vec<Option<JavaType>>>,
}

impl AliasWriteCollector {
    fn record(&mut self, target: &JavaExpr, value: &JavaExpr) {
        let JavaExpr::Field { owner, name } = target else {
            return;
        };
        if !matches!(owner.as_ref(), JavaExpr::This) {
            return;
        }
        let outer = match value {
            JavaExpr::QualifiedThis(outer) => Some(outer.clone().into_raw()),
            _ => None,
        };
        self.fields.entry(name.clone()).or_default().push(outer);
    }
}

impl JavaAstMutator for AliasWriteCollector {
    fn after_statement(&mut self, statement: &mut JavaStmt) {
        if let JavaStmt::Assign { target, value, .. } = statement {
            self.record(target, value);
        }
    }

    fn after_expression(&mut self, expression: &mut JavaExpr) {
        if let JavaExpr::Assignment { target, value, .. } = expression {
            self.record(target, value);
        }
    }

    fn visit_anonymous(&mut self, _body: &mut JavaAnonymousClassBody) {}
}

struct AliasStoreRemoval<'a> {
    owner: &'a JavaType,
    aliases: &'a BTreeMap<LexicalField, JavaType>,
}

impl AliasStoreRemoval<'_> {
    fn is_alias_store(&self, target: &JavaExpr, value: &JavaExpr) -> bool {
        let JavaExpr::Field { owner, name } = target else {
            return false;
        };
        if !matches!(owner.as_ref(), JavaExpr::This) {
            return false;
        }
        let Some(outer) = self
            .aliases
            .get(&LexicalField::new(self.owner, name.clone()))
        else {
            return false;
        };
        matches!(value, JavaExpr::QualifiedThis(value) if value.clone().into_raw() == *outer)
    }
}

impl JavaAstRewriter for AliasStoreRemoval<'_> {
    fn finish_statement(&mut self, statement: JavaStmt) -> JavaStmt {
        let JavaStmt::Assign { target, value, .. } = &statement else {
            return statement;
        };
        if self.is_alias_store(target, value) {
            JavaStmt::Empty
        } else {
            statement
        }
    }

    fn rewrite_anonymous_body(&mut self, _body: &mut JavaAnonymousClassBody) {}
}

struct AliasLoadRewriter<'a> {
    owner: &'a JavaType,
    aliases: &'a BTreeMap<LexicalField, JavaType>,
}

impl JavaAstRewriter for AliasLoadRewriter<'_> {
    fn finish_expression(&mut self, expression: JavaExpr) -> JavaExpr {
        let JavaExpr::Field { owner, name } = expression else {
            return expression;
        };
        let field_owner = match owner.as_ref() {
            JavaExpr::This => self.owner,
            JavaExpr::QualifiedThis(owner) => owner,
            _ => return JavaExpr::Field { owner, name },
        };
        self.aliases
            .get(&LexicalField::new(field_owner, name.clone()))
            .cloned()
            .map(JavaExpr::QualifiedThis)
            .unwrap_or(JavaExpr::Field { owner, name })
    }

    fn rewrite_anonymous_body(&mut self, _body: &mut JavaAnonymousClassBody) {}
}

struct EnclosingOwnerQualifier<'a> {
    path: &'a [JavaIdentifier],
}

impl EnclosingOwnerQualifier<'_> {
    fn qualify(&self, owner: &mut JavaType) {
        let JavaType::Class(class) = owner else {
            return;
        };
        let [segment] = class.segments.as_ref() else {
            return;
        };
        let Some(index) = self.path.iter().rposition(|name| name == &segment.name) else {
            return;
        };
        let arguments = segment.arguments.clone();
        *owner = JavaType::Class(JavaClassType {
            segments: self
                .path
                .iter()
                .take(index + 1)
                .enumerate()
                .map(|(position, name)| JavaClassTypeSegment {
                    name: name.clone(),
                    arguments: if position == index {
                        arguments.clone()
                    } else {
                        Default::default()
                    },
                })
                .collect(),
        });
    }
}

impl JavaAstMutator for EnclosingOwnerQualifier<'_> {
    fn after_expression(&mut self, expression: &mut JavaExpr) {
        if let JavaExpr::Call {
            owner: Some(owner), ..
        }
        | JavaExpr::StaticField { owner, .. } = expression
        {
            self.qualify(owner);
        }
    }
}

#[cfg(test)]
#[path = "../../../tests/internal/java_lexical_alias_candidates.rs"]
mod candidate_tests;
