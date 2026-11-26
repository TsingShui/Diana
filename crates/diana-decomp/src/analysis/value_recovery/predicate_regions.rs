//! Structured region formation from predicated semantic fragments.

use crate::ir::{
    analysis::SsaVar, semantic::SemanticCompletion, BlockId, BoolExpr, BoolVariable, InstructionId,
    SemanticBlock, SemanticExpressionFacts, SemanticFoldError, SemanticFolder, SemanticNode,
    SemanticPredicate, SemanticStatement, SemanticVisitor,
};

use super::ValueRecoveryError;

mod distribution;
mod guarded_assignment;

use distribution::GuardDistribution;
use guarded_assignment::GuardedAssignment;

pub(super) struct PredicateRegionFormation {
    changed: bool,
}

impl PredicateRegionFormation {
    pub(super) fn apply(root: &mut SemanticNode) -> Result<bool, ValueRecoveryError> {
        if crate::ir::trivial_early_returns() && !needs_predicate_regions(root) {
            return Ok(false);
        }
        let before = SemanticCompletion::analyze(root);
        let mut changed = false;
        loop {
            let mut formation = Self { changed: false };
            let body = std::mem::replace(root, SemanticNode::Empty);
            *root = formation.fold_node(body)?;
            if !formation.changed {
                break;
            }
            changed = true;
        }
        if changed {
            // Only a rewrite can change completion; verifying an untouched
            // tree would compare it against itself.
            let after = SemanticCompletion::analyze(root);
            if before != after {
                return Err(SemanticFoldError::CompletionChanged {
                    transform: "predicate-region-formation",
                }
                .into());
            }
        }
        Ok(changed)
    }

    fn form_sequence(
        &mut self,
        nodes: Vec<SemanticNode>,
    ) -> Result<SemanticNode, ValueRecoveryError> {
        if !has_guarded_tail(&nodes) {
            return Ok(SemanticNode::sequence(nodes));
        }
        let mut formed = Vec::<SemanticNode>::with_capacity(nodes.len());
        for mut node in nodes {
            if let (
                Some(SemanticNode::If {
                    condition,
                    then_node,
                    else_node: None,
                }),
                SemanticNode::If {
                    condition: next_condition,
                    then_node: next_then,
                    else_node: None,
                },
            ) = (formed.last_mut(), &mut node)
            {
                if Self::can_extend(condition, then_node, next_condition)? {
                    let first = SemanticCompletion::analyze(then_node);
                    let second = SemanticCompletion::analyze(next_then);
                    let original = SemanticCompletion::sequence([
                        SemanticCompletion::branch(condition, first.clone(), None),
                        SemanticCompletion::branch(next_condition, second.clone(), None),
                    ]);
                    let merged = SemanticCompletion::branch(
                        condition,
                        SemanticCompletion::sequence([first, second]),
                        None,
                    );
                    if original == merged {
                        let first = std::mem::replace(then_node.as_mut(), SemanticNode::Empty);
                        let second = std::mem::replace(next_then.as_mut(), SemanticNode::Empty);
                        **then_node = SemanticNode::sequence([first, second]);
                        self.changed = true;
                        continue;
                    }
                }
            }

            if let (
                Some(last),
                SemanticNode::If {
                    condition,
                    then_node,
                    else_node: None,
                },
            ) = (formed.last_mut(), &node)
            {
                if let Some(distributed) = GuardDistribution::apply(last, condition, then_node)? {
                    *last = distributed;
                    self.changed = true;
                    continue;
                }
            }
            formed.push(node);
        }
        Ok(SemanticNode::sequence(formed))
    }

    /// Splits a block around every definition whose selection is guarded.
    ///
    /// The recovered regions are emitted into the enclosing sequence, where
    /// region formation can merge them with the neighbours that share their
    /// predicate instead of leaving each merge to restate it.
    fn form_block(&mut self, block: SemanticBlock) -> SemanticNode {
        if block.statements.is_empty() {
            return SemanticNode::Empty;
        }
        if !block
            .statements
            .iter()
            .any(GuardedAssignment::is_self_selecting)
        {
            return SemanticNode::BasicBlock(block);
        }
        let id = block.id;
        let mut formed = Vec::<SemanticNode>::new();
        let mut pending = Vec::<SemanticStatement>::new();
        for statement in block.statements {
            match GuardedAssignment::recover(statement, id) {
                Ok(region) => {
                    formed.push(Self::fragment(id, std::mem::take(&mut pending)));
                    formed.push(region);
                    self.changed = true;
                }
                Err(statement) => pending.push(statement),
            }
        }
        formed.push(Self::fragment(id, pending));
        SemanticNode::sequence(formed)
    }

    fn fragment(id: BlockId, statements: Vec<SemanticStatement>) -> SemanticNode {
        if statements.is_empty() {
            return SemanticNode::Empty;
        }
        SemanticNode::BasicBlock(SemanticBlock { id, statements })
    }

