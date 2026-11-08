//! Input, archive and root-selection analysis owned by the application.
//!
//! `input` maps the leased file, `archive` reads ZIP metadata and entry
//! payloads from those bytes, `apk` selects the root DEX images, `dex` reads
//! original table facts, `query` streams rows in completion order, `xref` scans original reference sites, and `request`
//! holds one request's lease for its full lifetime.

mod android_strings;
pub mod apk;
pub mod archive;
pub mod at;
mod attr_names;
pub mod axml;
pub mod decompile;
pub mod dex;
pub mod extract;
pub mod filter;
pub mod images;
pub mod input;
pub mod manifest;
pub mod outline;
pub mod query;
pub mod request;
pub mod resources;
mod runner;
pub mod selector;
pub mod tree;
pub mod xref;

/// Worker count for independent DEX images on this host.
///
/// Image header reads, declaration scans and class-name resolution per image are
/// independent and must not serialize behind one large root. This is the host's
/// available parallelism; a request's method-worker setting stays separate.
pub(crate) fn image_jobs() -> usize {
    std::thread::available_parallelism()
        .map(|count| count.get())
        .unwrap_or(1)
}
