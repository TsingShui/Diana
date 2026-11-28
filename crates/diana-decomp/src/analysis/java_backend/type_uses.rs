use std::collections::BTreeSet;

use crate::frontend::{AnnotationNode, DexValue};
use crate::ir::generic_types::JvmTypeSignature;
use crate::ir::ty::ArgType;

use super::java_model::JavaClassModel;
use super::type_names::{JavaTypeImportCollector, JavaTypeNameError};

/// Imports follow declaration order so the first unresolved input is preserved.
/// The ABI also needs each intermediate array component, once per compilation unit.
#[derive(Default)]
pub(super) struct DeclaredTypeUses {
    types: BTreeSet<ArgType>,
    // Class identities and unresolved member descriptors affect ABI queries
    // without participating in import resolution or its diagnostic order.
    abi_only: smallvec::SmallVec<[ArgType; 1]>,
    imports: JavaTypeImportCollector,
    error: Option<JavaTypeNameError>,
}

impl DeclaredTypeUses {
    fn insert(&mut self, ty: &ArgType) {
        if self.error.is_none() && include_abi_type(&mut self.types, ty) {
            self.error = self.imports.collect_type(ty).err();
        }
    }

    fn source_class(&mut self, internal_name: String) {
        // Signatures contribute erased classes only, so their freshly built
        // key can move directly into the ABI set.
        let ty = ArgType::Object(internal_name.into());
        if self.error.is_none() && !self.types.contains(&ty) {
            self.error = self.imports.collect_type(&ty).err();
            self.types.insert(ty);
        }
    }

    pub(super) fn extend<'a>(&mut self, types: impl IntoIterator<Item = &'a ArgType>) {
        for ty in types {
            self.insert(ty);
        }
    }

    pub(super) fn finish(
        mut self,
    ) -> Result<(BTreeSet<ArgType>, JavaTypeImportCollector), JavaTypeNameError> {
        match self.error {
            Some(error) => Err(error),
            None => {
                for ty in &self.abi_only {
                    include_abi_type(&mut self.types, ty);
                }
                Ok((self.types, self.imports))
            }
        }
    }
}

pub(super) fn include_abi_type(types: &mut BTreeSet<ArgType>, ty: &ArgType) -> bool {
    let mut current = ty;
    if types.contains(current) {
        return false;
    }
    loop {
        types.insert(current.clone());
        match current {
            ArgType::Array(element) if !types.contains(element.as_ref()) => current = element,
            _ => return true,
        }
    }
}

pub(super) struct ClassTypeUses;

impl ClassTypeUses {
    pub(super) fn collect(class: &JavaClassModel) -> DeclaredTypeUses {
        let mut types = DeclaredTypeUses::default();
        let mut pending = vec![class];
        while let Some(class) = pending.pop() {
            pending.extend(class.nested.iter().rev());
            Self::collect_class(class, &mut types);
        }
        types
    }

    fn collect_class(class: &JavaClassModel, types: &mut DeclaredTypeUses) {
        if let Some(current) = class.declaration.current_type() {
            types.abi_only.push(current);
        }
        if let Some(extends) = &class.declaration.extends {
            types.insert(extends);
        }
        AnnotationTypeUses::collect(&class.declaration.annotations, types);
        types.extend(&class.declaration.implements);
        if let Some(signature) = &class.declaration.signature {
            GenericTypeUses::class(&signature.super_class, &mut |ty| types.source_class(ty));
            for interface in &signature.super_interfaces {
                GenericTypeUses::class(interface, &mut |ty| types.source_class(ty));
            }
            GenericTypeUses::bounds(&signature.type_parameters, &mut |ty| types.source_class(ty));
        }
        for field in &class.fields {
            AnnotationTypeUses::collect(&field.annotations, types);
            types.insert(&field.field_type);
            if let Some(initializer) = &field.initializer {
                DexValueTypeUses::collect(initializer, types);
            }
            if let Some(signature) = &field.signature {
                GenericTypeUses::ty(signature, &mut |ty| types.source_class(ty));
            }
        }
        for method in &class.methods {
            JavaMethodTypeUses::collect_into(method, types);
        }
    }
}

