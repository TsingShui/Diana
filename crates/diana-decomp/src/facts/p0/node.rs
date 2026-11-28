//! Original class facts to the recovery backend class view.

use std::collections::BTreeMap;

use crate::frontend::{
    AccessInfo, AnnotationElement, AnnotationNode, AnnotationVisibility, ClassInfo, ClassMetadata,
    ClassNode, DexValue, EnclosingInfo, FieldInfo, FieldNode, InnerClassInfo, MethodInfo,
    MethodNode,
};
use crate::input::annotation_facts::{
    AnnotationExtent, AnnotationRecord, AnnotationToken, AnnotationValueKind, OriginalAnnotations,
};
use crate::input::value_facts::{ValueDeclaration, ValueMember, ValuePrototype, ValueReference};
use crate::ir::{ArgType, FieldReference, MethodDescriptor, MethodReference, Utf16String};
use crate::p0::{ClassFacts, Fact, ScalarValue};
use crate::{AnnotationLocation, AnnotationValue, ByteOffset, TypeId};

const ANNOTATION_DEFAULT: &str = "Ldalvik/annotation/AnnotationDefault;";

/// Original directory ordinals omitted by the frontend declaration representation.
#[derive(Clone)]
pub(crate) struct BackendClass {
    pub(crate) node: ClassNode,
    omitted_methods: Vec<usize>,
}

impl BackendClass {
    pub(crate) fn method_position(&self, ordinal: usize) -> Option<usize> {
        match self.omitted_methods.binary_search(&ordinal) {
            Ok(_) => None,
            Err(earlier_omissions) => Some(ordinal - earlier_omissions),
        }
    }
}

/// Build the frontend class node the recovery backend expects from the original
/// class facts. Fields the facts do not carry stay empty rather than inferred.
pub(crate) fn build(facts: &ClassFacts) -> Option<BackendClass> {
    let descriptor = String::from_utf16(facts.descriptor().units()).ok()?;
    let info = ClassInfo::from_type_descriptor(&descriptor).ok()?;
    let mut node = ClassNode::new(
        facts.class_def().0,
        info,
        AccessInfo::for_class(facts.access_flags().raw()),
    );
    if let Fact::Present(super_class) = facts.superclass() {
        if let Some(ty) = parse_type(super_class.units()) {
            node.set_super_class(ty);
        }
    }
    if let Fact::Present(interfaces) = facts.interfaces() {
        for interface in interfaces {
            if let Fact::Present(descriptor) = &interface.descriptor {
                if let Some(ty) = parse_type(descriptor.units()) {
                    node.add_interface(ty);
                }
            }
        }
    }
    let mut omitted_methods = Vec::new();
    for (ordinal, entry) in facts.methods().iter().enumerate() {
        if let Some(method) = method_node(&descriptor, entry) {
            node.add_method(method);
        } else {
            omitted_methods.push(ordinal);
        }
    }
    for entry in facts.fields() {
        let Fact::Present(declaration) = entry.declaration() else {
            continue;
        };
        let Ok(name) = String::from_utf16(declaration.name.units()) else {
            continue;
        };
        let Some(ty) = parse_type(declaration.descriptor.units()) else {
            continue;
        };
        let info = FieldInfo::new(descriptor.clone(), name, ty);
        let mut field = FieldNode::new(
            entry.field().0,
            info,
            AccessInfo::for_field(entry.access_flags().raw()),
        );
        field.default_preparation = entry.default_preparation();
        if let Some(scalar) = entry.scalar() {
            field.initial_value = scalar_value(scalar, field.field_type());
        }
        node.add_field(field);
    }
    apply_annotations(&mut node, facts);
    Some(BackendClass {
        node,
        omitted_methods,
    })
}

fn method_node(owner: &str, entry: &crate::p0::MethodDirectoryEntry) -> Option<MethodNode> {
    let Fact::Present(declaration) = entry.declaration() else {
        return None;
    };
    let info = MethodInfo::new(
        owner.to_owned(),
        String::from_utf16(declaration.name_units()).ok()?,
        declaration
            .parameter_types
            .iter()
            .map(|parameter| parse_type(parameter.units()))
            .collect::<Option<Vec<_>>>()?,
        parse_type(declaration.return_type.units())?,
    );
    Some(MethodNode::new(
        entry.method().0,
        info,
        AccessInfo::for_method(entry.access_flags().raw()),
    ))
}

