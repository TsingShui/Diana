//! Classified reader failures shared by the frontend and the recovery entry.

use super::ClassInfoError;
use crate::ir::ty::DescriptorParseError;

/// Error type for DEX reading operations
#[derive(Debug)]
pub enum DexError {
    /// IO error while reading file
    IoError(std::io::Error),
    ClassInfo(ClassInfoError),
    MalformedMethodPrototype(String),
    MissingDexForClass(String),
    InvalidMemberReference {
        reference: String,
        source: crate::ir::ReferenceParseError,
    },
    InvalidInvokeRegisters {
        offset: u32,
        expected: usize,
        actual: usize,
    },
    OverrideAnalysis(crate::analysis::method_override::OverrideAnalysisError),
    InvalidDescriptor {
        descriptor: String,
        source: DescriptorParseError,
    },
    /// Language backend failure
    Backend(String),
    /// Invalid class index
    InvalidClassIndex(u32),
    /// Invalid method index
    InvalidMethodIndex(u32),
    /// Invalid field index
    InvalidFieldIndex(u32),
    /// Invalid type index
    InvalidTypeIndex(u32),
    /// Invalid string index
    InvalidStringIndex(u32),
}

impl std::fmt::Display for DexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DexError::IoError(e) => write!(f, "IO error: {}", e),
            DexError::ClassInfo(source) => write!(f, "invalid class metadata: {source}"),
            DexError::Backend(message) => write!(f, "backend failed: {message}"),
            DexError::MalformedMethodPrototype(proto) => {
                write!(f, "malformed method prototype {proto}")
            }
            DexError::MissingDexForClass(class) => {
                write!(f, "loaded class {class} has no owning DEX")
            }
            DexError::InvalidMemberReference { reference, source } => {
                write!(f, "invalid member reference {reference}: {source}")
            }
            DexError::InvalidInvokeRegisters {
                offset,
                expected,
                actual,
            } => write!(
                f,
                "invoke at {offset} uses {actual} register words, expected {expected}"
            ),
            DexError::OverrideAnalysis(source) => {
                write!(f, "method override analysis failed: {source}")
            }
            DexError::InvalidDescriptor { descriptor, source } => {
                write!(f, "Invalid descriptor {descriptor}: {source}")
            }
            DexError::InvalidClassIndex(i) => write!(f, "Invalid class index: {}", i),
            DexError::InvalidMethodIndex(i) => write!(f, "Invalid method index: {}", i),
            DexError::InvalidFieldIndex(i) => write!(f, "Invalid field index: {}", i),
            DexError::InvalidTypeIndex(i) => write!(f, "Invalid type index: {}", i),
            DexError::InvalidStringIndex(i) => write!(f, "Invalid string index: {}", i),
        }
    }
}

impl std::error::Error for DexError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::IoError(source) => Some(source),
            Self::ClassInfo(source) => Some(source),
            Self::InvalidMemberReference { source, .. } => Some(source),
            Self::OverrideAnalysis(source) => Some(source),
            Self::InvalidDescriptor { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl From<std::io::Error> for DexError {
    fn from(e: std::io::Error) -> Self {
        DexError::IoError(e)
    }
}

impl From<crate::analysis::method_override::OverrideAnalysisError> for DexError {
    fn from(source: crate::analysis::method_override::OverrideAnalysisError) -> Self {
        Self::OverrideAnalysis(source)
    }
}

/// Result type for DEX operations
pub type DexResult<T> = Result<T, DexError>;
