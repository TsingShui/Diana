use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap};

use crate::ir::{
    analysis::SsaVar, IfOp, InsnType, RegisterArg, SemanticExpression, SemanticExpressionTransform,
    SemanticInstructions, SemanticNode, SemanticOperation, SemanticPredicate,
    SemanticStatementKind,
};

use super::flow::{UseSite, ValueAction, ValueIdentity};
use super::ValueRecoveryError;

pub(super) struct ValueSchedule {
    identity: ValueIdentity,
    replacements: BTreeMap<SsaVar, SemanticExpression>,
    site_replacements: BTreeMap<UseSite, BTreeMap<SsaVar, SiteReplacement>>,
    removals: BTreeSet<SsaVar>,
    statement_removals: BTreeSet<crate::ir::SemanticSiteId>,
    discarded_results: BTreeSet<SsaVar>,
    changed: bool,
}

struct SiteReplacement {
    value: SemanticExpression,
}

impl ValueSchedule {
    pub(super) fn contains_select(root: &SemanticNode) -> bool {
        struct Selects(bool);
        impl crate::ir::SemanticVisitor for Selects {
            fn visits_values(&self) -> bool {
                false
            }
            fn enter_node(&mut self, node: &SemanticNode) {
                self.0 = self.0 || SemanticExpressionReduction::node_contains_select(node);
            }
        }
        let mut selects = Selects(false);
        crate::ir::SemanticVisitor::visit_node(&mut selects, root);
        selects.0
    }

    pub(super) fn compile(
        actions: impl IntoIterator<Item = ValueAction>,
        identity: ValueIdentity,
    ) -> Result<Self, ValueRecoveryError> {
        let mut schedule = Self {
            identity,
            replacements: BTreeMap::new(),
            site_replacements: BTreeMap::new(),
            removals: BTreeSet::new(),
            statement_removals: BTreeSet::new(),
            discarded_results: BTreeSet::new(),
            changed: false,
        };
        for action in actions {
            match action {
                ValueAction::Propagate {
                    key, replacement, ..
                } => {
                    schedule
                        .replacements
                        .insert(key, SemanticExpression::from_argument(replacement)?);
                }
                ValueAction::Inline {
                    key, expression, ..
                } => {
                    schedule.replacements.insert(key, expression);
                }
                ValueAction::InlineAt {
                    key,
                    remove_key,
                    remove_definitions,
                    replacements,
                    ..
                } => {
                    if remove_key {
                        schedule.removals.insert(key);
                    }
                    schedule.statement_removals.extend(remove_definitions);
                    for (site, expression) in replacements {
                        schedule.site_replacements.entry(site).or_default().insert(
                            key,
                            SiteReplacement {
                                value: expression.value,
                            },
                        );
                    }
                }
                ValueAction::ReplaceAt {
                    key, replacements, ..
                } => {
                    for (site, value) in replacements {
                        schedule.site_replacements.entry(site).or_default().insert(
                            key,
                            SiteReplacement {
                                value: SemanticExpression::from_argument(value)?,
                            },
                        );
                    }
                }
                ValueAction::EliminateStatements { statements, .. } => {
                    schedule.statement_removals.extend(statements);
                }
                ValueAction::Remove { key, .. } => {
                    schedule.removals.insert(key);
                }
                ValueAction::DiscardResult { key, .. } => {
                    schedule.discarded_results.insert(key);
                }
            }
        }
        schedule.canonicalize()?;
        Ok(schedule)
    }

    pub(super) fn apply(mut self, root: &mut SemanticNode) -> Result<bool, ValueRecoveryError> {
        self.apply_tree(root)?;
        Ok(self.changed)
    }

    fn apply_tree(&mut self, node: &mut SemanticNode) -> Result<(), ValueRecoveryError> {
        match node {
            SemanticNode::Empty | SemanticNode::BasicBlock(_) | SemanticNode::Leave(_) => {}
            SemanticNode::Sequence(children) => {
                for child in children {
                    self.apply_tree(child)?;
                }
            }
            SemanticNode::If {
                then_node,
                else_node,
                ..
            } => {
                self.apply_tree(then_node.as_mut())?;
                if let Some(else_node) = else_node {
                    self.apply_tree(else_node.as_mut())?;
                }
            }
            SemanticNode::Loop { test, body, .. } => {
                self.apply_tree(test.setup.as_mut())?;
                self.apply_tree(body.as_mut())?;
            }
            SemanticNode::For { body, .. } => self.apply_tree(body.as_mut())?,
            SemanticNode::ForEach { body, .. } => self.apply_tree(body.as_mut())?,
            SemanticNode::Switch { cases, .. } => {
                for case in cases {
                    self.apply_tree(&mut case.body)?;
                }
            }
            SemanticNode::Try {
                body,
                catches,
                finally,
                ..
            } => {
                self.apply_tree(body.as_mut())?;
                for catch in catches {
                    self.apply_tree(&mut catch.body)?;
                }
                if let Some(finally) = finally {
                    self.apply_tree(finally.body.as_mut())?;
                }
            }
            SemanticNode::Synchronized { body, .. } | SemanticNode::Label { body, .. } => {
                self.apply_tree(body.as_mut())?;
            }
        }
        self.apply_node(node)
    }

