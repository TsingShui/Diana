//! Renders method bodies from the final emission plan through one event stack.

mod conditional;
mod control;
mod decision;
mod header;
mod layout;
mod loops;
mod protection;
mod statement;
mod statement_origins;
mod suspension;
mod terminal;
mod transfer;
mod traversal;
mod work;

use work::{Event, push};
