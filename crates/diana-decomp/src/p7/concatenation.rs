//! Owned call-site presentation. The original Custom call remains authoritative.

use super::{ExprId, ReferenceId};
use serde::Serialize;

#[derive(Clone, Copy, Debug, Serialize)]
pub enum ConcatConversion {
    Value,
    Byte,
    Short,
    Int,
    Char,
}

#[derive(Debug, Serialize)]
pub enum ConcatPart {
    Argument {
        expression: ExprId,
        conversion: ConcatConversion,
    },
    /// A slice of an already-owned recipe or bootstrap String reference.
    Text {
        reference: ReferenceId,
        units: std::ops::Range<usize>,
    },
    /// A standard primitive bootstrap constant's one-time String conversion.
    Constant { units: Vec<u16>, event: usize },
}

#[derive(Debug, Serialize)]
pub struct Concatenation {
    pub(in crate::p7) parts: Vec<ConcatPart>,
    /// String conversion notation, not an additional original call argument.
    pub(in crate::p7) empty_prefix: bool,
}
impl Concatenation {
    pub fn parts(&self) -> &[ConcatPart] {
        &self.parts
    }
    pub fn empty_prefix(&self) -> bool {
        self.empty_prefix
    }
    /// A resolved String can be composed only before the first printed join.
    pub(in crate::p7) fn permits_string_composition(&self, expression: ExprId) -> bool {
        self.parts.iter().enumerate().any(|(position, part)| {
            position + usize::from(self.empty_prefix) < 2
                && matches!(part, ConcatPart::Argument { expression: id, .. } if *id == expression)
        })
    }
}

#[derive(Clone, Copy, Debug, Serialize)]
pub enum ConcatUnavailable {
    BootstrapSignature,
    CallSignature,
    ParameterType,
    ArgumentLayout,
    ArgumentView,
    BootstrapConstant,
    RecipeArity,
    ResourceExhausted,
}
impl ConcatUnavailable {
    pub fn message(self) -> &'static str {
        match self {
            Self::BootstrapSignature => "bootstrap signature or handle unavailable",
            Self::CallSignature => "call-site return or invocation signature unavailable",
            Self::ParameterType => {
                "parameter conversion outside the supported String/primitive views"
            }
            Self::ArgumentLayout => "original argument layout unavailable",
            Self::ArgumentView => "original argument view unavailable",
            Self::BootstrapConstant => "recipe or constant has no supported flat String conversion",
            Self::RecipeArity => "recipe argument or constant count mismatch",
            Self::ResourceExhausted => "concat presentation allocation failed",
        }
    }
    pub fn kind(self) -> crate::FailureKind {
        if matches!(self, Self::ResourceExhausted) {
            crate::FailureKind::ResourceExhausted
        } else {
            crate::FailureKind::Unsupported
        }
    }
}

#[derive(Debug, Serialize)]
pub enum ConcatOutcome {
    /// Assumes the named standard bootstrap contract, without runtime linkage proof.
    Inferred(Concatenation),
    /// Only this call's optional presentation is unavailable; its original stays.
    Explicit(ConcatUnavailable),
}
