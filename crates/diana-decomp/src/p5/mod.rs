//! P5 storage bindings, transfer copies and exception snapshots.
mod bindings;
mod build;
mod coalesce;
mod copies;
mod materialization;
mod product;
mod transfers;

pub use build::StorageError;
pub(crate) use build::{StorageBuild, build_storage};
pub use product::*;