fn apply_annotations(node: &mut ClassNode, facts: &ClassFacts) {
    if let Fact::Present(source_file) = facts.source_file() {
        if let Ok(source_file) = String::from_utf16(source_file.units()) {
            node.source_file = Some(source_file);
        }
    }

    let original = facts.annotations();
    if original.records().is_empty() {
        return;
    }
    let mut default_records = Vec::new();
    let mut class_annotations = Vec::new();
    let mut method_annotations: BTreeMap<u32, Vec<AnnotationNode>> = BTreeMap::new();
    let mut field_annotations: BTreeMap<u32, Vec<AnnotationNode>> = BTreeMap::new();
    let mut parameter_annotations: BTreeMap<u32, BTreeMap<u32, Vec<AnnotationNode>>> =
        BTreeMap::new();

    for record in original.records() {
        if record
            .descriptor()
            .iter()
            .copied()
            .eq(ANNOTATION_DEFAULT.encode_utf16())
        {
            default_records.push(record);
            continue;
        }
        let Some(annotation) = annotation(record, original) else {
            continue;
        };
        match record.location() {
            AnnotationLocation::Class => class_annotations.push(annotation),
            AnnotationLocation::Field(id) => {
                field_annotations.entry(id.0).or_default().push(annotation)
            }
            AnnotationLocation::Method(id) => {
                method_annotations.entry(id.0).or_default().push(annotation)
            }
            AnnotationLocation::Parameter { method, parameter } => {
                parameter_annotations
                    .entry(method.0)
                    .or_default()
                    .entry(parameter.0)
                    .or_default()
                    .push(annotation);
            }
        }
    }

    node.signature = signature(&class_annotations);
    node.metadata = class_metadata(&class_annotations);
    if let Some(enclosing) = &node.metadata.enclosing {
        if let Some(parent) = &enclosing.class_descriptor {
            node.set_parent_class(parent.clone());
        }
    }
    let member_classes = node.metadata.member_classes.clone();
    for class in member_classes {
        node.add_inner_class(class);
    }
    class_annotations.retain(|annotation| !is_system_annotation(annotation));
    node.annotations = class_annotations;

    for method in node.methods_mut() {
        if let Some(mut annotations) = method_annotations.remove(&method.id) {
            method.signature = signature(&annotations);
            method.throws = throws(&annotations);
            annotations.retain(|annotation| !is_system_annotation(annotation));
            method.annotations = annotations;
        }
        if let Some(parameters) = parameter_annotations.remove(&method.id) {
            let last = parameters.keys().next_back().copied().unwrap_or(0);
            method
                .parameter_annotations
                .resize(last as usize + 1, Vec::new());
            for (index, mut annotations) in parameters {
                annotations.retain(|annotation| !is_system_annotation(annotation));
                method.parameter_annotations[index as usize] = annotations;
            }
        }
    }
    for field in node.fields_mut() {
        if let Some(mut annotations) = field_annotations.remove(&field.id) {
            field.signature = signature(&annotations);
            annotations.retain(|annotation| !is_system_annotation(annotation));
            field.annotations = annotations;
        }
    }
    node.annotation_default_issue = apply_annotation_defaults(
        node,
        facts.type_index(),
        &default_records,
        original.references(),
    );
}

fn apply_annotation_defaults(
    node: &mut ClassNode,
    owner: TypeId,
    records: &[&AnnotationRecord],
    references: &[ValueReference],
) -> Option<Box<str>> {
    let mut class_records = records
        .iter()
        .copied()
        .filter(|record| record.location() == AnnotationLocation::Class);
    let mut class_issue = match (class_records.next(), class_records.next()) {
        (Some(_), Some(duplicate)) => Some(default_issue(
            duplicate.range().start,
            "appears more than once on the class",
        )),
        (Some(record), None) => apply_class_annotation_default(node, owner, record, references),
        (None, _) => None,
    };

    let mut misplaced_methods = BTreeMap::new();
    for record in records {
        match record.location() {
            AnnotationLocation::Class => continue,
            AnnotationLocation::Method(method) | AnnotationLocation::Parameter { method, .. } => {
                misplaced_methods
                    .entry(method.0)
                    .or_insert(record.range().start);
            }
            AnnotationLocation::Field(_) => {
                class_issue.get_or_insert_with(|| {
                    default_issue(record.range().start, "appears on a field")
                });
            }
        }
    }
    if !misplaced_methods.is_empty() {
        for method in node.methods_mut() {
            if let Some(offset) = misplaced_methods.remove(&method.id) {
                method.annotation_default_issue.get_or_insert_with(|| {
                    default_issue(offset, "appears on a method or parameter")
                });
            }
        }
        if let Some(offset) = misplaced_methods.into_values().next() {
            class_issue.get_or_insert_with(|| {
                default_issue(offset, "appears on an unavailable method or parameter")
            });
        }
    }
    class_issue
}

