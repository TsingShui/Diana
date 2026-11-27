use crate::frontend::{ClassNode, MethodNode};
use crate::ir::{AnalysisEvent, AnalysisObserver, CFG};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MethodRecoveryStage {
    Decode,
    JvmDecode,
    Metadata,
    Semantics,
    SourceLowering,
}

impl MethodRecoveryStage {
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Decode => "DEX decoding",
            Self::JvmDecode => "JVM decoding",
            Self::Metadata => "method metadata",
            Self::Semantics => "semantic recovery",
            Self::SourceLowering => "source lowering",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MethodRecoveryFailure {
    stage: MethodRecoveryStage,
    cause: String,
}

impl MethodRecoveryFailure {
    pub(crate) fn new(stage: MethodRecoveryStage, cause: impl std::fmt::Display) -> Self {
        Self {
            stage,
            cause: cause.to_string(),
        }
    }

    pub(crate) fn cause(&self) -> &str {
        &self.cause
    }

    pub(crate) fn summary(&self) -> String {
        format!(
            "Method could not be decompiled during {}: {}",
            self.stage.label(),
            self.cause
        )
    }

    pub(crate) fn observe(
        &self,
        observer: &dyn AnalysisObserver,
        class: &str,
        method: &str,
        descriptor: &str,
    ) {
        observer.observe(AnalysisEvent::MethodFailure {
            class,
            method,
            descriptor,
            stage: self.stage.label(),
            reason: &self.cause,
        });
    }
}

#[derive(Debug)]
pub(crate) enum ClassMethodState {
    Decoded(CFG),
    Failed(MethodRecoveryFailure),
}

#[derive(Debug)]
pub(crate) struct ClassMethodInput {
    position: usize,
    state: ClassMethodState,
}

impl ClassMethodInput {
    pub(crate) fn decoded(position: usize, cfg: CFG) -> Self {
        Self {
            position,
            state: ClassMethodState::Decoded(cfg),
        }
    }

    pub(crate) fn failed(position: usize, failure: MethodRecoveryFailure) -> Self {
        Self {
            position,
            state: ClassMethodState::Failed(failure),
        }
    }

    pub(crate) fn position(&self) -> usize {
        self.position
    }

    pub(crate) fn method<'a>(&self, class: &'a ClassNode) -> &'a MethodNode {
        // The input builder obtained this position from this class's method slice.
        &class.methods()[self.position]
    }

    pub(crate) fn cfg(&self) -> Option<&CFG> {
        match &self.state {
            ClassMethodState::Decoded(cfg) => Some(cfg),
            ClassMethodState::Failed(_) => None,
        }
    }

    pub(crate) fn into_cfg(self) -> Result<CFG, MethodRecoveryFailure> {
        match self.state {
            ClassMethodState::Decoded(cfg) => Ok(cfg),
            ClassMethodState::Failed(failure) => Err(failure),
        }
    }

    pub(crate) fn failure(&self) -> Option<&MethodRecoveryFailure> {
        match &self.state {
            ClassMethodState::Decoded(_) => None,
            ClassMethodState::Failed(failure) => Some(failure),
        }
    }
}
