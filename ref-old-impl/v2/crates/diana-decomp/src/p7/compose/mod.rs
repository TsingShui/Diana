//! Orders the display passes over the established emission layout.
//!
//! Every pass consumes the same closed P7 product.

mod access;
mod adjacency;
mod allocation;
mod arrays;
mod booleans;
mod branches;
mod casts;
mod concatenations;
mod continuations;
mod continues;
mod declarations;
mod expressions;
mod fallthrough;
mod labels;
mod literals;
mod loops;
mod pipeline;
mod references;
mod returns;
mod scope_labels;
mod scopes;
mod sequence;
mod stores;

use super::build::{bug, filled};
use super::*;

pub(super) use pipeline::construct;
