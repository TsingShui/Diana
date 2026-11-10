//! Original DEX facts shared by the recovery stages and the decompiler core.
//!
//! This crate owns the reader product, P0 class facts and the P1 decoded method.
//! It states what the input contains; it performs no recovery.

mod flow;
pub mod input;
pub mod p0;
pub mod p1;
pub mod pipeline;
pub mod substrate;

pub use flow::FlowError;
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

pub mod raw {
    pub use crate::input::formats::{
        DexVersion, Format, IndexedRef, InsnLayout, LayoutKind, LayoutProblem, OpcodeSpec,
        OperandReference, RawDecodeError, ReferenceKind, ReferenceSlot, instruction_layout,
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