    fn canonicalize(&mut self) -> Result<(), ValueRecoveryError> {
        let order = ReplacementGraph::new(&self.replacements, self.identity)?.into_order();
        let mut raw = std::mem::take(&mut self.replacements);
        let mut canonical = BTreeMap::new();
        for key in order {
            let value = raw
                .remove(&key)
                .ok_or(ValueRecoveryError::MissingReplacement(key))?;
            canonical.insert(
                key,
                ExpressionDependencies::substitute(value, &canonical, self.identity)?,
            );
        }
        for replacements in self.site_replacements.values_mut() {
            for replacement in replacements.values_mut() {
                replacement.value = ExpressionDependencies::substitute(
                    replacement.value.clone(),
                    &canonical,
                    self.identity,
                )?;
            }
        }
        let mut required = BTreeSet::new();
        for value in canonical.values().chain(
            self.site_replacements
                .values()
                .flat_map(|sites| sites.values().map(|replacement| &replacement.value)),
        ) {
            ExpressionDependencies::visit(value, self.identity, &mut |key| {
                if (self.removals.contains(&key) || self.discarded_results.contains(&key))
                    && !canonical.contains_key(&key)
                {
                    required.insert(key);
                }
            })?;
        }
        self.removals.retain(|value| !required.contains(value));
        self.discarded_results
            .retain(|value| !required.contains(value));
        self.changed |= !canonical.is_empty();
        self.changed |= !self.site_replacements.is_empty();
        self.removals.extend(canonical.keys().copied());
        self.replacements = canonical;
        Ok(())
    }

    fn apply_node(&mut self, node: &mut SemanticNode) -> Result<(), ValueRecoveryError> {
        if !self.replacements.is_empty()
            || !self.discarded_results.is_empty()
            || SemanticExpressionReduction::node_contains_select(node)
        {
            let mut substitution = ValueSubstitution {
                replacements: &self.replacements,
                discarded_results: &self.discarded_results,
                identity: self.identity,
                changed: false,
            };
            SemanticInstructions::transform_node(node, &mut substitution)?;
            self.changed |= substitution.changed;
        }
        match node {
            SemanticNode::BasicBlock(block) => {
                for statement in &mut block.statements {
                    self.apply_statement_site(statement)?;
                }
            }
            SemanticNode::For {
                init,
                condition,
                update,
                ..
            } => {
                self.apply_statement_site(init)?;
                self.apply_predicate_site(condition)?;
                self.apply_statement_site(update)?;
            }
            SemanticNode::If { condition, .. } => {
                self.apply_predicate_site(condition)?;
            }
            SemanticNode::Loop { test, .. } => {
                self.apply_predicate_site(&mut test.condition)?;
            }
            SemanticNode::ForEach { iterable, .. } => {
                self.apply_expression_site(iterable)?;
            }
            SemanticNode::Switch { selector, .. } => {
                self.apply_expression_site(selector)?;
            }
            SemanticNode::Synchronized { lock, .. } => {
                self.apply_expression_site(lock)?;
            }
            _ => {}
        }
        if let SemanticNode::Leave(leave) = node {
            if let Some(replacements) = leave
                .site
                .and_then(|site| self.site_replacements.get(&UseSite::Leave(site)))
            {
                if let Some(value) = leave.value_mut() {
                    let mut substitution = SiteSubstitution {
                        replacements,
                        identity: self.identity,
                        changed: false,
                    };
                    SemanticInstructions::transform_expression(value, &mut substitution)?;
                    self.changed |= substitution.changed;
                }
            }
        }
        if let SemanticNode::BasicBlock(block) = node {
            let before = block.statements.len();
            block.statements.retain(|statement| {
                if statement
                    .site
                    .is_some_and(|site| self.statement_removals.contains(&site))
                {
                    return false;
                }
                statement
                    .result()
                    .and_then(|register| self.identity.key(register))
                    .is_none_or(|key| !self.removals.contains(&key))
            });
            self.changed |= block.statements.len() != before;
        }
        Ok(())
    }

