use crate::ir::{InsnType, RegisterArg, SemanticBindingKind, SemanticNode, SemanticVisitor};

use super::JavaLoweringError;

pub(super) struct JavaInputBindings {
    bindings: Vec<(SemanticBindingKind, RegisterArg)>,
    invalid: Option<JavaLoweringError>,
}

impl JavaInputBindings {
    pub(super) fn collect(
        root: &SemanticNode,
    ) -> Result<Vec<(SemanticBindingKind, RegisterArg)>, JavaLoweringError> {
        let mut input = Self {
            bindings: Vec::new(),
            invalid: None,
        };
        input.visit_node(root);
        input.invalid.map_or(Ok(input.bindings), Err)
    }

    fn reject(&mut self, error: JavaLoweringError) {
        if self.invalid.is_none() {
            self.invalid = Some(error);
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