    fn can_extend(
        condition: &SemanticPredicate,
        body: &SemanticNode,
        next_condition: &SemanticPredicate,
    ) -> Result<bool, ValueRecoveryError> {
        if !Self::predicate_is_pure(condition) || !Self::equivalent(condition, next_condition)? {
            return Ok(false);
        }
        let predicate = SemanticExpressionFacts::of_predicate(condition);
        let body = SemanticExpressionFacts::of_node(body);
        Ok(predicate
            .used_variables()
            .all(|variable| body.definition_count(variable) == 0)
            && predicate
                .used_ssa_variables()
                .all(|variable| body.ssa_definition_count(variable) == 0))
    }

    fn equivalent(
        left: &SemanticPredicate,
        right: &SemanticPredicate,
    ) -> Result<bool, ValueRecoveryError> {
        match PredicateEquivalence::new(left, right).equivalent(left, right)? {
            Ok(equivalent) => Ok(equivalent),
            Err(error) if error.is_resource_limit() => Ok(false),
            Err(error) => Err(SemanticFoldError::from(error).into()),
        }
    }

    fn predicate_is_pure(predicate: &SemanticPredicate) -> bool {
        let mut pending = vec![predicate];
        while let Some(predicate) = pending.pop() {
            match predicate {
                SemanticPredicate::Test(instruction) => {
                    if !instruction.effects().without_control().is_pure() {
                        return false;
                    }
                }
                SemanticPredicate::Not(inner) => pending.push(inner),
                SemanticPredicate::And(terms) | SemanticPredicate::Or(terms) => {
                    pending.extend(terms);
                }
                SemanticPredicate::True | SemanticPredicate::False => {}
            }
        }
        true
    }
}

fn has_guarded_tail(nodes: &[SemanticNode]) -> bool {
    nodes.iter().skip(1).any(|node| {
        matches!(
            node,
            SemanticNode::If {
                else_node: None,
                ..
            }
        )
    })
}

fn needs_predicate_regions(root: &SemanticNode) -> bool {
    #[derive(Default)]
    struct Sites {
        needs_fold: bool,
        has_if: bool,
        needs_normalization: bool,
    }
    impl SemanticVisitor for Sites {
        fn visits_values(&self) -> bool {
            false
        }

        fn enter_node(&mut self, node: &SemanticNode) {
            if self.needs_fold {
                return;
            }
            match node {
                SemanticNode::BasicBlock(block) => {
                    self.needs_normalization |= block.statements.is_empty();
                    self.needs_fold = block
                        .statements
                        .iter()
                        .any(GuardedAssignment::is_self_selecting);
                }
                SemanticNode::Sequence(children) => {
                    self.needs_fold = has_guarded_tail(children);
                    self.needs_normalization |= children.len() < 2
                        || children.iter().any(|child| {
                            matches!(child, SemanticNode::Empty | SemanticNode::Sequence(_))
                        });
                }
                SemanticNode::If { .. } => self.has_if = true,
                // The original pass also enters on a self-selecting loop
                // statement, then canonicalizes any blocks and sequences.
                SemanticNode::For { init, update, .. } => {
                    self.needs_fold = GuardedAssignment::is_self_selecting(init)
                        || GuardedAssignment::is_self_selecting(update);
                }
                _ => {}
            }
            // An isolated branch has no region pairing, but still activates
            // the existing normalization of empty blocks and nested sequences.
            self.needs_fold |= self.has_if && self.needs_normalization;
        }
    }
    let mut sites = Sites::default();
    sites.visit_node(root);
    sites.needs_fold
}

struct PredicateEquivalence {
    symbols: std::collections::BTreeMap<InstructionId, BoolVariable>,
}

impl PredicateEquivalence {
    fn new(left: &SemanticPredicate, right: &SemanticPredicate) -> Self {
        let mut atoms = Vec::<&crate::ir::SemanticOperation>::new();
        let mut symbols = std::collections::BTreeMap::new();
        for predicate in [left, right] {
            let mut pending = vec![predicate];
            while let Some(predicate) = pending.pop() {
                match predicate {
                    SemanticPredicate::Test(instruction) => {
                        let identity = atoms
                            .iter()
                            .position(|atom| Self::instruction(atom, instruction))
                            .unwrap_or_else(|| {
                                atoms.push(instruction);
                                atoms.len() - 1
                            });
                        symbols.insert(
                            instruction.id,
                            BoolVariable::Named(format!("predicate-value:{identity}")),
                        );
                    }
                    SemanticPredicate::Not(inner) => pending.push(inner),
                    SemanticPredicate::And(terms) | SemanticPredicate::Or(terms) => {
                        pending.extend(terms);
                    }
                    SemanticPredicate::True | SemanticPredicate::False => {}
                }
            }
        }
        Self { symbols }
    }