    fn apply_statement_site(
        &mut self,
        statement: &mut crate::ir::SemanticStatement,
    ) -> Result<(), ValueRecoveryError> {
        let Some(site) = statement.site else {
            return Ok(());
        };
        match &mut statement.kind {
            SemanticStatementKind::Instruction(instruction) => {
                let replacements = self.site_replacements.get(&UseSite::Statement(site));
                if replacements.is_none() && !instruction.contains_select() {
                    return Ok(());
                }
                let mut expression = SemanticExpression::Operation(instruction.clone());
                if let Some(replacements) = replacements {
                    let mut substitution = SiteSubstitution {
                        replacements,
                        identity: self.identity,
                        changed: false,
                    };
                    SemanticInstructions::transform_expression(&mut expression, &mut substitution)?;
                    self.changed |= substitution.changed;
                }
                let (expression, reduced) =
                    if SemanticExpressionReduction::contains_select(&expression) {
                        SemanticExpressionReduction::new(self.identity).reduce(expression)?
                    } else {
                        (expression, false)
                    };
                let SemanticExpression::Operation(operation) = expression else {
                    return Err(crate::ir::SemanticFoldError::NonOperationStatement.into());
                };
                *instruction = operation;
                self.changed |= reduced;
            }
            SemanticStatementKind::Definition { value, .. } => {
                if self.has_selected_value_replacements(site) {
                    self.apply_selected_value_sites(site, value)?;
                }
                if !SemanticExpressionReduction::contains_select(value) {
                    return Ok(());
                }
                let (reduced, changed) =
                    SemanticExpressionReduction::new(self.identity).reduce(value.clone())?;
                *value = reduced;
                self.changed |= changed;
            }
        }
        Ok(())
    }

    fn has_selected_value_replacements(&self, site: crate::ir::SemanticSiteId) -> bool {
        if self.site_replacements.is_empty() {
            return false;
        }
        self.site_replacements
            .range(UseSite::SelectedArgument(site, 0)..=UseSite::SelectedArgument(site, u32::MAX))
            .next()
            .is_some()
            || self
                .site_replacements
                .range(
                    UseSite::SelectedPredicate(site, 0)
                        ..=UseSite::SelectedPredicate(site, u32::MAX),
                )
                .next()
                .is_some()
    }

    fn apply_selected_value_sites(
        &mut self,
        site: crate::ir::SemanticSiteId,
        value: &mut SemanticExpression,
    ) -> Result<(), ValueRecoveryError> {
        let mut pending = vec![value];
        let mut argument_occurrence = 0u32;
        let mut predicate_occurrence = 0u32;
        while let Some(value) = pending.pop() {
            match value {
                SemanticExpression::Register(_)
                | SemanticExpression::Literal(_)
                | SemanticExpression::Operation(_) => {
                    let current = argument_occurrence;
                    argument_occurrence += 1;
                    let Some(replacements) = self
                        .site_replacements
                        .get(&UseSite::SelectedArgument(site, current))
                    else {
                        continue;
                    };
                    let mut substitution = SiteSubstitution {
                        replacements,
                        identity: self.identity,
                        changed: false,
                    };
                    SemanticInstructions::transform_expression(value, &mut substitution)?;
                    self.changed |= substitution.changed;
                }
                SemanticExpression::Select {
                    condition,
                    when_true,
                    when_false,
                } => {
                    let current = predicate_occurrence;
                    predicate_occurrence += 1;
                    if let Some(replacements) = self
                        .site_replacements
                        .get(&UseSite::SelectedPredicate(site, current))
                    {
                        let mut substitution = SiteSubstitution {
                            replacements,
                            identity: self.identity,
                            changed: false,
                        };
                        SemanticInstructions::transform_predicate(condition, &mut substitution)?;
                        self.changed |= substitution.changed;
                    }
                    pending.push(when_false);
                    pending.push(when_true);
                }
            }
        }
        Ok(())
    }

    fn apply_predicate_site(
        &mut self,
        predicate: &mut crate::ir::SemanticOperand<SemanticPredicate>,
    ) -> Result<(), ValueRecoveryError> {
        let Some(replacements) = predicate
            .site
            .and_then(|site| self.site_replacements.get(&UseSite::Predicate(site)))
        else {
            return Ok(());
        };
        let mut substitution = SiteSubstitution {
            replacements,
            identity: self.identity,
            changed: false,
        };
        SemanticInstructions::transform_predicate(predicate, &mut substitution)?;
        self.changed |= substitution.changed;
        Ok(())
    }

    fn apply_expression_site(
        &mut self,
        argument: &mut crate::ir::SemanticOperand<SemanticExpression>,
    ) -> Result<(), ValueRecoveryError> {
        let Some(replacements) = argument
            .site
            .and_then(|site| self.site_replacements.get(&UseSite::Argument(site)))
        else {
            return Ok(());
        };
        let mut substitution = SiteSubstitution {
            replacements,
            identity: self.identity,
            changed: false,
        };
        SemanticInstructions::transform_expression(&mut argument.value, &mut substitution)?;
        self.changed |= substitution.changed;
        Ok(())
    }
}

