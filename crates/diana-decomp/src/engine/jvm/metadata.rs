//! Optional JVM source attributes. Descriptors remain authoritative.

use std::collections::BTreeMap;

use crate::frontend::{ClassNode, DebugInfo, LocalVarInfo, MethodNode};
use crate::input::jvm::{bytecode::Code, Attribute, Attributes, ClassFile, Member};
use crate::ir::{
    generic_types::{
        ClassSignature, ClassTypeSignature, GenericSignatures, JvmTypeSignature, TypeArgument,
        TypeParameter,
    },
    ArgType,
};
use crate::FailureKind;

use super::{declaration_text, malformed, SourceError, SourceIssue};

pub(super) fn class_signature(
    class: &ClassFile<'_>,
    node: &mut ClassNode,
    issues: &mut Vec<SourceIssue>,
) -> Option<ClassSignature> {
    let result = (|| {
        let Some(text) = signature(class, class.attributes()?)? else {
            return Ok(None);
        };
        let parsed = GenericSignatures::class(&text).map_err(malformed)?;
        if node.super_class.as_ref() != Some(&parsed.super_class.erased_type())
            || !parsed
                .super_interfaces
                .iter()
                .map(|ty| ty.erased_type())
                .eq(node.interfaces.iter().cloned())
        {
            return Err(malformed(
                "Signature contradicts the original class hierarchy",
            ));
        }
        let visible = |name: &str| class_variable(&parsed, name);
        if parsed
            .type_parameters
            .iter()
            .flat_map(|parameter| {
                parameter
                    .class_bound
                    .iter()
                    .chain(&parameter.interface_bounds)
            })
            .any(|ty| !type_variables_visible(ty, &visible))
            || std::iter::once(&parsed.super_class)
                .chain(&parsed.super_interfaces)
                .any(|ty| !class_arguments_visible(ty, &visible))
        {
            unresolved_signature(None, None, issues);
            return Ok(None);
        }
        node.signature = Some(text);
        Ok(Some(parsed))
    })();
    declaration(result, None, None, issues)
}

pub(super) fn field_signature(
    class: &ClassFile<'_>,
    field: Member<'_>,
    ty: &ArgType,
    scope: Option<&ClassSignature>,
    issues: &mut Vec<SourceIssue>,
) -> Option<String> {
    let result = (|| {
        let Some(text) = signature(class, field.attributes())? else {
            return Ok(None);
        };
        let parsed = GenericSignatures::field(&text).map_err(malformed)?;
        if !type_variables_visible(&parsed, &|name| {
            scope.is_some_and(|class| class_variable(class, name))
        }) {
            unresolved_signature(Some(field.ordinal()), None, issues);
            return Ok(None);
        }
        if let Some(erased) = erase(&parsed, &[], scope) {
            if erased != *ty {
                return Err(malformed(
                    "Signature contradicts the original field descriptor",
                ));
            }
        } else {
            unresolved_signature(Some(field.ordinal()), None, issues);
            return Ok(None);
        }
        Ok(Some(text))
    })();
    declaration(result, Some(field.ordinal()), None, issues)
}

