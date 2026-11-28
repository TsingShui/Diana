//! Adapt checked JVM metadata to the shared Java declaration renderer.

use crate::{
    frontend::{AnnotationElement, AnnotationNode, AnnotationVisibility, DexValue},
    input::jvm::{
        annotations::{Annotation, ElementValue, Target, Value, Visibility},
        ClassFile, JvmString, Member,
    },
    ir::{ArgType, FieldReference, Utf16String},
    FailureKind,
};

use super::{declaration_text, malformed, SourceError, SourceIssue};

pub(super) enum Owner<'a> {
    Class,
    Field(Member<'a>),
    Method {
        member: Member<'a>,
        parameters: usize,
        default_return_type: Option<&'a ArgType>,
    },
}

#[derive(Default)]
pub(super) struct SourceAnnotations {
    pub declaration: Vec<AnnotationNode>,
    pub parameters: Vec<Vec<AnnotationNode>>,
    pub default: Option<Box<DexValue>>,
}

impl<'a> Owner<'a> {
    fn member(&self) -> Option<Member<'a>> {
        match self {
            Self::Class => None,
            Self::Field(member) | Self::Method { member, .. } => Some(*member),
        }
    }

    fn issue(&self, issues: &mut Vec<SourceIssue>, error: SourceError) {
        let (field, method) = match self {
            Self::Class => (None, None),
            Self::Field(member) => (Some(member.ordinal()), None),
            Self::Method { member, .. } => (None, Some(member.ordinal())),
        };
        issues.push(SourceIssue::declaration(field, method, error));
    }

    fn unsupported(&self, issues: &mut Vec<SourceIssue>, offset: usize, metadata: &str) {
        self.issue(
            issues,
            SourceError::Operation {
                kind: FailureKind::Unsupported,
                reason: format!(
                    "{metadata} at JVM class-file byte {offset} is unavailable in Java source"
                ),
            },
        );
    }
}

pub(super) fn read(
    class: &ClassFile<'_>,
    owner: Owner<'_>,
    issues: &mut Vec<SourceIssue>,
) -> SourceAnnotations {
    let mut output = SourceAnnotations::default();
    let facts = match class.annotation_facts(owner.member()) {
        Ok(facts) => facts,
        Err(error) => {
            owner.issue(issues, error.into());
            return output;
        }
    };
    for error in facts.issues {
        owner.issue(issues, error.into());
    }
    if let Some(default) = facts
        .source_default
        .and_then(|index| facts.defaults.into_iter().nth(index))
    {
        let return_type = match &owner {
            Owner::Method {
                default_return_type,
                ..
            } => *default_return_type,
            _ => None,
        };
        let offset = default.offset;
        let issue_count = issues.len();
        if let Some(ty) = return_type {
            let value = convert_value(default, &mut |error| owner.issue(issues, error));
            if issues.len() == issue_count {
                output.default = value
                    .filter(|value| value.matches_annotation_default(ty))
                    .map(Box::new);
            }
        }
        if output.default.is_none() && issues.len() == issue_count {
            owner.issue(issues, malformed(format!(
                "annotation default at JVM class-file byte {offset} does not match its annotation element"
            )));
        }
    }
    for entry in facts.annotations {
        let slot = match entry.target {
            Target::Declaration => None,
            Target::Parameter {
                descriptor_index, ..
            } => {
                let Some(index) = descriptor_index.map(usize::from) else {
                    continue;
                };
                let Owner::Method { parameters, .. } = owner else {
                    continue;
                };
                if index >= parameters {
                    owner.issue(
                        issues,
                        malformed("annotation parameter is outside the original method descriptor"),
                    );
                    continue;
                }
                if output.parameters.is_empty() {
                    output.parameters.resize_with(parameters, Vec::new);
                }
                Some(index)
            }
            Target::TypeUse { .. } => {
                owner.unsupported(
                    issues,
                    entry.annotation.offset,
                    "type-use annotation placement",
                );
                continue;
            }
        };
        let mut issue = |error| owner.issue(issues, error);
        let Some(annotation) = convert_annotation(entry.annotation, &mut issue) else {
            continue;
        };
        if let Some(index) = slot {
            output.parameters[index].push(annotation);
        } else {
            output.declaration.push(annotation);
        }
    }
    output
}

fn convert_annotation(
    annotation: Annotation<'_>,
    issue: &mut impl FnMut(SourceError),
) -> Option<AnnotationNode> {
    let descriptor = annotation.descriptor?;
    let annotation_type = match type_value(descriptor) {
        Ok(ty) => ty,
        Err(error) => {
            issue(error);
            return None;
        }
    };
    let mut elements = Vec::new();
    for element in annotation.elements {
        let Some(encoded_name) = element.name else {
            continue;
        };
        let name = match declaration_text(encoded_name) {
            Ok(name) => name,
            Err(error) => {
                issue(error);
                continue;
            }
        };
        if let Some(value) = convert_value(element.value, issue) {
            elements.push(AnnotationElement { name, value });
        }
    }
    Some(AnnotationNode {
        visibility: match annotation.visibility {
            Visibility::RuntimeVisible => AnnotationVisibility::Runtime,
            Visibility::RuntimeInvisible => AnnotationVisibility::Build,
        },
        annotation_type,
        elements,
    })
}

fn convert_value(
    element: ElementValue<'_>,
    issue: &mut impl FnMut(SourceError),
) -> Option<DexValue> {
    let result = match element.value {
        Value::Unavailable => return None,
        Value::Byte(value) => Ok(DexValue::Byte(value)),
        Value::Char(value) => Ok(DexValue::Char(value)),
        Value::Double(bits) => Ok(DexValue::Double(f64::from_bits(bits))),
        Value::Float(bits) => Ok(DexValue::Float(f32::from_bits(bits))),
        Value::Int(value) => Ok(DexValue::Int(value)),
        Value::Long(value) => Ok(DexValue::Long(value)),
        Value::Short(value) => Ok(DexValue::Short(value)),
        Value::Boolean(value) => Ok(DexValue::Boolean(value)),
        Value::String(text) => text
            .to_utf16()
            .map_err(SourceError::from)
            .map(|units| DexValue::String(Utf16String::from_utf16(units))),
        Value::Class(descriptor) => type_value(descriptor).map(DexValue::Type),
        Value::Enum { descriptor, name } => {
            let name = name?;
            type_value(descriptor?).and_then(|field_type| {
                let name = declaration_text(name)?;
                Ok(DexValue::Enum(FieldReference {
                    owner: field_type.clone(),
                    name,
                    field_type,
                }))
            })
        }
        Value::Annotation(annotation) => {
            return convert_annotation(*annotation, issue)
                .map(|annotation| DexValue::Annotation(Box::new(annotation)))
        }
        Value::Array { values, complete } => {
            if !complete {
                return None;
            }
            // An unavailable element leaves the array value incomplete. The
            // original fact reader still exposes every checked element and its
            // class-file offset independently to reference queries.
            let mut converted = Vec::new();
            for item in values {
                converted.push(convert_value(item, issue)?);
            }
            Ok(DexValue::Array(converted))
        }
    };
    match result {
        Ok(value) => Some(value),
        Err(error) => {
            issue(SourceError::Operation {
                kind: error.kind(),
                reason: format!(
                    "annotation value at JVM class-file byte {}: {error}",
                    element.offset
                ),
            });
            None
        }
    }
}

fn type_value(descriptor: JvmString<'_>) -> Result<ArgType, SourceError> {
    declaration_text(descriptor)?.parse().map_err(malformed)
}