struct SemanticExpressionReduction {
    identity: ValueIdentity,
}

impl SemanticExpressionReduction {
    fn new(identity: ValueIdentity) -> Self {
        Self { identity }
    }

    fn contains_select(value: &SemanticExpression) -> bool {
        match value {
            SemanticExpression::Select { .. } => true,
            SemanticExpression::Operation(operation) => operation.contains_select(),
            SemanticExpression::Register(_) | SemanticExpression::Literal(_) => false,
        }
    }

    fn predicate_contains_select(predicate: &SemanticPredicate) -> bool {
        let mut pending = Vec::new();
        let mut current = predicate;
        loop {
            match current {
                SemanticPredicate::Test(operation) => {
                    if operation.contains_select() {
                        return true;
                    }
                }
                SemanticPredicate::Not(inner) => pending.push(inner.as_ref()),
                SemanticPredicate::And(terms) | SemanticPredicate::Or(terms) => {
                    pending.extend(terms);
                }
                SemanticPredicate::True | SemanticPredicate::False => {}
            }
            let Some(next) = pending.pop() else {
                return false;
            };
            current = next;
        }
    }

    fn statement_contains_select(statement: &crate::ir::SemanticStatement) -> bool {
        match &statement.kind {
            SemanticStatementKind::Instruction(operation) => operation.contains_select(),
            SemanticStatementKind::Definition { value, .. } => Self::contains_select(value),
        }
    }

    fn node_contains_select(node: &SemanticNode) -> bool {
        match node {
            SemanticNode::BasicBlock(block) => {
                block.statements.iter().any(Self::statement_contains_select)
            }
            SemanticNode::If { condition, .. } => Self::predicate_contains_select(condition),
            SemanticNode::Loop { test, .. } => Self::predicate_contains_select(&test.condition),
            SemanticNode::For {
                init,
                condition,
                update,
                ..
            } => {
                Self::statement_contains_select(init)
                    || Self::predicate_contains_select(condition)
                    || Self::statement_contains_select(update)
            }
            SemanticNode::ForEach { iterable, .. } => Self::contains_select(iterable),
            SemanticNode::Switch { selector, .. } => Self::contains_select(selector),
            SemanticNode::Synchronized { lock, .. } => Self::contains_select(lock),
            SemanticNode::Leave(leave) => {
                leave
                    .condition
                    .as_ref()
                    .is_some_and(Self::predicate_contains_select)
                    || leave.value().is_some_and(Self::contains_select)
            }
            SemanticNode::Empty
            | SemanticNode::Sequence(_)
            | SemanticNode::Try { .. }
            | SemanticNode::Label { .. } => false,
        }
    }

    fn reduce(
        &self,
        value: SemanticExpression,
    ) -> Result<(SemanticExpression, bool), ValueRecoveryError> {
        let mut pending = vec![ValueReductionTask::Visit(value)];
        let mut results = Vec::<SemanticExpression>::new();
        let mut operations = Vec::new();
        let mut changed = false;
        while let Some(task) = pending.pop() {
            match task {
                ValueReductionTask::Visit(
                    expression @ (SemanticExpression::Register(_) | SemanticExpression::Literal(_)),
                ) => results.push(expression),
                ValueReductionTask::Visit(SemanticExpression::Operation(mut operation)) => {
                    // Select reduction only changes choices. Plain sibling operations
                    // retain their original operand storage and shared metadata.
                    if !operation.contains_select() {
                        results.push(SemanticExpression::Operation(operation));
                        continue;
                    }
                    pending.push(ValueReductionTask::FinishOperation);
                    pending.extend(operation.children_mut().rev().map(|value| {
                        ValueReductionTask::Visit(std::mem::replace(
                            value,
                            SemanticExpression::Literal(crate::ir::LiteralArg::int(0)),
                        ))
                    }));
                    operations.push(operation);
                }
                ValueReductionTask::Visit(SemanticExpression::Select {
                    condition,
                    when_true,
                    when_false,
                }) => {
                    pending.push(ValueReductionTask::Select(condition));
                    pending.push(ValueReductionTask::Visit(*when_false));
                    pending.push(ValueReductionTask::Visit(*when_true));
                }
                ValueReductionTask::Select(condition) => {
                    let when_false = results
                        .pop()
                        .ok_or(crate::ir::SemanticFoldError::MalformedWorkStack)?;
                    let when_true = results
                        .pop()
                        .ok_or(crate::ir::SemanticFoldError::MalformedWorkStack)?;
                    if let Some(value) = condition.constant_value() {
                        results.push(if value { when_true } else { when_false });
                        changed = true;
                    } else if self.equivalent(&when_true, &when_false) {
                        results.push(when_true);
                        changed = true;
                    } else {
                        results.push(SemanticExpression::select(condition, when_true, when_false));
                    }
                }
                ValueReductionTask::FinishOperation => {
                    let mut operation = operations
                        .pop()
                        .ok_or(crate::ir::SemanticFoldError::MalformedWorkStack)?;
                    let count = operation.operands().len()
                        + usize::from(operation.compound_target().is_some());
                    let start = results
                        .len()
                        .checked_sub(count)
                        .ok_or(crate::ir::SemanticFoldError::MalformedWorkStack)?;
                    for (slot, value) in operation.children_mut().zip(results.drain(start..)) {
                        *slot = value;
                    }
                    results.push(SemanticExpression::Operation(operation));
                }
            }
        }
        if results.len() != 1 {
            return Err(crate::ir::SemanticFoldError::MalformedWorkStack.into());
        }
        Ok((
            results
                .pop()
                .ok_or(crate::ir::SemanticFoldError::MalformedWorkStack)?,
            changed,
        ))
    }