fn apply_class_annotation_default(
    node: &mut ClassNode,
    owner: TypeId,
    record: &AnnotationRecord,
    references: &[ValueReference],
) -> Option<Box<str>> {
    let malformed_envelope = || {
        Some(default_issue(
            record.range().start,
            "does not contain a single value annotation for the declaring type",
        ))
    };
    if record.visibility() != 2 || !node.is_annotation() || !node.is_interface() {
        return Some(default_issue(
            record.range().start,
            "requires system visibility on an annotation interface",
        ));
    }
    let tokens = record.tokens();
    let Some(AnnotationToken::Value(wrapper)) = tokens.first() else {
        return malformed_envelope();
    };
    let AnnotationValueKind::Annotation { type_id, .. } = wrapper.kind() else {
        return malformed_envelope();
    };
    if *type_id != owner
        || !wrapper
            .name()
            .is_some_and(|name| name.iter().copied().eq("value".encode_utf16()))
    {
        return malformed_envelope();
    }
    let mut elements = match (
        annotation_value_token_count(tokens, wrapper),
        record.extent(),
    ) {
        (Some(end), AnnotationExtent::Complete) if end == tokens.len() => &tokens[1..end - 1],
        (None, AnnotationExtent::Prefix) => &tokens[1..],
        _ => return malformed_envelope(),
    };

    let mut method_positions = BTreeMap::new();
    for (index, method) in node.methods().iter().enumerate() {
        if method.can_have_annotation_default() {
            method_positions
                .entry(method.name().to_owned())
                .and_modify(|position| *position = None)
                .or_insert(Some(index));
        }
    }

    let mut class_issue = None;
    while let Some(AnnotationToken::Value(element)) = elements.first() {
        // Each complete child is a separate declaration fact. A bad value must
        // not stop lookup of its siblings or consume another element's tokens.
        let end = annotation_value_token_count(elements, element);
        let offset = element.range().start;
        let name = element
            .name()
            .and_then(|name| String::from_utf16(name).ok());
        let position = name
            .as_ref()
            .and_then(|name| method_positions.get(name).copied().flatten());
        if let Some(position) = position {
            let method = &mut node.methods_mut()[position];
            if method.annotation_default.is_some() || method.annotation_default_issue.is_some() {
                method.annotation_default = None;
                method.annotation_default_issue =
                    Some(default_issue(offset, "repeats the annotation element name"));
            } else {
                let value = end.and_then(|end| {
                    let mut cursor = 1;
                    let value =
                        annotation_value(&elements[..end], &mut cursor, element, references, 0)?;
                    (cursor == end && value.matches_annotation_default(method.return_type()))
                        .then_some(value)
                });
                match (end, value) {
                    (_, Some(value)) => method.annotation_default = Some(Box::new(value)),
                    (None, None) => {
                        method.annotation_default_issue =
                            Some(default_issue(offset, "value is incomplete"));
                    }
                    (Some(_), None) => {
                        method.annotation_default_issue = Some(default_issue(
                            offset,
                            "value is invalid for the annotation element's return type",
                        ));
                    }
                }
            }
        } else {
            class_issue.get_or_insert_with(|| {
                default_issue(offset, "element has no unique matching annotation method")
            });
        }
        let Some(end) = end else { break };
        elements = &elements[end..];
    }
    if record.extent() == AnnotationExtent::Prefix && elements.is_empty() {
        class_issue.get_or_insert_with(|| {
            default_issue(
                record.range().end,
                "is incomplete after its last checked element",
            )
        });
    }
    class_issue
}

fn default_issue(offset: ByteOffset, reason: &str) -> Box<str> {
    format!("DEX annotation default {reason} at byte {}", offset.0).into_boxed_str()
}

