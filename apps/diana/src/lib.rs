//! Diana application library: input ownership and request scope.
//!
//! The library borrows a read-only mapping of the leased file; production code
//! uses no external process or runtime dependency, and the mapping is the only
//! `unsafe` operation in the crate.

pub mod analysis;
pub mod cli;