    fn select(
        condition: SemanticPredicate,
        when_true: SemanticExpression,
        when_false: SemanticExpression,
    ) -> (SemanticExpression, bool) {
        match condition.constant_value() {
            Some(true) => (when_true, true),
            Some(false) => (when_false, true),
            None => {
                if let Some(equal_when_true) =
                    Self::stable_equality_selection(&condition, &when_true, &when_false)
                {
                    return if equal_when_true {
                        (when_false, true)
                    } else {
                        (when_true, true)
                    };
                }
                (
                    SemanticExpression::select(condition, when_true, when_false),
                    false,
                )
            }
        }
    }

    fn stable_equality_selection(
        condition: &SemanticPredicate,
        when_true: &SemanticExpression,
        when_false: &SemanticExpression,
    ) -> Option<bool> {
        let (equal_when_true, left, right) = Self::stable_equality_test(condition, true)?;
        let aligned = (when_true.same_stable_value(left) && when_false.same_stable_value(right))
            || (when_true.same_stable_value(right) && when_false.same_stable_value(left));
        aligned.then_some(equal_when_true)
    }

    fn stable_equality_test(
        condition: &SemanticPredicate,
        polarity: bool,
    ) -> Option<(bool, &SemanticExpression, &SemanticExpression)> {
        match condition {
            SemanticPredicate::Not(inner) => Self::stable_equality_test(inner, !polarity),
            SemanticPredicate::Test(test) if test.insn_type == InsnType::If => {
                let equal_when_true = match test.payload.if_op? {
                    IfOp::Eq => polarity,
                    IfOp::Ne => !polarity,
                    IfOp::Lt | IfOp::Ge | IfOp::Gt | IfOp::Le => return None,
                };
                let [left, right] = test.operands() else {
                    return None;
                };
                Some((equal_when_true, left, right))
            }
            SemanticPredicate::True
            | SemanticPredicate::False
            | SemanticPredicate::And(_)
            | SemanticPredicate::Or(_)
            | SemanticPredicate::Test(_) => None,
        }
    }

    fn equivalent(&self, left: &SemanticExpression, right: &SemanticExpression) -> bool {
        let mut pending = vec![(left, right)];
        while let Some((left, right)) = pending.pop() {
            match (left, right) {
                (SemanticExpression::Register(left), SemanticExpression::Register(right)) => {
                    let equivalent = match (self.identity.key(left), self.identity.key(right)) {
                        (Some(left), Some(right)) => left == right,
                        (None, None) => {
                            left.reg_num == right.reg_num
                                && left.ssa_version == right.ssa_version
                                && left.code_var == right.code_var
                        }
                        _ => false,
                    };
                    if !equivalent {
                        return false;
                    }
                }
                (SemanticExpression::Literal(left), SemanticExpression::Literal(right))
                    if left == right => {}
                (SemanticExpression::Operation(left), SemanticExpression::Operation(right)) => {
                    if !left.operation_equivalent(right)
                        || left.operands().len() != right.operands().len()
                        || left.compound_target().is_some() != right.compound_target().is_some()
                    {
                        return false;
                    }
                    pending.extend(left.operands().iter().zip(right.operands()));
                    if let (Some(left), Some(right)) =
                        (left.compound_target(), right.compound_target())
                    {
                        pending.push((left, right));
                    }
                }
                (SemanticExpression::Select { .. }, SemanticExpression::Select { .. }) => {
                    return false;
                }
                _ => return false,
            }
        }
        true
    }
}

enum ValueReductionTask {
    Visit(SemanticExpression),
    Select(SemanticPredicate),
    FinishOperation,
}