pub(super) fn method_signature(
    class: &ClassFile<'_>,
    member: Member<'_>,
    method: &MethodNode,
    scope: Option<&ClassSignature>,
    issues: &mut Vec<SourceIssue>,
) -> Option<String> {
    let result = (|| {
        let Some(text) = signature(class, member.attributes())? else {
            return Ok(None);
        };
        let parsed = GenericSignatures::method(&text).map_err(malformed)?;
        if parsed.parameter_types.len() != method.param_types().len() {
            // Signature can omit implicit JVM parameters. This class alone
            // does not always establish their source-to-descriptor projection.
            issues.push(SourceIssue {
                method: Some(member.ordinal()),
                field: None,
                offset_byte: None,
                end_offset_byte: None,
                stage: "JVM declaration",
                kind: FailureKind::Unsupported,
                reason: "Signature does not identify every original descriptor parameter".into(),
            });
            return Ok(None);
        }
        if parsed.has_unbound_type_variables(scope.into_iter().flat_map(|class| {
            class
                .type_parameters
                .iter()
                .map(|parameter| parameter.name.as_str())
        })) {
            unresolved_signature(None, Some(member.ordinal()), issues);
            return Ok(None);
        }
        for (generic, descriptor) in parsed
            .parameter_types
            .iter()
            .zip(method.param_types())
            .chain([(&parsed.return_type, method.return_type())])
        {
            if let Some(erased) = erase(generic, &parsed.type_parameters, scope) {
                if erased != *descriptor {
                    return Err(malformed(
                        "Signature contradicts the original method descriptor",
                    ));
                }
            } else {
                unresolved_signature(None, Some(member.ordinal()), issues);
                return Ok(None);
            }
        }
        Ok(Some(text))
    })();
    declaration(result, None, Some(member.ordinal()), issues)
}

fn signature<'a>(
    class: &ClassFile<'a>,
    attributes: Attributes<'a>,
) -> Result<Option<String>, SourceError> {
    class
        .signature(attributes)?
        .map(declaration_text)
        .transpose()
}

fn declaration<T>(
    result: Result<Option<T>, SourceError>,
    field: Option<u16>,
    method: Option<u16>,
    issues: &mut Vec<SourceIssue>,
) -> Option<T> {
    match result {
        Ok(value) => value,
        Err(error) => {
            issues.push(SourceIssue::declaration(field, method, error));
            None
        }
    }
}

fn unresolved_signature(field: Option<u16>, method: Option<u16>, issues: &mut Vec<SourceIssue>) {
    issues.push(SourceIssue {
        method,
        field,
        offset_byte: None,
        end_offset_byte: None,
        stage: "JVM declaration",
        kind: FailureKind::Unsupported,
        reason: "Signature erasure depends on unavailable type-variable bounds".into(),
    });
}

fn class_variable(scope: &ClassSignature, name: &str) -> bool {
    scope
        .type_parameters
        .iter()
        .any(|parameter| parameter.name == name)
}

fn type_variables_visible(ty: &JvmTypeSignature, visible: &impl Fn(&str) -> bool) -> bool {
    match ty {
        JvmTypeSignature::ClassType(class) => class_arguments_visible(class, visible),
        JvmTypeSignature::TypeVariable(name) => visible(name),
        JvmTypeSignature::Array(element) => type_variables_visible(element, visible),
        JvmTypeSignature::BaseType(_) => true,
    }
}

fn class_arguments_visible(ty: &ClassTypeSignature, visible: &impl Fn(&str) -> bool) -> bool {
    ty.type_arguments
        .iter()
        .chain(
            ty.inner_segments
                .iter()
                .flat_map(|segment| &segment.type_arguments),
        )
        .all(|argument| match argument {
            TypeArgument::Exact(ty) | TypeArgument::Extends(ty) | TypeArgument::Super(ty) => {
                type_variables_visible(ty, visible)
            }
            TypeArgument::Unbounded => true,
        })
}

/// JVM erasure uses the first bound in the declaring lexical scope. A method
/// type parameter shadows a class parameter with the same name.
fn erase<'a>(
    mut ty: &'a JvmTypeSignature,
    method: &'a [TypeParameter],
    class: Option<&'a ClassSignature>,
) -> Option<ArgType> {
    let class = class.map_or(&[][..], |scope| scope.type_parameters.as_slice());
    let mut method_scope = method;
    let mut aliases = method.len().saturating_add(class.len());
    let mut dimensions = 0;
    let erased = loop {
        match ty {
            JvmTypeSignature::Array(element) if dimensions < 255 => {
                dimensions += 1;
                ty = element;
            }
            JvmTypeSignature::Array(_) => return None,
            JvmTypeSignature::TypeVariable(name) => {
                aliases = aliases.checked_sub(1)?;
                let bound = match method_scope.iter().find(|item| item.name == *name) {
                    Some(parameter) => parameter,
                    None => {
                        method_scope = &[];
                        class.iter().find(|item| item.name == *name)?
                    }
                };
                match bound
                    .class_bound
                    .as_ref()
                    .or_else(|| bound.interface_bounds.first())
                {
                    Some(bound) => ty = bound,
                    None => break ArgType::object("java/lang/Object"),
                }
            }
            _ => break ty.erased(),
        }
    };
    Some((0..dimensions).fold(erased, |ty, _| ArgType::array(ty)))
}

