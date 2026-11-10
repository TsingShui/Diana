//! P1 disassembly, structural checks and original instruction associations.

pub mod binding;
mod decode;
mod disassembly;
mod ir;
mod reference;
mod structure;

pub use disassembly::{
    ArrayPayload, DecodedInstruction, DecodedItem, DecodedMethod, DecodedOutcome, DecodedPayload,
    DecodedReference, EntryLayout, EntryParameter, InvokeLayout, MethodIr,
};
pub use reference::{
    BootstrapDeclaration, BootstrapReferenceId, BootstrapToken, BootstrapValue,
    CheckedBootstrapFact, CheckedMemberFact, CheckedMethodHandleFact, CheckedParameter,
    CheckedPrototype, CheckedReferenceFact, TypeForm,
};
pub use structure::{CheckedMethodLayout, MethodError, MethodProblem};