struct ReplacementEdges {
    offsets: Vec<usize>,
    targets: Vec<usize>,
}

impl ReplacementEdges {
    fn row(&self, key: usize) -> &[usize] {
        &self.targets[self.offsets[key]..self.offsets[key + 1]]
    }

    fn transpose(&self) -> Self {
        let mut offsets = vec![0; self.offsets.len()];
        for &target in &self.targets {
            offsets[target + 1] += 1;
        }
        for i in 1..offsets.len() {
            offsets[i] += offsets[i - 1];
        }
        let mut next = offsets[..offsets.len() - 1].to_vec();
        let mut targets = vec![0; self.targets.len()];
        for source in 0..next.len() {
            for &target in self.row(source) {
                targets[next[target]] = source;
                next[target] += 1;
            }
        }
        Self { offsets, targets }
    }
}

struct ReplacementGraph {
    keys: Vec<SsaVar>,
    dependencies: ReplacementEdges,
    ordered: bool,
}

#[cfg(test)]
#[path = "../../../tests/internal/replacement_order.rs"]
mod replacement_order_tests;

impl ReplacementGraph {
    fn new(
        replacements: &BTreeMap<SsaVar, SemanticExpression>,
        identity: ValueIdentity,
    ) -> Result<Self, ValueRecoveryError> {
        let keys: Vec<_> = replacements.keys().copied().collect();
        let mut offsets = Vec::with_capacity(keys.len() + 1);
        let mut targets = Vec::new();
        let mut row = Vec::new();
        let mut ordered = true;
        // Validate every original expression in key order, including ones
        // that will later prove cyclic. Each unique internal edge is kept.
        for (source, value) in replacements.values().enumerate() {
            offsets.push(targets.len());
            ExpressionDependencies::visit(value, identity, &mut |key| {
                if let Ok(target) = keys.binary_search(&key) {
                    ordered &= target < source;
                    row.push(target);
                }
            })?;
            row.sort_unstable();
            row.dedup();
            targets.append(&mut row);
        }
        offsets.push(targets.len());
        Ok(Self {
            keys,
            dependencies: ReplacementEdges { offsets, targets },
            ordered,
        })
    }

    fn into_order(self) -> Vec<SsaVar> {
        // If every dependency precedes its consumer, ascending keys are
        // already exactly the smallest-ready-key topological order.
        if self.ordered {
            return self.keys;
        }
        let dependents = self.dependencies.transpose();
        let mut cyclic = vec![false; self.keys.len()];
        let mut order = self.sorted_ready(&dependents, &cyclic);
        if order.len() < self.keys.len() {
            // Only unprocessed nodes can be cyclic. Their dependents may be
            // legal once the actual cycle members retain their definitions.
            self.mark_cycles(&dependents, &order, &mut cyclic);
            order = self.sorted_ready(&dependents, &cyclic);
        }
        order.into_iter().map(|node| self.keys[node]).collect()
    }

    fn sorted_ready(&self, dependents: &ReplacementEdges, cyclic: &[bool]) -> Vec<usize> {
        let mut degrees = Vec::with_capacity(self.keys.len());
        let mut ready = BinaryHeap::new();
        for (node, &excluded) in cyclic.iter().enumerate() {
            let count = self
                .dependencies
                .row(node)
                .iter()
                .filter(|&&input| !cyclic[input])
                .count();
            degrees.push(count);
            if !excluded && count == 0 {
                ready.push(Reverse(node));
            }
        }
        let mut order = Vec::with_capacity(self.keys.len());
        while let Some(Reverse(node)) = ready.pop() {
            order.push(node);
            for &target in dependents.row(node) {
                if !cyclic[target] {
                    degrees[target] -= 1;
                    if degrees[target] == 0 {
                        ready.push(Reverse(target));
                    }
                }
            }
        }
        order
    }

    fn mark_cycles(&self, dependents: &ReplacementEdges, solved: &[usize], cyclic: &mut [bool]) {
        let mut visited = vec![false; self.keys.len()];
        for &node in solved {
            visited[node] = true;
        }
        let mut assigned = visited.clone();
        let mut finish = Vec::new();
        let mut stack = Vec::new();
        for root in 0..visited.len() {
            if std::mem::replace(&mut visited[root], true) {
                continue;
            }
            stack.push((root, 0));
            while let Some((node, index)) = stack.last_mut() {
                if let Some(&child) = self.dependencies.row(*node).get(*index) {
                    *index += 1;
                    if !std::mem::replace(&mut visited[child], true) {
                        stack.push((child, 0));
                    }
                } else {
                    finish.push(*node);
                    stack.pop();
                }
            }
        }
        let mut pending = Vec::new();
        let mut component = Vec::new();
        for root in finish.into_iter().rev() {
            if std::mem::replace(&mut assigned[root], true) {
                continue;
            }
            component.clear();
            pending.push(root);
            while let Some(node) = pending.pop() {
                component.push(node);
                for &dependent in dependents.row(node) {
                    if !std::mem::replace(&mut assigned[dependent], true) {
                        pending.push(dependent);
                    }
                }
            }
            if component.len() > 1 || self.dependencies.row(root).binary_search(&root).is_ok() {
                for &node in &component {
                    cyclic[node] = true;
                }
            }
        }
    }
}

