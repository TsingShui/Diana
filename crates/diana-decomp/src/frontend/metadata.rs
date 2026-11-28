//! Typed DEX metadata values and annotation nodes shared by declarations.

use crate::ir::{
    ArgType, DescriptorParseError, FieldReference, MethodDescriptor, MethodReference,
    ReferenceParseError, Utf16String,
};

#[derive(Debug, Clone)]
pub enum MetadataConversionError {
    Type {
        descriptor: String,
        source: DescriptorParseError,
    },
    Field {
        reference: String,
        source: ReferenceParseError,
    },
    Method {
        reference: String,
        source: ReferenceParseError,
    },
    MethodType {
        descriptor: String,
        source: DescriptorParseError,
    },
}

impl std::fmt::Display for MetadataConversionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Type { descriptor, source } => {
                write!(formatter, "invalid DEX type value {descriptor}: {source}")
            }
            Self::Field { reference, source } => {
                write!(formatter, "invalid DEX field value {reference}: {source}")
            }
            Self::Method { reference, source } => {
                write!(formatter, "invalid DEX method value {reference}: {source}")
            }
            Self::MethodType { descriptor, source } => {
                write!(
                    formatter,
                    "invalid DEX method type value {descriptor}: {source}"
                )
            }
        }
    }
}

impl std::error::Error for MetadataConversionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Type { source, .. } | Self::MethodType { source, .. } => Some(source),
            Self::Field { source, .. } | Self::Method { source, .. } => Some(source),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnnotationNode {
    pub visibility: AnnotationVisibility,
    pub annotation_type: ArgType,
    pub elements: Vec<AnnotationElement>,
}

impl AnnotationNode {
    pub fn is_java_source_annotation(&self) -> bool {
        matches!(
            self.visibility,
            AnnotationVisibility::Build | AnnotationVisibility::Runtime
        ) && !is_dalvik_system_annotation(&self.annotation_type.to_string())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnnotationVisibility {
    Build,
    Runtime,
    System,
    Unknown(u8),
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnnotationElement {
    pub name: String,
    pub value: DexValue,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DexValue {
    Null,
    Boolean(bool),
    Byte(i8),
    Short(i16),
    Char(u16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    String(Utf16String),
    Type(ArgType),
    Field(FieldReference),
    Method(MethodReference),
    MethodType(MethodDescriptor),
    Enum(FieldReference),
    Array(Vec<DexValue>),
    Annotation(Box<AnnotationNode>),
    Unsupported { value_type: u8, raw: u64 },
}

impl DexValue {
    pub(crate) fn matches_annotation_default(&self, ty: &ArgType) -> bool {
        match self {
            Self::Boolean(_) => ty == &ArgType::BOOLEAN,
            Self::Byte(_) => ty == &ArgType::BYTE,
            Self::Short(_) => ty == &ArgType::SHORT,
            Self::Char(_) => ty == &ArgType::CHAR,
            Self::Int(_) => ty == &ArgType::INT,
            Self::Long(_) => ty == &ArgType::LONG,
            Self::Float(_) => ty == &ArgType::FLOAT,
            Self::Double(_) => ty == &ArgType::DOUBLE,
            Self::String(_) => ty.as_object() == Some("java/lang/String"),
            Self::Type(_) => ty.as_object() == Some("java/lang/Class"),
            Self::Enum(field) => &field.owner == ty && &field.field_type == ty,
            Self::Annotation(annotation) => {
                &annotation.annotation_type == ty && self.is_annotation_constant()
            }
            Self::Array(values) => ty.as_array_element().is_some_and(|element| {
                (element.is_object()
                    || matches!(element.as_primitive(),
                        Some(primitive) if primitive == crate::ir::PrimitiveType::Boolean
                            || primitive.is_integral()
                            || matches!(primitive, crate::ir::PrimitiveType::Float | crate::ir::PrimitiveType::Double)))
                    && values.iter().all(|value| value.matches_annotation_default(element))
            }),
            Self::Null | Self::Field(_) | Self::Method(_) | Self::MethodType(_) | Self::Unsupported { .. } => false,
        }
    }

    fn is_annotation_constant(&self) -> bool {
        match self {
            Self::Null
            | Self::Field(_)
            | Self::Method(_)
            | Self::MethodType(_)
            | Self::Unsupported { .. } => false,
            Self::Enum(field) => field.owner == field.field_type && field.owner.is_object(),
            Self::Array(values) => values
                .iter()
                .all(|value| !matches!(value, Self::Array(_)) && value.is_annotation_constant()),
            Self::Annotation(annotation) => {
                let mut names = std::collections::BTreeSet::new();
                annotation.annotation_type.is_object()
                    && annotation.elements.iter().all(|element| {
                        !element.name.is_empty()
                            && names.insert(element.name.as_str())
                            && element.value.is_annotation_constant()
                    })
            }
            _ => true,
        }
    }
}

/// Whether an annotation type belongs to the Dalvik system namespace.
fn is_dalvik_system_annotation(annotation_type: &str) -> bool {
    annotation_type.starts_with("Ldalvik/annotation/")
}
