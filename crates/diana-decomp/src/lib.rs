//! Dex Decompiler Core Library
//!
//! This library recovers source text from original class, method and CFG facts.
//!
//! ## Architecture
//!
//! ```text
//! DEX -> normalized SSA CFG -> RegionGraph -> Semantic IR
//!     -> sparse value and type recovery -> typed source AST -> language printer
//! ```
//!
//! Control-flow meaning is recovered once in Region IR. Value recovery and
//! language backends consume semantic facts and never infer meaning from
//! rendered source shapes.
//!
//! ## Modules
//!
//! - `ir` - Intermediate Representation (types, args, instructions, blocks, CFG, passes, exception)
//! - `decoder` - Dalvik bytecode decoder
//! - `frontend` - class, method and field node structures
//! - `analysis` - Value, type, hierarchy, and language-backend analyses
//! - `engine` - The single public decompilation entry.
//! - `language` - Typed target-language ASTs and printers

pub mod analysis;
mod engine;
pub mod frontend;
pub mod ir;
pub mod language;
pub mod platform_symbols;
pub mod profiling;
pub mod render;

pub mod facts;

pub use facts::flow::FlowError;
pub use facts::{input, p0, p1, pipeline, substrate};

pub mod raw {
    pub use crate::input::formats::{
        instruction_layout, DexVersion, Format, IndexedRef, InsnLayout, LayoutKind, LayoutProblem,
        OpcodeSpec, OperandReference, RawDecodeError, ReferenceKind, ReferenceSlot,
    };
    pub use crate::input::operands::{
        ArrayElements, RawArrayElement, RawCodeItem, RawInstruction, RawLiteral, RawOperands,
        RawPayload, RawRegisters, RawSwitchCase, RegisterIter, SwitchCases,
    };
}

pub use input::annotation_facts::{
    AnnotationExtent, AnnotationLocation, AnnotationParameterIndex, AnnotationPrefix,
    AnnotationRecord, AnnotationToken, AnnotationValue, AnnotationValueKind, OriginalAnnotations,
};
pub use pipeline::diagnostics::{DiagnosticCode, FailureKind};
pub use pipeline::request::RequestError;
pub use substrate::fields::{FieldStorage, StaticFieldOrdinal};
pub use substrate::ids::{
    ByteLength, ByteOffset, ByteRange, CallSiteId, ClassDefId, ClassKey, CodeUnitCount,
    CodeUnitDelta, CodeUnitOffset, DexId, DiagnosticId, FieldId, InputId, MethodHandleId, MethodId,
    MethodKey, ProtoId, RegisterId, RegisterWordCount, SourceByteOffset, StringId, TypeId,
    Utf16Count, Utf16Offset,
};
pub use substrate::report::{Diagnostic, DiagnosticCause, ImageFact, ImageOrigin};
pub use substrate::slots::{RegisterSlot, SlotWidth};

// Engine facade over original facts and the recovery backend.
pub use engine::{
    ClassAnalysis, ClassSource, DexImage, Engine, MethodSelector, MethodSource, Request,
    SingleMethodAnalysis,
};

pub use frontend::{ClassNode, DexError, DexResult, MethodCode, MethodNode};
pub use platform_symbols::{
    default_platform_symbols, PlatformAnnotation, PlatformAnnotationValue, PlatformClass,
    PlatformConstant, PlatformConstantDomain, PlatformConstantKind, PlatformConstantMember,
    PlatformFamily, PlatformField, PlatformFieldReference, PlatformMethod, PlatformSymbolDatabase,
    PlatformSymbolSet, PlatformTarget, SymbolAvailability, SymbolDatabaseStats, SymbolProvider,
    SymbolSource,
};

// IR types
pub use ir::{ArgType, InsnArg, InsnNode, InsnType, RegisterArg};
pub use ir::{Block, BlockId, EdgeKind, ExceptionHandler, CFG};
pub use ir::{Pass, PassResult, Splitter};

// Exception analysis
pub use ir::{CatchHandler, ExceptionAnalysis, TryRegion};