struct ExpressionDependencies;

impl ExpressionDependencies {
    fn visit(
        value: &SemanticExpression,
        identity: ValueIdentity,
        visit: &mut impl FnMut(SsaVar),
    ) -> Result<(), ValueRecoveryError> {
        let mut expressions = smallvec::SmallVec::<[&SemanticExpression; 4]>::new();
        expressions.push(value);
        let mut predicates = Vec::new();
        while let Some(expression) = expressions.pop() {
            match expression {
                SemanticExpression::Register(register) => {
                    if let Some(key) = identity.key(register) {
                        visit(key);
                    }
                }
                SemanticExpression::Operation(operation) => {
                    expressions.extend(operation.evaluation_operands_iter()?);
                }
                SemanticExpression::Select {
                    condition,
                    when_true,
                    when_false,
                } => {
                    predicates.push(condition);
                    expressions.push(when_false);
                    expressions.push(when_true);
                }
                SemanticExpression::Literal(_) => {}
            }
        }
        while let Some(predicate) = predicates.pop() {
            match predicate {
                SemanticPredicate::Test(operation) => {
                    for operand in operation.evaluation_operands_iter()? {
                        Self::visit(operand, identity, visit)?;
                    }
                }
                SemanticPredicate::Not(inner) => predicates.push(inner),
                SemanticPredicate::And(terms) | SemanticPredicate::Or(terms) => {
                    predicates.extend(terms);
                }
                SemanticPredicate::True | SemanticPredicate::False => {}
            }
        }
        Ok(())
    }

    fn substitute(
        mut value: SemanticExpression,
        replacements: &BTreeMap<SsaVar, SemanticExpression>,
        identity: ValueIdentity,
    ) -> Result<SemanticExpression, crate::ir::SemanticFoldError> {
        let discarded_results = BTreeSet::new();
        SemanticInstructions::transform_expression(
            &mut value,
            &mut ValueSubstitution {
                replacements,
                discarded_results: &discarded_results,
                identity,
                changed: false,
            },
        )?;
        Ok(value)
    }
}

struct ValueSubstitution<'a> {
    replacements: &'a BTreeMap<SsaVar, SemanticExpression>,
    discarded_results: &'a BTreeSet<SsaVar>,
    identity: ValueIdentity,
    changed: bool,
}

struct SiteSubstitution<'a> {
    replacements: &'a BTreeMap<SsaVar, SiteReplacement>,
    identity: ValueIdentity,
    changed: bool,
}

impl SemanticExpressionTransform for SiteSubstitution<'_> {
    fn transform_register(&mut self, register: RegisterArg) -> SemanticExpression {
        let replacement = self
            .identity
            .key(&register)
            .and_then(|key| self.replacements.get(&key))
            .map(|replacement| replacement.value.clone());
        self.changed |= replacement.is_some();
        replacement.unwrap_or(SemanticExpression::Register(register))
    }

    fn transform_select(
        &mut self,
        condition: SemanticPredicate,
        when_true: SemanticExpression,
        when_false: SemanticExpression,
    ) -> SemanticExpression {
        let (expression, changed) =
            SemanticExpressionReduction::select(condition, when_true, when_false);
        self.changed |= changed;
        expression
    }
}

impl ValueSubstitution<'_> {
    fn discards_result(&self, operation: &SemanticOperation) -> bool {
        operation
            .result
            .as_ref()
            .and_then(|register| self.identity.key(register))
            .is_some_and(|key| self.discarded_results.contains(&key))
    }
}

impl SemanticExpressionTransform for ValueSubstitution<'_> {
    fn preserves_direct_operation(&self, operation: &SemanticOperation) -> bool {
        !self.discards_result(operation)
            && !operation
                .operands()
                .iter()
                .chain(operation.compound_target())
                .any(|value| match value {
                    SemanticExpression::Register(register) => self
                        .identity
                        .key(register)
                        .is_some_and(|key| self.replacements.contains_key(&key)),
                    _ => false,
                })
    }

    fn transform_register(&mut self, register: RegisterArg) -> SemanticExpression {
        let replacement = self
            .identity
            .key(&register)
            .and_then(|key| self.replacements.get(&key))
            .cloned();
        self.changed |= replacement.is_some();
        replacement.unwrap_or(SemanticExpression::Register(register))
    }

    fn transform_operation(&mut self, mut operation: SemanticOperation) -> SemanticExpression {
        if self.discards_result(&operation) {
            operation.discard_result();
            self.changed = true;
        }
        SemanticExpression::Operation(operation)
    }

    fn transform_select(
        &mut self,
        condition: SemanticPredicate,
        when_true: SemanticExpression,
        when_false: SemanticExpression,
    ) -> SemanticExpression {
        let (expression, changed) =
            SemanticExpressionReduction::select(condition, when_true, when_false);
        self.changed |= changed;
        expression
    }
}

