//! Versioned platform ABI metadata.
//!
//! Platform symbols are immutable facts obtained from class files. They
//! constrain type and source recovery but never infer control-flow meaning.

#![allow(dead_code)]
use std::io;
use std::path::Path;
use std::sync::{Arc, OnceLock};

mod archive;
mod codec;
mod model;

pub use codec::DexSymbolsCodec;
pub use model::{
    PlatformAnnotation, PlatformAnnotationValue, PlatformClass, PlatformConstant,
    PlatformConstantDomain, PlatformConstantKind, PlatformConstantMember, PlatformFamily,
    PlatformField, PlatformFieldReference, PlatformMethod, PlatformNullability,
    PlatformSymbolDatabase, PlatformSymbolSet, PlatformTarget, SymbolAvailability,
    SymbolDatabaseStats, SymbolProvider, SymbolSource,
};

const EMBEDDED_SYMBOLS: &[u8] = include_bytes!("../../resources/symbols/platform.dexsym");

/// Returns the process-wide target-selected platform ABI.
///
/// The immutable database is decoded once. `DEXDEC_SYMBOLS` may point at a
/// compatible `.dexsym` file before the first request.
pub fn default_platform_symbols() -> io::Result<Arc<PlatformSymbolSet>> {
    static DEFAULT: OnceLock<Arc<PlatformSymbolSet>> = OnceLock::new();
    if let Some(symbols) = DEFAULT.get() {
        return Ok(Arc::clone(symbols));
    }
    let bytes = match std::env::var_os("DEXDEC_SYMBOLS") {
        Some(path) => std::fs::read(Path::new(&path))?,
        None => EMBEDDED_SYMBOLS.to_vec(),
    };
    let symbols = Arc::new(PlatformSymbolSet::open(&bytes, None)?);
    match DEFAULT.set(Arc::clone(&symbols)) {
        Ok(()) => Ok(symbols),
        Err(_) => DEFAULT.get().map(Arc::clone).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::Other,
                "platform symbol cache publication failed",
            )
        }),
    }
}