fn annotation_value_token_count(
    tokens: &[AnnotationToken],
    value: &AnnotationValue,
) -> Option<usize> {
    let is_container = |value: &AnnotationValue| {
        matches!(
            value.kind(),
            AnnotationValueKind::Array { .. } | AnnotationValueKind::Annotation { .. }
        )
    };
    if !is_container(value) {
        return Some(1);
    }
    let mut depth = 1;
    for (index, token) in tokens.iter().enumerate().skip(1) {
        match token {
            AnnotationToken::EndContainer => {
                depth -= 1;
                if depth == 0 {
                    return Some(index + 1);
                }
            }
            AnnotationToken::Value(value) if is_container(value) => depth += 1,
            AnnotationToken::Value(_) => {}
        }
    }
    None
}

fn annotation(record: &AnnotationRecord, original: &OriginalAnnotations) -> Option<AnnotationNode> {
    let annotation_type = parse_type(record.descriptor())?;
    let visibility = match record.visibility() {
        0 => AnnotationVisibility::Build,
        1 => AnnotationVisibility::Runtime,
        2 => AnnotationVisibility::System,
        value => AnnotationVisibility::Unknown(value),
    };
    let mut position = 0;
    let elements = annotation_elements(record.tokens(), &mut position, original.references(), 0)?;
    Some(AnnotationNode {
        visibility,
        annotation_type,
        elements,
    })
}

fn annotation_elements(
    tokens: &[AnnotationToken],
    position: &mut usize,
    references: &[ValueReference],
    depth: usize,
) -> Option<Vec<AnnotationElement>> {
    if depth > 64 {
        return None;
    }
    let mut elements = Vec::new();
    while let Some(token) = tokens.get(*position) {
        match token {
            AnnotationToken::EndContainer => {
                *position += 1;
                break;
            }
            AnnotationToken::Value(value) => {
                *position += 1;
                let name = value
                    .name()
                    .map(String::from_utf16)
                    .transpose()
                    .ok()?
                    .unwrap_or_default();
                let value = annotation_value(tokens, position, value, references, depth + 1)?;
                elements.push(AnnotationElement { name, value });
            }
        }
    }
    Some(elements)
}

fn annotation_value(
    tokens: &[AnnotationToken],
    position: &mut usize,
    value: &crate::AnnotationValue,
    references: &[ValueReference],
    depth: usize,
) -> Option<DexValue> {
    if depth > 64 {
        return None;
    }
    let tag = value.tag();
    Some(match value.kind() {
        AnnotationValueKind::Null => DexValue::Null,
        AnnotationValueKind::Boolean(value) => DexValue::Boolean(*value),
        AnnotationValueKind::Signed(value) => signed_scalar(*value, tag),
        AnnotationValueKind::Unsigned(value) => unsigned_scalar(*value, tag),
        AnnotationValueKind::Bits { bits, .. } => match tag {
            0x10 => DexValue::Float(f32::from_bits(*bits as u32)),
            0x11 => DexValue::Double(f64::from_bits(*bits)),
            _ => DexValue::Unsupported {
                value_type: tag,
                raw: *bits,
            },
        },
        AnnotationValueKind::String { units, .. } => {
            DexValue::String(Utf16String::from_utf16(units.clone()))
        }
        AnnotationValueKind::Reference { declaration, .. } => reference(
            &references.get(declaration.0 as usize)?.declaration,
            value.tag(),
        ),
        AnnotationValueKind::Array { count } => {
            let mut values = Vec::new();
            for _ in 0..*count {
                let AnnotationToken::Value(value) = tokens.get(*position)? else {
                    return None;
                };
                *position += 1;
                values.push(annotation_value(
                    tokens,
                    position,
                    value,
                    references,
                    depth + 1,
                )?);
            }
            if !matches!(tokens.get(*position), Some(AnnotationToken::EndContainer)) {
                return None;
            }
            *position += 1;
            DexValue::Array(values)
        }
        AnnotationValueKind::Annotation { descriptor, .. } => {
            let annotation_type = parse_type(descriptor)?;
            let elements = annotation_elements(tokens, position, references, depth + 1)?;
            DexValue::Annotation(Box::new(AnnotationNode {
                visibility: AnnotationVisibility::System,
                annotation_type,
                elements,
            }))
        }
    })
}

