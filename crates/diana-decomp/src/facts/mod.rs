//! Original DEX facts.
//!
//! The reader, P0 class facts, P1 decoded methods, shared substrate and
//! classified request errors live together here. Recovery stages consume these
//! products; they never re-open the input.

pub mod flow;
pub mod input;
pub mod p0;
pub mod p1;
pub mod pipeline;
pub mod substrate;
