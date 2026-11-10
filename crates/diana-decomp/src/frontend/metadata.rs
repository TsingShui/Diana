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

/// Whether an annotation type belongs to the Dalvik system namespace.
fn is_dalvik_system_annotation(annotation_type: &str) -> bool {
    annotation_type.starts_with("Ldalvik/annotation/")
}
