//! Symbol sources for IR reference resolution.
//!
//! The IR decoder leaves pool indices in each instruction. Resolution turns
//! those indices into method, field, string and type identities. A whole-DEX
//! reader is one source; a caller that already decoded the method can supply the
//! same identities directly, so the recovery pipeline does not have to re-parse
//! the image for a local request.

use crate::frontend::DexFileReader;
use std::collections::HashMap;

/// Resolved pool entries the IR needs while its instructions are still indexed.
pub trait SymbolSource {
    fn method(&self, index: u32) -> Option<&str>;
    fn field(&self, index: u32) -> Option<&str>;
    fn ty(&self, index: u32) -> Option<&str>;
    fn string(&self, index: u32) -> Option<&[u16]>;
}

/// The whole-image reader bound to one DEX image.
pub struct ReaderSymbols<'a> {
    pub reader: &'a DexFileReader,
    pub dex: usize,
}

impl SymbolSource for ReaderSymbols<'_> {
    fn method(&self, index: u32) -> Option<&str> {
        self.reader.get_method(self.dex, index)
    }
    fn field(&self, index: u32) -> Option<&str> {
        self.reader.get_field(self.dex, index)
    }
    fn ty(&self, index: u32) -> Option<&str> {
        self.reader.get_type(self.dex, index)
    }
    fn string(&self, index: u32) -> Option<&[u16]> {
        self.reader.get_string(self.dex, index).map(|s| s.utf16())
    }
}

/// Identities a caller already resolved for one method's pool indices.
#[derive(Default)]
pub struct LocalSymbols {
    pub methods: HashMap<u32, String>,
    pub fields: HashMap<u32, String>,
    pub types: HashMap<u32, String>,
    pub strings: HashMap<u32, Vec<u16>>,
}

impl SymbolSource for LocalSymbols {
    fn method(&self, index: u32) -> Option<&str> {
        self.methods.get(&index).map(String::as_str)
    }
    fn field(&self, index: u32) -> Option<&str> {
        self.fields.get(&index).map(String::as_str)
    }
    fn ty(&self, index: u32) -> Option<&str> {
        self.types.get(&index).map(String::as_str)
    }
    fn string(&self, index: u32) -> Option<&[u16]> {
        self.strings.get(&index).map(Vec::as_slice)
    }
}
