use crate::ir::{InsnType, RegisterArg, SemanticBindingKind, SemanticNode, SemanticVisitor};

use super::JavaLoweringError;

#[derive(Debug, Clone, Default)]
pub(crate) struct JavaInputBindings {
    bindings: Vec<(SemanticBindingKind, RegisterArg)>,
    invalid: Option<Box<JavaLoweringError>>,
}

impl JavaInputBindings {
    pub(super) fn collect(root: &SemanticNode) -> Self {
        let mut input = Self::default();
        input.visit_node(root);
        input
    }

    pub(super) fn finish(
        self,
    ) -> Result<Vec<(SemanticBindingKind, RegisterArg)>, JavaLoweringError> {
        match self.invalid {
            Some(error) => Err(*error),
            None => Ok(self.bindings),
        }
    }

    fn reject(&mut self, error: JavaLoweringError) {
        if self.invalid.is_none() {
            self.invalid = Some(Box::new(error));
        }
    }
}

impl SemanticVisitor for JavaInputBindings {
    fn visit_binding(&mut self, kind: SemanticBindingKind, register: &RegisterArg) {
        self.bindings.push((kind, register.clone()));
    }

    fn enter_operation(&mut self, instruction: &crate::ir::SemanticOperation) {
        let error = match instruction.insn_type {
            InsnType::Phi => JavaLoweringError::UnrecoveredPhi(instruction.offset),
            InsnType::MoveResult => JavaLoweringError::UnrecoveredMoveResult(instruction.offset),
            InsnType::MoveException => {
                JavaLoweringError::UnrecoveredExceptionValue(instruction.offset)
            }
            InsnType::Nop => JavaLoweringError::UnsupportedStatement(InsnType::Nop),
            _ => return,
        };
        self.reject(error);
    }
}

#[cfg(test)]
#[path = "../../../../tests/internal/java_input_bindings.rs"]
mod input_tests;