fn reference(
    declaration: &Result<ValueDeclaration, crate::input::InputError>,
    tag: u8,
) -> DexValue {
    let Ok(declaration) = declaration else {
        return DexValue::Unsupported {
            value_type: tag,
            raw: 0,
        };
    };
    match declaration {
        ValueDeclaration::String(units) => DexValue::String(Utf16String::from_utf16(units.clone())),
        ValueDeclaration::Type(value) => parse_type(&value.descriptor)
            .map(DexValue::Type)
            .unwrap_or(DexValue::Unsupported {
                value_type: tag,
                raw: 0,
            }),
        ValueDeclaration::Member(ValueMember::Field {
            owner,
            name,
            field_type,
        }) => {
            let Some(owner) = parse_type(&owner.descriptor) else {
                return DexValue::Unsupported {
                    value_type: tag,
                    raw: 0,
                };
            };
            let Some(name) = String::from_utf16(name).ok() else {
                return DexValue::Unsupported {
                    value_type: tag,
                    raw: 0,
                };
            };
            let Some(field_type) = parse_type(&field_type.descriptor) else {
                return DexValue::Unsupported {
                    value_type: tag,
                    raw: 0,
                };
            };
            let field = FieldReference {
                owner,
                name,
                field_type,
            };
            if tag == 0x1b {
                DexValue::Enum(field)
            } else {
                DexValue::Field(field)
            }
        }
        ValueDeclaration::Member(ValueMember::Method {
            owner,
            name,
            prototype,
        }) => {
            let Some(owner) = parse_type(&owner.descriptor) else {
                return DexValue::Unsupported {
                    value_type: tag,
                    raw: 0,
                };
            };
            let Some(name) = String::from_utf16(name).ok() else {
                return DexValue::Unsupported {
                    value_type: tag,
                    raw: 0,
                };
            };
            let Some(descriptor) = method_descriptor(prototype) else {
                return DexValue::Unsupported {
                    value_type: tag,
                    raw: 0,
                };
            };
            DexValue::Method(MethodReference {
                owner,
                name,
                descriptor,
            })
        }
        ValueDeclaration::Prototype(prototype) => method_descriptor(prototype)
            .map(DexValue::MethodType)
            .unwrap_or(DexValue::Unsupported {
                value_type: tag,
                raw: 0,
            }),
        ValueDeclaration::Handle { .. } => DexValue::Unsupported {
            value_type: tag,
            raw: 0,
        },
    }
}

fn method_descriptor(prototype: &ValuePrototype) -> Option<MethodDescriptor> {
    let mut parameters = Vec::new();
    for parameter in &prototype.parameters {
        parameters.push(parse_type(&parameter.descriptor)?);
    }
    Some(MethodDescriptor {
        parameters,
        return_type: parse_type(&prototype.returns.descriptor)?,
    })
}

fn signed_scalar(value: i64, tag: u8) -> DexValue {
    match tag {
        0x00 => DexValue::Byte(value as i8),
        0x02 => DexValue::Short(value as i16),
        0x03 => DexValue::Char(value as u16),
        0x04 => DexValue::Int(value as i32),
        0x06 => DexValue::Long(value),
        _ => DexValue::Unsupported {
            value_type: tag,
            raw: value as u64,
        },
    }
}

fn unsigned_scalar(value: u64, tag: u8) -> DexValue {
    match tag {
        0x03 => DexValue::Char(value as u16),
        0x04 => DexValue::Int(value as i32),
        0x06 => DexValue::Long(value as i64),
        _ => DexValue::Unsupported {
            value_type: tag,
            raw: value,
        },
    }
}

fn is_system_annotation(annotation: &AnnotationNode) -> bool {
    matches!(&annotation.annotation_type, ArgType::Object(name) if name.starts_with("dalvik/annotation/"))
}

fn signature(annotations: &[AnnotationNode]) -> Option<String> {
    annotation_string(annotations, "Ldalvik/annotation/Signature;")
}

fn throws(annotations: &[AnnotationNode]) -> Vec<ArgType> {
    let Some(value) = annotation_value_named(annotations, "Ldalvik/annotation/Throws;", "value")
    else {
        return Vec::new();
    };
    match value {
        DexValue::Array(values) => values
            .iter()
            .filter_map(|value| match value {
                DexValue::Type(ty) => Some(ty.clone()),
                _ => None,
            })
            .collect(),
        DexValue::Type(ty) => vec![ty.clone()],
        _ => Vec::new(),
    }
}