pub(super) fn parameter_names(
    class: &ClassFile<'_>,
    member: Member<'_>,
    method: &MethodNode,
    issues: &mut Vec<SourceIssue>,
) -> Vec<Option<String>> {
    let ordinal = member.ordinal();
    let result = class
        .method_parameters(member, method.param_types().len())
        .map_err(SourceError::from);
    let Some(table) = declaration(result, None, Some(ordinal), issues) else {
        return Vec::new();
    };
    if let Some(error) = table.issue() {
        issues.push(SourceIssue::declaration(None, Some(ordinal), error.into()));
    }
    table
        .entries()
        .map(|entry| {
            let index = entry.name_index;
            let result = (index != 0).then(|| local_name(class, index)).transpose();
            declaration(result, None, Some(ordinal), issues)
        })
        .collect()
}

pub(crate) struct LocalCoordinates {
    pub register: u32,
    pub start: u32,
    pub end: u32,
}

pub(crate) fn debug_info(
    class: &ClassFile<'_>,
    code: Code<'_>,
    method: &MethodNode,
    inputs: &[(u16, ArgType)],
    location: impl Fn(u16, u16) -> Option<LocalCoordinates>,
    issues: &mut Vec<SourceIssue>,
) -> Option<DebugInfo> {
    let ordinal = method.id as u16;
    let candidates = inputs.iter().skip(usize::from(!method.is_static()));
    let mut arguments = None;
    if candidates.len() != 0 {
        match code.attributes() {
            Ok(attributes) => {
                for attribute in attributes {
                    match attribute
                        .and_then(|attribute| Ok((class.utf8(attribute.name_index)?, attribute)))
                    {
                        Ok((name, attribute)) if name.bytes() == b"LocalVariableTable" => {
                            let entries = arguments.get_or_insert_with(|| {
                                candidates
                                    .clone()
                                    .enumerate()
                                    .map(|(index, (slot, ty))| {
                                        (
                                            *slot,
                                            EntryLocal {
                                                index,
                                                ty,
                                                local: None,
                                                invalid: false,
                                                conflict: false,
                                            },
                                        )
                                    })
                                    .collect()
                            });
                            read_local_table(class, attribute, ordinal, &location, entries, issues);
                        }
                        Ok(_) => (),
                        Err(error) => metadata_issue(ordinal, error, issues),
                    }
                }
            }
            Err(error) if code.missing_bytes().is_none() => metadata_issue(ordinal, error, issues),
            Err(_) => (),
        }
    }
    let mut info = DebugInfo {
        param_names: method.parameter_names.clone(),
        ..DebugInfo::default()
    };
    for entry in arguments.into_iter().flat_map(BTreeMap::into_values) {
        if entry.conflict {
            metadata_issue(
                ordinal,
                "conflicting entry-scope parameter names in LocalVariableTable",
                issues,
            );
            continue;
        }
        if let Some(local) = entry.local {
            if info
                .param_names
                .get(entry.index)
                .is_none_or(Option::is_none)
            {
                info.param_names.resize(method.param_types().len(), None);
                info.param_names[entry.index] = Some(local.name.clone());
            }
            info.local_vars.push(local);
        }
    }
    (!info.local_vars.is_empty() || !info.param_names.is_empty()).then_some(info)
}