pub(super) struct JavaMethodTypeUses;

impl JavaMethodTypeUses {
    pub(super) fn collect(method: &super::java_model::JavaMethodModel) -> DeclaredTypeUses {
        let mut types = DeclaredTypeUses::default();
        Self::collect_into(method, &mut types);
        types
    }

    fn collect_into(method: &super::java_model::JavaMethodModel, types: &mut DeclaredTypeUses) {
        AnnotationTypeUses::collect(&method.declaration.annotations, types);
        if let Some(value) = &method.declaration.annotation_default {
            DexValueTypeUses::collect(value, types);
        }
        if method.declaration.override_semantics.is_some() {
            types.insert(&ArgType::object("java/lang/Override"));
        }
        if let Some(return_type) = &method.declaration.return_type {
            types.insert(return_type);
        }
        if let Some(return_type) = &method.declaration.source_return_type {
            types.insert(return_type);
        }
        if let Some(interface) = &method.declaration.function_interface {
            GenericTypeUses::ty(interface, &mut |ty| types.source_class(ty));
        }
        for parameter in &method.declaration.parameters {
            types.insert(&parameter.ty);
            AnnotationTypeUses::collect(&parameter.annotations, types);
        }
        types.extend(method.declaration.source_parameter_types.iter().flatten());
        types.extend(&method.declaration.throws);
        if let Some(signature) = &method.declaration.signature {
            GenericTypeUses::ty(&signature.return_type, &mut |ty| types.source_class(ty));
            for parameter in &signature.parameter_types {
                GenericTypeUses::ty(parameter, &mut |ty| types.source_class(ty));
            }
            for exception in &signature.throws {
                GenericTypeUses::ty(exception, &mut |ty| types.source_class(ty));
            }
            GenericTypeUses::bounds(&signature.type_parameters, &mut |ty| types.source_class(ty));
        }
        if let Some(body) = &method.body {
            types.extend(body.lambda_reference_types());
            types.extend(body.type_uses());
            types
                .abi_only
                .extend(body.unresolved_member_types().iter().cloned());
        }
    }
}

pub(super) struct GenericTypeUses;

impl GenericTypeUses {
    pub(super) fn contracts<'a>(
        fields: impl Iterator<Item = &'a crate::ir::generic_types::GenericFieldContract>,
        methods: impl Iterator<Item = &'a crate::ir::generic_types::GenericMethodContract>,
    ) -> Vec<ArgType> {
        let mut types = BTreeSet::new();
        let mut insert = |name: String| {
            types.insert(ArgType::Object(name.into()));
        };
        for contract in fields {
            Self::class(&contract.owner, &mut insert);
            Self::ty(&contract.signature, &mut insert);
        }
        for contract in methods {
            Self::class(&contract.owner, &mut insert);
            for parameter in &contract.signature.parameter_types {
                Self::ty(parameter, &mut insert);
            }
            Self::ty(&contract.signature.return_type, &mut insert);
            for exception in &contract.signature.throws {
                Self::ty(exception, &mut insert);
            }
            Self::bounds(&contract.signature.type_parameters, &mut insert);
        }
        types.into_iter().collect()
    }

    fn bounds(
        parameters: &[crate::ir::generic_types::TypeParameter],
        types: &mut impl FnMut(String),
    ) {
        for parameter in parameters {
            if let Some(bound) = &parameter.class_bound {
                GenericTypeUses::ty(bound, types);
            }
            for bound in &parameter.interface_bounds {
                GenericTypeUses::ty(bound, types);
            }
        }
    }

    fn ty(signature: &JvmTypeSignature, types: &mut impl FnMut(String)) {
        Self::visit(GenericTypeTask::Type(signature), types);
    }

    fn class(class: &crate::ir::generic_types::ClassTypeSignature, types: &mut impl FnMut(String)) {
        Self::visit(GenericTypeTask::Class(class), types);
    }

    fn visit(root: GenericTypeTask<'_>, types: &mut impl FnMut(String)) {
        let mut pending = smallvec::SmallVec::<[GenericTypeTask<'_>; 4]>::new();
        pending.push(root);
        while let Some(task) = pending.pop() {
            match task {
                GenericTypeTask::Type(JvmTypeSignature::Array(element)) => {
                    pending.push(GenericTypeTask::Type(element));
                }
                GenericTypeTask::Type(JvmTypeSignature::ClassType(class))
                | GenericTypeTask::Class(class) => {
                    types(class.erased_name());
                    pending.extend(
                        class
                            .inner_segments
                            .iter()
                            .rev()
                            .flat_map(|segment| segment.type_arguments.iter().rev())
                            .map(GenericTypeTask::Argument),
                    );
                    pending.extend(
                        class
                            .type_arguments
                            .iter()
                            .rev()
                            .map(GenericTypeTask::Argument),
                    );
                }
                GenericTypeTask::Argument(argument) => match argument {
                    crate::ir::generic_types::TypeArgument::Unbounded => {}
                    crate::ir::generic_types::TypeArgument::Extends(ty)
                    | crate::ir::generic_types::TypeArgument::Super(ty)
                    | crate::ir::generic_types::TypeArgument::Exact(ty) => {
                        pending.push(GenericTypeTask::Type(ty));
                    }
                },
                GenericTypeTask::Type(JvmTypeSignature::TypeVariable(_))
                | GenericTypeTask::Type(JvmTypeSignature::BaseType(_)) => {}
            }
        }
    }
}

