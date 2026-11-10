//! Native DEX analysis with caller-owned input and classified failures.

mod engine;
pub(crate) use diana_facts::substrate;
pub use diana_facts::{input, p0, p1};
pub mod p2;
pub mod p3;
pub mod p4;
pub mod p5;
pub mod p6;
pub mod p7;
pub mod p8;
pub mod p9;
mod recovery;
pub use diana_facts::raw;
pub use diana_facts::{
    AnnotationExtent, AnnotationLocation, AnnotationParameterIndex, AnnotationPrefix,
    AnnotationRecord, AnnotationToken, AnnotationValue, AnnotationValueKind, OriginalAnnotations,
};
pub use diana_facts::{
    ByteLength, ByteOffset, ByteRange, CallSiteId, ClassDefId, ClassKey, CodeUnitCount,
    CodeUnitDelta, CodeUnitOffset, DexId, DiagnosticId, FieldId, InputId, MethodHandleId, MethodId,
    MethodKey, ProtoId, RegisterId, RegisterWordCount, SourceByteOffset, StringId, TypeId,
    Utf16Count, Utf16Offset,
};
pub use diana_facts::{
    Diagnostic, DiagnosticCause, DiagnosticCode, FailureKind, FieldStorage, ImageFact, ImageOrigin,
    RegisterSlot, RequestError, SlotWidth, StaticFieldOrdinal,
};
pub use engine::{
    DexImage, Engine, MethodOutcome, MethodSelector, Options, Presentation, RecoveryPolicy,
    RepresentationPolicy, Request, SingleMethodAnalysis,
};
