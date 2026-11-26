//! Versioned platform ABI metadata.
//!
//! Platform symbols are immutable facts obtained from class files. They
//! constrain type and source recovery but never infer control-flow meaning.

#![allow(dead_code)]
use std::io;
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};

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

/// A failed platform load remains retryable. Only the first successful
/// producer builds the shared value, including when DEX images open in parallel.
pub(crate) struct PlatformCache<T> {
    value: OnceLock<Arc<T>>,
    loading: Mutex<()>,
}

impl<T> PlatformCache<T> {
    pub(crate) const fn new() -> Self {
        Self {
            value: OnceLock::new(),
            loading: Mutex::new(()),
        }
    }

    pub(crate) fn get_or_load(&self, load: impl FnOnce() -> io::Result<T>) -> io::Result<Arc<T>> {
        if let Some(value) = self.value.get() {
            return Ok(Arc::clone(value));
        }
        let _loading = self
            .loading
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if let Some(value) = self.value.get() {
            return Ok(Arc::clone(value));
        }
        let loaded = Arc::new(load()?);
        Ok(Arc::clone(self.value.get_or_init(|| loaded)))
    }
}

/// Returns the process-wide target-selected platform ABI.
///
/// The immutable database is decoded once. `DEXDEC_SYMBOLS` may point at a
/// compatible `.dexsym` file before the first request.
pub fn default_platform_symbols() -> io::Result<Arc<PlatformSymbolSet>> {
    static DEFAULT: PlatformCache<PlatformSymbolSet> = PlatformCache::new();
    DEFAULT.get_or_load(|| match std::env::var_os("DEXDEC_SYMBOLS") {
        Some(path) => PlatformSymbolSet::open_owned(std::fs::read(Path::new(&path))?, None),
        None => PlatformSymbolSet::open_static(EMBEDDED_SYMBOLS, None),
    })
}

#[cfg(test)]
#[path = "../../tests/internal/platform_cache.rs"]
mod cache_tests;