enum GenericTypeTask<'a> {
    Type(&'a JvmTypeSignature),
    Class(&'a crate::ir::generic_types::ClassTypeSignature),
    Argument(&'a crate::ir::generic_types::TypeArgument),
}

struct AnnotationTypeUses;

impl AnnotationTypeUses {
    pub(super) fn collect(annotations: &[AnnotationNode], types: &mut DeclaredTypeUses) {
        for annotation in annotations {
            types.insert(&annotation.annotation_type);
            for element in &annotation.elements {
                DexValueTypeUses::collect(&element.value, types);
            }
        }
    }
}

struct DexValueTypeUses;

impl DexValueTypeUses {
    pub(super) fn collect(value: &DexValue, types: &mut DeclaredTypeUses) {
        let mut pending = vec![value];
        while let Some(value) = pending.pop() {
            match value {
                DexValue::Type(ty) => types.insert(ty),
                DexValue::Field(reference) | DexValue::Enum(reference) => {
                    types.insert(&reference.owner);
                    types.insert(&reference.field_type);
                }
                DexValue::Method(reference) => {
                    types.insert(&reference.owner);
                    types.extend(&reference.descriptor.parameters);
                    types.insert(&reference.descriptor.return_type);
                }
                DexValue::MethodType(descriptor) => {
                    types.extend(&descriptor.parameters);
                    types.insert(&descriptor.return_type);
                }
                DexValue::Array(values) => pending.extend(values.iter().rev()),
                DexValue::Annotation(annotation) => {
                    types.insert(&annotation.annotation_type);
                    pending.extend(
                        annotation
                            .elements
                            .iter()
                            .rev()
                            .map(|element| &element.value),
                    );
                }
                DexValue::Null
                | DexValue::Boolean(_)
                | DexValue::Byte(_)
                | DexValue::Short(_)
                | DexValue::Char(_)
                | DexValue::Int(_)
                | DexValue::Long(_)
                | DexValue::Float(_)
                | DexValue::Double(_)
                | DexValue::String(_)
                | DexValue::Unsupported { .. } => {}
            }
        }
    }
}

#[cfg(test)]
#[path = "../../../tests/internal/declared_type_uses.rs"]
mod tests;

#[cfg(test)]
#[path = "../../../tests/internal/java_abi_only_types.rs"]
mod abi_only_tests;
