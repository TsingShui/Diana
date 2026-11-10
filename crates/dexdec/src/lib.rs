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
//! - `api` - High-level decompilation API
//! - `analysis` - Value, type, hierarchy, and language-backend analyses
//! - `language` - Typed target-language ASTs and printers

pub mod analysis;
pub mod api;
pub mod decoder;
pub mod frontend;
pub mod ir;
pub mod language;
pub mod local;
pub mod platform_symbols;
pub mod profiling;

// Recovery entry: original facts in, source text out.
pub use api::{
    decompile_class, decompile_method, ClassInput, ClassOutput, MethodInput, MethodOptions,
    MethodOutcome,
};
pub use frontend::{ClassNode, DexError, DexResult, MethodCode, MethodNode};
pub use local::LocalSymbols;
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

// Analysis and code generation
pub use analysis::{JavaDecompiler, JavaDecompilerConfig, JavaDecompilerError};
#[cfg(feature = "kotlin")]
pub use analysis::{KotlinDecompiler, KotlinDecompilerConfig, KotlinDecompilerError};