struct EntryLocal<'a> {
    index: usize,
    ty: &'a ArgType,
    local: Option<LocalVarInfo>,
    invalid: bool,
    conflict: bool,
}

fn read_local_table(
    class: &ClassFile<'_>,
    attribute: Attribute<'_>,
    ordinal: u16,
    location: &impl Fn(u16, u16) -> Option<LocalCoordinates>,
    arguments: &mut BTreeMap<u16, EntryLocal<'_>>,
    issues: &mut Vec<SourceIssue>,
) {
    let Some((count, bytes)) = attribute.payload.split_first_chunk::<2>() else {
        metadata_issue(ordinal, "short LocalVariableTable attribute", issues);
        return;
    };
    let count = usize::from(u16::from_be_bytes(*count));
    if count * 10 != bytes.len() {
        metadata_issue(
            ordinal,
            "invalid LocalVariableTable attribute length",
            issues,
        );
    }
    for &[a, b, c, d, e, f, g, h, i, j] in bytes.as_chunks::<10>().0.iter().take(count) {
        // This attribute also names interior lifetimes. Only an original
        // lifetime covering entry can identify an incoming parameter.
        if a != 0 || b != 0 || (c == 0 && d == 0) {
            continue;
        }
        let slot = u16::from_be_bytes([i, j]);
        let Some(entry) = arguments.get_mut(&slot).filter(|entry| !entry.conflict) else {
            continue;
        };
        let end = u16::from_be_bytes([c, d]);
        let local = read_entry_local(
            class,
            slot,
            end,
            u16::from_be_bytes([e, f]),
            u16::from_be_bytes([g, h]),
            entry.ty,
            location,
        );
        match local {
            Ok(local) => match &mut entry.local {
                Some(previous) if previous.name == local.name => {
                    previous.end_addr = previous.end_addr.min(local.end_addr);
                }
                Some(_) => entry.conflict = true,
                None => entry.local = Some(local),
            },
            Err(error) if !entry.invalid => {
                entry.invalid = true;
                metadata_issue(ordinal, error, issues);
            }
            Err(_) => (),
        }
    }
}

fn read_entry_local(
    class: &ClassFile<'_>,
    slot: u16,
    end: u16,
    name: u16,
    descriptor: u16,
    ty: &ArgType,
    location: &impl Fn(u16, u16) -> Option<LocalCoordinates>,
) -> Result<LocalVarInfo, SourceError> {
    let coordinates = location(slot, end)
        .ok_or_else(|| malformed("LocalVariableTable range has no original JVM coordinate"))?;
    let descriptor = declaration_text(class.utf8(descriptor)?)?;
    let observed: ArgType = descriptor.parse().map_err(malformed)?;
    if observed != *ty {
        return Err(malformed(
            "LocalVariableTable descriptor contradicts the original parameter",
        ));
    }
    Ok(LocalVarInfo {
        name: local_name(class, name)?,
        var_type: observed,
        start_addr: coordinates.start,
        end_addr: coordinates.end,
        register: coordinates.register,
    })
}

fn local_name(class: &ClassFile<'_>, index: u16) -> Result<String, SourceError> {
    let value = class.utf8(index)?;
    if value.bytes().is_empty()
        || value
            .bytes()
            .iter()
            .any(|byte| matches!(byte, b'.' | b';' | b'[' | b'/'))
    {
        return Err(malformed("invalid JVM debug variable name"));
    }
    declaration_text(value)
}

fn metadata_issue(ordinal: u16, reason: impl std::fmt::Display, issues: &mut Vec<SourceIssue>) {
    issues.push(SourceIssue {
        method: Some(ordinal),
        field: None,
        offset_byte: None,
        end_offset_byte: None,
        stage: "JVM debug metadata",
        kind: FailureKind::Malformed,
        reason: reason.to_string(),
    });
}
