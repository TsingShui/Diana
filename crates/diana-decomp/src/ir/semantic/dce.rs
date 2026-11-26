use super::{SemanticFoldError, SemanticFolder, SemanticNode, SemanticStatementKind};

/// Eliminates effect statements whose complete expression tree is proven
/// source-observable-effect free.
pub struct SemanticDeadCodeElimination {
    changed: bool,
}

impl SemanticDeadCodeElimination {
    pub fn apply(root: &mut SemanticNode) -> Result<bool, SemanticFoldError> {
        let mut elimination = Self { changed: false };
        super::visit::for_each_block_mut(root, |block| elimination.edit_block(block));
        Ok(elimination.changed)
    }

    fn edit_block(&mut self, block: &mut super::SemanticBlock) {
        block.statements.retain(|statement| {
            let removable = matches!(
                &statement.kind,
                SemanticStatementKind::Instruction(instruction)
                    if instruction.result.is_none()
            ) && statement.effects().is_pure();
            self.changed |= removable;
            !removable
        });
    }
}

impl SemanticFolder for SemanticDeadCodeElimination {
    type Error = SemanticFoldError;

    fn finish_node(&mut self, mut node: SemanticNode) -> Result<SemanticNode, Self::Error> {
        if let SemanticNode::BasicBlock(block) = &mut node {
            self.edit_block(block);
        }
        Ok(node)
    }
}