fn annotation_string(annotations: &[AnnotationNode], name: &str) -> Option<String> {
    let value = annotation_value_named(annotations, name, "value")?;
    match value {
        DexValue::String(value) => Some(value.to_string_lossy()),
        DexValue::Array(values) => {
            let mut result = String::new();
            for value in values {
                let DexValue::String(value) = value else {
                    return None;
                };
                result.push_str(&value.to_string_lossy());
            }
            Some(result)
        }
        _ => None,
    }
}

fn annotation_value_named<'a>(
    annotations: &'a [AnnotationNode],
    annotation: &str,
    element: &str,
) -> Option<&'a DexValue> {
    let name = annotation.strip_prefix('L')?.strip_suffix(';')?;
    annotations
        .iter()
        .find(|candidate| matches!(&candidate.annotation_type, ArgType::Object(owner) if owner == name))
        .and_then(|candidate| {
            candidate
                .elements
                .iter()
                .find(|element_value| element_value.name == element)
        })
        .map(|element| &element.value)
}

fn class_metadata(annotations: &[AnnotationNode]) -> ClassMetadata {
    let mut metadata = ClassMetadata::default();
    if let Some(value) =
        annotation_value_named(annotations, "Ldalvik/annotation/InnerClass;", "accessFlags")
    {
        let access_flags_raw = match value {
            DexValue::Int(value) => *value as u32,
            _ => 0,
        };
        let simple_name =
            annotation_value_named(annotations, "Ldalvik/annotation/InnerClass;", "name").and_then(
                |value| match value {
                    DexValue::String(value) => Some(value.to_string_lossy()),
                    _ => None,
                },
            );
        metadata.inner_class = Some(InnerClassInfo {
            simple_name,
            access_flags_raw,
        });
    }
    if let Some(DexValue::Type(value)) =
        annotation_value_named(annotations, "Ldalvik/annotation/EnclosingClass;", "value")
    {
        metadata.enclosing = Some(EnclosingInfo {
            class_descriptor: Some(value.to_descriptor()),
            method_reference: None,
            method_static: None,
        });
    }
    if let Some(DexValue::Method(value)) =
        annotation_value_named(annotations, "Ldalvik/annotation/EnclosingMethod;", "value")
    {
        metadata.enclosing = Some(EnclosingInfo {
            class_descriptor: Some(value.owner.to_descriptor()),
            method_reference: Some(value.to_string()),
            method_static: None,
        });
    }
    if let Some(DexValue::Array(values)) =
        annotation_value_named(annotations, "Ldalvik/annotation/MemberClasses;", "value")
    {
        for value in values {
            if let DexValue::Type(value) = value {
                metadata.member_classes.push(value.to_descriptor());
            }
        }
    }
    metadata
}

fn scalar_value(value: &ScalarValue, ty: &ArgType) -> Option<DexValue> {
    match value {
        ScalarValue::Signed(value) => match ty {
            ArgType::Primitive(crate::ir::PrimitiveType::Byte) => {
                Some(DexValue::Byte(*value as i8))
            }
            ArgType::Primitive(crate::ir::PrimitiveType::Short) => {
                Some(DexValue::Short(*value as i16))
            }
            ArgType::Primitive(crate::ir::PrimitiveType::Int) => Some(DexValue::Int(*value as i32)),
            ArgType::Primitive(crate::ir::PrimitiveType::Long) => Some(DexValue::Long(*value)),
            _ => None,
        },
        ScalarValue::Unsigned(value) => match ty {
            ArgType::Primitive(crate::ir::PrimitiveType::Char) => {
                Some(DexValue::Char(*value as u16))
            }
            ArgType::Primitive(crate::ir::PrimitiveType::Int) => Some(DexValue::Int(*value as i32)),
            _ => None,
        },
        ScalarValue::FloatBits(value) => Some(DexValue::Float(f32::from_bits(*value))),
        ScalarValue::DoubleBits(value) => Some(DexValue::Double(f64::from_bits(*value))),
        ScalarValue::Boolean(value) => Some(DexValue::Boolean(*value)),
        ScalarValue::Null => Some(DexValue::Null),
        ScalarValue::String(value) => Some(DexValue::String(Utf16String::from_utf16(
            value.units().to_vec(),
        ))),
    }
}

/// Parse one original descriptor into the frontend argument type.
fn parse_type(units: &[u16]) -> Option<ArgType> {
    String::from_utf16(units).ok()?.parse().ok()
}