#[cfg(test)]
#[path = "../../../tests/internal/selection_reduction_storage.rs"]
mod selection_reduction_storage;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{ArgType, InsnArg, InsnNode, InsnType};

    #[test]
    fn select_scan_skips_flat_expressions_and_finds_nested_selects() {
        let literal =
            |value| SemanticExpression::from_argument(InsnArg::lit(value, ArgType::INT)).unwrap();
        let flat =
            SemanticOperation::from_parts(InsnNode::new(InsnType::Move, 1), vec![literal(1)], None);
        assert!(!flat.contains_select());

        let nested = SemanticOperation::from_parts(
            InsnNode::new(InsnType::Move, 1),
            vec![SemanticExpression::select(
                SemanticPredicate::True,
                literal(1),
                literal(0),
            )],
            None,
        );
        assert!(nested.contains_select());
    }

    #[test]
    fn strongly_connected_replacements_keep_their_definitions() {
        let first = SsaVar::new(0, 0);
        let second = SsaVar::new(0, 1);
        let dependent = SsaVar::new(1, 0);
        let register = |value: SsaVar| InsnArg::reg_ssa(value.reg_num, value.version, ArgType::INT);
        let schedule = ValueSchedule::compile(
            [
                ValueAction::Propagate {
                    event: 0,
                    key: first,
                    replacement: register(second),
                },
                ValueAction::Propagate {
                    event: 1,
                    key: second,
                    replacement: register(first),
                },
                ValueAction::Propagate {
                    event: 2,
                    key: dependent,
                    replacement: register(first),
                },
            ],
            ValueIdentity::Ssa,
        )
        .unwrap();

        assert!(!schedule.replacements.contains_key(&first));
        assert!(!schedule.replacements.contains_key(&second));
        assert!(!schedule.removals.contains(&first));
        assert!(!schedule.removals.contains(&second));
        assert_eq!(
            schedule
                .replacements
                .get(&dependent)
                .and_then(SemanticExpression::as_register)
                .and_then(SsaVar::from_reg),
            Some(first)
        );
        assert!(schedule.removals.contains(&dependent));
    }

    #[test]
    fn constructor_receiver_is_not_an_expression_dependency() {
        let result = SsaVar::new(0, 0);
        let first = SsaVar::new(1, 0);
        let second = SsaVar::new(2, 0);
        let object = ArgType::object("example/Value");
        let register =
            |value: SsaVar, ty: ArgType| InsnArg::reg_ssa(value.reg_num, value.version, ty);
        let mut constructor = InsnNode::new(InsnType::Constructor, 3);
        constructor.set_result(RegisterArg::new_ssa(
            result.reg_num,
            result.version,
            object.clone(),
        ));
        constructor.add_arg(register(result, object));
        constructor.add_arg(register(first, ArgType::INT));
        constructor.add_arg(register(second, ArgType::INT));

        let expression = SemanticExpression::from_argument(InsnArg::wrap(constructor)).unwrap();
        let mut dependencies = BTreeSet::new();
        ExpressionDependencies::visit(&expression, ValueIdentity::Ssa, &mut |key| {
            dependencies.insert(key);
        })
        .unwrap();

        assert_eq!(dependencies, BTreeSet::from([first, second]));
    }

    #[test]
    fn replacement_dependencies_remain_live() {
        let value = SsaVar::new(0, 0);
        let dependency = SsaVar::new(1, 0);
        let mut expression = InsnNode::mov(
            RegisterArg::new_ssa(value.reg_num, value.version, ArgType::INT),
            InsnArg::reg_ssa(dependency.reg_num, dependency.version, ArgType::INT),
        );
        expression.payload.edge_copy = true;

        let schedule = ValueSchedule::compile(
            [
                ValueAction::Inline {
                    event: 0,
                    key: value,
                    expression: SemanticExpression::Operation(
                        SemanticOperation::from_instruction(expression).unwrap(),
                    ),
                },
                ValueAction::DiscardResult {
                    event: 1,
                    key: dependency,
                },
            ],
            ValueIdentity::Ssa,
        )
        .unwrap();

        assert!(!schedule.discarded_results.contains(&dependency));
    }
}
