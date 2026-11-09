//! Prints delivered expressions and statement contents through one event stack.
//! Complete and partial products share rendering without reconstructing facts.

mod arrays;
mod calls;
mod composition;
mod concatenations;
mod context;
mod copies;
mod literals;
mod read;
mod spelling;
mod statements;
mod traversal;
mod work;

pub(super) use context::{ExpressionSource, Expressions};
pub(super) use literals::machine_type;