    fn equivalent(
        &self,
        left: &SemanticPredicate,
        right: &SemanticPredicate,
    ) -> Result<Result<bool, crate::ir::bdd::BddError>, SemanticFoldError> {
        let left = self.expression(left.domain()?)?;
        let right = self.expression(right.domain()?)?;
        Ok(left.equivalent(&right))
    }

    fn expression(&self, expression: BoolExpr) -> Result<BoolExpr, SemanticFoldError> {
        let mut pending = vec![PredicateRemapTask::Visit(expression)];
        let mut results = Vec::new();
        while let Some(task) = pending.pop() {
            match task {
                PredicateRemapTask::Visit(expression) => match expression {
                    BoolExpr::Symbol(BoolVariable::Instruction(instruction)) => {
                        results.push(BoolExpr::Symbol(
                            self.symbols
                                .get(&instruction)
                                .cloned()
                                .ok_or(SemanticFoldError::MalformedWorkStack)?,
                        ));
                    }
                    BoolExpr::Not(inner) => {
                        pending.push(PredicateRemapTask::Not);
                        pending.push(PredicateRemapTask::Visit(*inner));
                    }
                    BoolExpr::And(terms) => {
                        pending.push(PredicateRemapTask::Junction {
                            count: terms.len(),
                            conjunction: true,
                        });
                        pending.extend(terms.into_iter().rev().map(PredicateRemapTask::Visit));
                    }
                    BoolExpr::Or(terms) => {
                        pending.push(PredicateRemapTask::Junction {
                            count: terms.len(),
                            conjunction: false,
                        });
                        pending.extend(terms.into_iter().rev().map(PredicateRemapTask::Visit));
                    }
                    expression => results.push(expression),
                },
                PredicateRemapTask::Not => {
                    let inner = results.pop().ok_or(SemanticFoldError::MalformedWorkStack)?;
                    results.push(BoolExpr::not(inner));
                }
                PredicateRemapTask::Junction { count, conjunction } => {
                    let start = results
                        .len()
                        .checked_sub(count)
                        .ok_or(SemanticFoldError::MalformedWorkStack)?;
                    let terms = results.drain(start..).collect();
                    results.push(if conjunction {
                        BoolExpr::and(terms)
                    } else {
                        BoolExpr::or(terms)
                    });
                }
            }
        }
        if results.len() != 1 {
            return Err(SemanticFoldError::MalformedWorkStack);
        }
        results.pop().ok_or(SemanticFoldError::MalformedWorkStack)
    }

    fn instruction(
        left: &crate::ir::SemanticOperation,
        right: &crate::ir::SemanticOperation,
    ) -> bool {
        if !left.operation_equivalent(right)
            || left.operands().len() != right.operands().len()
            || left.compound_target().is_some() != right.compound_target().is_some()
        {
            return false;
        }
        let mut pending = left
            .operands()
            .iter()
            .zip(right.operands())
            .collect::<Vec<_>>();
        if let (Some(left), Some(right)) = (left.compound_target(), right.compound_target()) {
            pending.push((left, right));
        }
        while let Some((left, right)) = pending.pop() {
            match (left, right) {
                (
                    crate::ir::SemanticExpression::Register(left),
                    crate::ir::SemanticExpression::Register(right),
                ) => {
                    let equivalent = match (left.code_var, right.code_var) {
                        (Some(left), Some(right)) => left == right,
                        (None, None) => match (SsaVar::from_reg(left), SsaVar::from_reg(right)) {
                            (Some(left), Some(right)) => left == right,
                            (None, None) => left.reg_num == right.reg_num,
                            _ => false,
                        },
                        _ => false,
                    };
                    if !equivalent {
                        return false;
                    }
                }
                (
                    crate::ir::SemanticExpression::Literal(left),
                    crate::ir::SemanticExpression::Literal(right),
                ) if left == right => {}
                (
                    crate::ir::SemanticExpression::Operation(left),
                    crate::ir::SemanticExpression::Operation(right),
                ) => {
                    if !left.operation_equivalent(right)
                        || left.operands().len() != right.operands().len()
                    {
                        return false;
                    }
                    pending.extend(left.operands().iter().zip(right.operands()));
                }
                _ => return false,
            }
        }
        true
    }
}

enum PredicateRemapTask {
    Visit(BoolExpr),
    Not,
    Junction { count: usize, conjunction: bool },
}

impl SemanticFolder for PredicateRegionFormation {
    type Error = ValueRecoveryError;

    fn finish_node(&mut self, node: SemanticNode) -> Result<SemanticNode, Self::Error> {
        match node {
            SemanticNode::Sequence(nodes) => self.form_sequence(nodes),
            SemanticNode::BasicBlock(block) => Ok(self.form_block(block)),
            node => Ok(node),
        }
    }
}

#[cfg(test)]
#[path = "../../../tests/internal/predicate_regions.rs"]
mod tests;
