use std::collections::TryReserveError;

use thiserror::Error;

use crate::{CodeUnitOffset, FailureKind};

/// Classified P2 flow-construction failure. It lives beside the facts because
/// the request error carries it; the flow planner that produces it stays in the
/// recovery crate.
#[derive(Debug, Error)]
pub enum FlowError {
    #[error("flow allocation failed: {0}")]
    Allocation(#[source] TryReserveError),
    #[error("flow capacity is not representable on this platform")]
    PlatformCapacity,
    #[error("unsupported flow at {pc:?} opcode 0x{opcode:02x}: {reason}")]
    Unsupported {
        pc: CodeUnitOffset,
        opcode: u8,
        reason: &'static str,
    },
    #[error("P2 correspondence failed at {pc:?}: {obligation}")]
    Correspondence {
        pc: CodeUnitOffset,
        obligation: &'static str,
    },
    #[error("BUG: {0}")]
    InternalInvariant(&'static str),
}

impl FlowError {
    pub fn kind(&self) -> FailureKind {
        match self {
            Self::Allocation(_) | Self::PlatformCapacity => FailureKind::ResourceExhausted,
            Self::Unsupported { .. } => FailureKind::Unsupported,
            Self::InternalInvariant(_) => FailureKind::InternalInvariant,
            Self::Correspondence { .. } => FailureKind::InternalInvariant,
        }
    }
}
