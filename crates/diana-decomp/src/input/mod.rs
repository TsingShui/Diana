//! Checked DEX byte reads and header envelopes. Header acceptance does not
//! validate variable items, instruction semantics or Java representability.

pub mod annotation_facts;
pub mod annotations;
pub mod catalog;
mod class_locations;
mod code;
mod container;
mod cursor;
mod declarations;
mod dynamic;
mod error;
pub mod formats;
mod header;
mod method;
mod reference_leaves;
mod retained;
pub use retained::RetainedCode;
pub mod operands;
pub mod prefix;
pub mod protection;
pub mod strings;
pub mod value_facts;
pub mod values;

pub use crate::p1::{
    CheckedMethodLayout, DecodedInstruction, DecodedItem, DecodedMethod, DecodedPayload,
    DecodedReference, MethodError, MethodProblem,
};
pub use catalog::{
    ClassHierarchy, ClassView, FieldDeclaration, FieldEntry, FieldStorage, MethodDeclaration,
    MethodEntry, ProgramIndex, Prototype, ReferenceFacts,
};
pub use code::{CatchTypeFact, MethodBody, RawMethodView};
pub use container::DexContainer;
pub use cursor::{checked_range, Cursor};
pub use declarations::{LocalDeclarations, LocalParent};
use error::{count_usize, file_length, file_offset, malformed};
pub use error::{InputError, InputProblem, UnsupportedInput};
pub use header::{read_header, DexHeader, TableSection};
pub use method::{CheckedAnnotations, MethodSource, RawPrintable};
pub use prefix::{DeclaredClass, PrefixClasses};
pub use strings::DexString;
