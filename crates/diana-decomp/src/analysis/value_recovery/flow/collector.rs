use std::cell::RefCell;

use crate::ir::semantic::{CompletionDomain, CompletionInterpreter};
use crate::ir::{
    InsnType, SemanticExpression, SemanticLabel, SemanticLeave, SemanticLoopControl,
    SemanticLoopKind, SemanticNode, SemanticOperation, SemanticPredicate, SemanticStatement,
    SemanticStatementKind,
};

use super::{
    effects::EffectSummary, DefinitionFact, EffectFact, PredicateUseFact, ScopeId, UseContext,
    UseFact, UseSite, ValueFlowGraph,
};
use crate::analysis::value_recovery::{domain::ControlDomain, ValueRecoveryError};

mod scheduler;

use scheduler::NodeScheduler;

pub(super) struct FlowCollector<'ir> {
    graph: ValueFlowGraph<'ir>,
    next_event: usize,
    next_protection: u32,
    scope: ScopeId,
    loop_owner: ScopeId,
    enclosing_scopes: Vec<(ScopeId, ScopeId)>,
    protections: Vec<u32>,
    fallthrough_domains: SequenceFallthrough,
}

impl<'ir> FlowCollector<'ir> {
    pub(super) fn new(graph: ValueFlowGraph<'ir>) -> Self {
        Self {
            graph,
            next_event: 0,
            next_protection: 0,
            scope: ScopeId::default(),
            loop_owner: ScopeId::default(),
            enclosing_scopes: Vec::new(),
            protections: Vec::new(),
            fallthrough_domains: SequenceFallthrough::default(),
        }
    }

    pub(super) fn collect(
        mut self,
        root: &'ir SemanticNode,
    ) -> Result<ValueFlowGraph<'ir>, ValueRecoveryError> {
        self.fallthrough_domains = ControlFallthrough::analyze(root, &mut self.graph.logic)?;
        self.collect_facts(root)?;
        Ok(self.graph)
    }

    fn collect_facts(&mut self, root: &'ir SemanticNode) -> Result<(), ValueRecoveryError> {
        let mut pending = vec![FlowTask::Node(root, self.graph.logic.truth())];
        while let Some(task) = pending.pop() {
            match task {
                FlowTask::Node(node, domain) => {
                    NodeScheduler::new(self, domain, &mut pending).schedule(node)?
                }
                FlowTask::Statement(statement, domain) => {
                    self.visit_statement(statement, domain)?
                }
                FlowTask::Predicate(predicate, domain, site) => {
                    let event = self.event();
                    let point = site.map(UseSite::before);
                    let effect =
                        self.visit_predicate_uses(predicate, domain, event, site, point.as_ref())?;
                    self.record_effect(site.map(UseSite::id), domain, effect);
                }
                FlowTask::EnterScope(repetitive) => self.enter_scope(repetitive),
                FlowTask::LeaveScope(repetitive) => self.leave_scope(repetitive)?,
                FlowTask::EnterProtection(protection) => self.enter_protection(protection),
                FlowTask::LeaveProtection(protection) => self.leave_protection(protection)?,
            }
        }
        if !self.enclosing_scopes.is_empty() || !self.protections.is_empty() {
            return Err(ValueRecoveryError::UnclosedScope);
        }
        Ok(())
    }

    fn visit_statement(
        &mut self,
        statement: &'ir SemanticStatement,
        domain: ControlDomain,
    ) -> Result<(), ValueRecoveryError> {
        let event = self.event();
        let point = statement
            .site
            .map(crate::ir::analysis::SemanticFlowPoint::before);
        let origin = point.as_ref();
        let mut evaluation_prefix = EffectSummary::pure();
        let execution_domain = domain;
        let (definition, prefix_covers_statement) = match &statement.kind {
            SemanticStatementKind::Definition { result, value, .. } => {
                if self.graph.is_identity_definition(result, value) {
                    if let Some(site) = statement.site {
                        self.graph.identity_statements.push((site, event));
                    }
                    return Ok(());
                }
                let mut argument = 0;
                let mut predicate = 0;
                let covers_value = self.visit_semantic_value_uses(
                    value,
                    execution_domain,
                    event,
                    &mut evaluation_prefix,
                    statement.site,
                    &mut argument,
                    &mut predicate,
                    origin,
                )?;
                (
                    Some(result),
                    covers_value && !value.as_operation().is_some_and(|op| op.payload.edge_copy),
                )
            }
            SemanticStatementKind::Instruction(instruction) => {
                if self.graph.is_identity_move(instruction) {
                    if let Some(site) = statement.site {
                        self.graph.identity_statements.push((site, event));
                    }
                    return Ok(());
                }
                let site = statement.site.map(UseSite::Statement);
                let covers_operation = self.visit_operation_uses(
                    instruction,
                    execution_domain,
                    event,
                    &mut evaluation_prefix,
                    UseContext::Value,
                    site,
                    origin,
                )?;
                (
                    instruction.result.as_ref(),
                    covers_operation && !instruction.payload.edge_copy,
                )
            }
        };
        if let Some(result) = definition {
            self.record_definition(statement, result, execution_domain, event);
        }
        if let Some(site) = statement.site {
            let effect = if prefix_covers_statement {
                evaluation_prefix
            } else {
                EffectSummary::statement(statement)
            };
            self.record_effect(Some(site), execution_domain, effect);
        }
        Ok(())
    }

    fn record_definition(
        &mut self,
        statement: &'ir SemanticStatement,
        result: &'ir crate::ir::RegisterArg,
        domain: ControlDomain,
        event: usize,
    ) {
        let Some(key) = self.graph.key(result) else {
            return;
        };
        self.graph
            .definitions
            .entry(key)
            .or_default()
            .push(DefinitionFact {
                key,
                site: statement.site,
                origin: statement.origin.as_ref(),
                instruction: statement.id(),
                result,
                statement,
                domain,
                scope: self.scope,
                protection: self.protections.last().copied(),
                loop_owner: self.loop_owner,
                event,
                repetitive: self.loop_owner != ScopeId::default(),
            });
    }

    fn visit_semantic_value_uses(
        &mut self,
        value: &SemanticExpression,
        domain: ControlDomain,
        event: usize,
        prefix: &mut EffectSummary,
        site: Option<crate::ir::SemanticSiteId>,
        next_argument: &mut u32,
        next_predicate: &mut u32,
        origin: Option<&crate::ir::analysis::SemanticFlowPoint>,
    ) -> Result<bool, ValueRecoveryError> {
        match value {
            SemanticExpression::Register(_)
            | SemanticExpression::Literal(_)
            | SemanticExpression::Operation(_) => {
                let current = *next_argument;
                *next_argument += 1;
                let argument_site = site.map(|site| UseSite::SelectedArgument(site, current));
                self.visit_expression_in_order(
                    value,
                    domain,
                    event,
                    prefix,
                    UseContext::Value,
                    None,
                    argument_site,
                    origin,
                )
            }
            SemanticExpression::Select {
                condition,
                when_true,
                when_false,
            } => {
                let current = *next_predicate;
                *next_predicate += 1;
                *prefix = prefix.clone().join(self.visit_predicate_uses(
                    condition,
                    domain,
                    event,
                    site.map(|site| UseSite::SelectedPredicate(site, current)),
                    origin,
                )?);
                let condition = self.graph.logic.predicate(condition)?;
                let true_domain = self.graph.logic.and(domain, condition)?;
                let negated = self.graph.logic.not(condition)?;
                let false_domain = self.graph.logic.and(domain, negated)?;
                let mut true_prefix = prefix.clone();
                self.visit_semantic_value_uses(
                    when_true,
                    true_domain,
                    event,
                    &mut true_prefix,
                    site,
                    next_argument,
                    next_predicate,
                    origin,
                )?;
                let mut false_prefix = prefix.clone();
                self.visit_semantic_value_uses(
                    when_false,
                    false_domain,
                    event,
                    &mut false_prefix,
                    site,
                    next_argument,
                    next_predicate,
                    origin,
                )?;
                *prefix = true_prefix.join(false_prefix);
                // The scheduling prefix keeps control from the condition;
                // the effect of the selected value strips that control.
                Ok(false)
            }
        }
    }

    fn visit_leave_uses(
        &mut self,
        value: &SemanticExpression,
        condition: Option<&SemanticPredicate>,
        domain: ControlDomain,
        event: usize,
        site: Option<UseSite>,
        origin: Option<&crate::ir::analysis::SemanticFlowPoint>,
    ) -> Result<EffectSummary, ValueRecoveryError> {
        let Some(condition) = condition else {
            return self.visit_expression_uses(value, domain, event, site, origin);
        };
        let mut prefix = self.visit_predicate_uses(condition, domain, event, None, origin)?;
        self.visit_expression_in_order(
            value,
            domain,
            event,
            &mut prefix,
            UseContext::Value,
            None,
            site,
            origin,
        )?;
        Ok(prefix)
    }

    fn record_block(
        &mut self,
        block: crate::ir::BlockId,
        domain: ControlDomain,
    ) -> Result<(), ValueRecoveryError> {
        let domain = match self.graph.block_domains.get(&block).copied() {
            Some(existing) => self.graph.logic.or(existing, domain)?,
            None => domain,
        };
        self.graph.block_domains.insert(block, domain);
        Ok(())
    }

    fn visit_predicate_uses(
        &mut self,
        predicate: &SemanticPredicate,
        domain: ControlDomain,
        event: usize,
        site: Option<UseSite>,
        origin: Option<&crate::ir::analysis::SemanticFlowPoint>,
    ) -> Result<EffectSummary, ValueRecoveryError> {
        let mut pending = Vec::new();
        let mut current = (predicate, domain);
        let mut evaluation_prefix = EffectSummary::pure();
        loop {
            let (predicate, domain) = current;
            match predicate {
                SemanticPredicate::True | SemanticPredicate::False => {}
                SemanticPredicate::Test(insn) => {
                    if let Some(operand_point) = origin.copied().filter(|_| {
                        self.graph.identity() == super::ValueIdentity::Source && insn.id.is_valid()
                    }) {
                        let effect_point = site
                            .map(UseSite::id)
                            .map(crate::ir::analysis::SemanticFlowPoint::after)
                            .unwrap_or(operand_point);
                        self.graph
                            .predicate_uses
                            .entry(insn.id)
                            .or_default()
                            .insert(PredicateUseFact {
                                operand_point,
                                effect_point,
                                domain,
                            });
                    }
                    self.visit_operation_uses(
                        insn,
                        domain,
                        event,
                        &mut evaluation_prefix,
                        UseContext::Predicate,
                        site,
                        origin,
                    )?;
                }
                SemanticPredicate::Not(inner) => {
                    current = (inner.as_ref(), domain);
                    continue;
                }
                SemanticPredicate::And(terms) | SemanticPredicate::Or(terms) => {
                    let conjunction = matches!(predicate, SemanticPredicate::And(_));
                    let mut evaluation_domain = domain;
                    let start = pending.len();
                    for term in terms {
                        pending.push((term, evaluation_domain));
                        let term_domain = self.graph.logic.predicate(term)?;
                        let continuation = if conjunction {
                            term_domain
                        } else {
                            self.graph.logic.not(term_domain)?
                        };
                        evaluation_domain =
                            self.graph.logic.and(evaluation_domain, continuation)?;
                    }
                    pending[start..].reverse();
                }
            }
            let Some(next) = pending.pop() else { break };
            current = next;
        }
        Ok(evaluation_prefix)
    }

    fn visit_expression_uses(
        &mut self,
        expression: &SemanticExpression,
        domain: ControlDomain,
        event: usize,
        site: Option<UseSite>,
        origin: Option<&crate::ir::analysis::SemanticFlowPoint>,
    ) -> Result<EffectSummary, ValueRecoveryError> {
        let mut prefix = EffectSummary::pure();
        self.visit_expression_in_order(
            expression,
            domain,
            event,
            &mut prefix,
            UseContext::Value,
            None,
            site,
            origin,
        )?;
        Ok(prefix)
    }

    fn visit_operation_uses(
        &mut self,
        operation: &SemanticOperation,
        domain: ControlDomain,
        event: usize,
        prefix: &mut EffectSummary,
        context: UseContext,
        site: Option<UseSite>,
        origin: Option<&crate::ir::analysis::SemanticFlowPoint>,
    ) -> Result<bool, ValueRecoveryError> {
        let (mut operands, mut covers_operation) = evaluation_operands(operation)?;
        let nested = loop {
            match operands.next() {
                Some(SemanticExpression::Register(register)) => self.record_use(
                    register,
                    domain,
                    event,
                    prefix,
                    context,
                    Some(operation.insn_type),
                    site,
                    origin,
                ),
                Some(SemanticExpression::Literal(_)) => {}
                Some(nested) => break nested,
                None => {
                    *prefix = std::mem::take(prefix).join(EffectSummary::direct(operation));
                    return Ok(covers_operation);
                }
            }
        };
        let mut pending = vec![EvaluationTask::Effect(operation)];
        pending.extend(
            operands
                .rev()
                .map(|expression| EvaluationTask::Expression(expression, operation.insn_type)),
        );
        pending.push(EvaluationTask::Expression(nested, operation.insn_type));
        while let Some(task) = pending.pop() {
            match task {
                EvaluationTask::Expression(SemanticExpression::Register(register), consumer) => {
                    self.record_use(
                        register,
                        domain,
                        event,
                        prefix,
                        context,
                        Some(consumer),
                        site,
                        origin,
                    );
                }
                EvaluationTask::Expression(SemanticExpression::Operation(child), _) => {
                    pending.push(EvaluationTask::Effect(child));
                    let (operands, covers_child) = evaluation_operands(child)?;
                    covers_operation &= covers_child;
                    pending.extend(
                        operands.rev().map(|expression| {
                            EvaluationTask::Expression(expression, child.insn_type)
                        }),
                    );
                }
                EvaluationTask::Expression(
                    SemanticExpression::Select {
                        condition,
                        when_true,
                        when_false,
                    },
                    consumer,
                ) => {
                    covers_operation = false;
                    *prefix = prefix
                        .clone()
                        .join(self.visit_predicate_uses(condition, domain, event, site, origin)?);
                    let condition = self.graph.logic.predicate(condition)?;
                    let true_domain = self.graph.logic.and(domain, condition)?;
                    let negated = self.graph.logic.not(condition)?;
                    let false_domain = self.graph.logic.and(domain, negated)?;
                    let mut true_prefix = prefix.clone();
                    self.visit_expression_in_order(
                        when_true,
                        true_domain,
                        event,
                        &mut true_prefix,
                        context,
                        Some(consumer),
                        site,
                        origin,
                    )?;
                    let mut false_prefix = prefix.clone();
                    self.visit_expression_in_order(
                        when_false,
                        false_domain,
                        event,
                        &mut false_prefix,
                        context,
                        Some(consumer),
                        site,
                        origin,
                    )?;
                    *prefix = true_prefix.join(false_prefix);
                }
                EvaluationTask::Expression(SemanticExpression::Literal(_), _) => {}
                EvaluationTask::Effect(operation) => {
                    *prefix = std::mem::take(prefix).join(EffectSummary::direct(operation));
                }
            }
        }
        Ok(covers_operation)
    }

    fn record_use(
        &mut self,
        register: &crate::ir::RegisterArg,
        domain: ControlDomain,
        event: usize,
        prefix: &EffectSummary,
        context: UseContext,
        consumer: Option<InsnType>,
        site: Option<UseSite>,
        origin: Option<&crate::ir::analysis::SemanticFlowPoint>,
    ) {
        if let Some(key) = self.graph.key(register) {
            self.graph.has_predicate_uses |= context == UseContext::Predicate;
            self.graph.uses.entry(key).or_default().push(UseFact {
                point: origin.cloned(),
                domain,
                scope: self.scope,
                protection: self.protections.last().copied(),
                loop_owner: self.loop_owner,
                event,
                repetitive: self.loop_owner != ScopeId::default(),
                evaluation_prefix: prefix.clone(),
                context,
                consumer,
                site,
            });
        }
    }

    fn visit_expression_in_order(
        &mut self,
        expression: &SemanticExpression,
        domain: ControlDomain,
        event: usize,
        prefix: &mut EffectSummary,
        context: UseContext,
        consumer: Option<InsnType>,
        site: Option<UseSite>,
        origin: Option<&crate::ir::analysis::SemanticFlowPoint>,
    ) -> Result<bool, ValueRecoveryError> {
        match expression {
            SemanticExpression::Register(register) => {
                self.record_use(
                    register, domain, event, prefix, context, consumer, site, origin,
                );
                Ok(true)
            }
            SemanticExpression::Operation(operation) => {
                self.visit_operation_uses(operation, domain, event, prefix, context, site, origin)
            }
            SemanticExpression::Select {
                condition,
                when_true,
                when_false,
            } => {
                *prefix = prefix
                    .clone()
                    .join(self.visit_predicate_uses(condition, domain, event, site, origin)?);
                let condition = self.graph.logic.predicate(condition)?;
                let true_domain = self.graph.logic.and(domain, condition)?;
                let negated = self.graph.logic.not(condition)?;
                let false_domain = self.graph.logic.and(domain, negated)?;
                let mut true_prefix = prefix.clone();
                self.visit_expression_in_order(
                    when_true,
                    true_domain,
                    event,
                    &mut true_prefix,
                    context,
                    consumer,
                    site,
                    origin,
                )?;
                let mut false_prefix = prefix.clone();
                self.visit_expression_in_order(
                    when_false,
                    false_domain,
                    event,
                    &mut false_prefix,
                    context,
                    consumer,
                    site,
                    origin,
                )?;
                *prefix = true_prefix.join(false_prefix);
                Ok(false)
            }
            SemanticExpression::Literal(_) => Ok(true),
        }
    }

    fn event(&mut self) -> usize {
        let event = self.next_event;
        self.next_event += 1;
        event
    }

    fn protection(&mut self) -> u32 {
        let protection = self.next_protection;
        self.next_protection += 1;
        protection
    }

    pub(super) fn record_effect(
        &mut self,
        site: Option<crate::ir::SemanticSiteId>,
        domain: ControlDomain,
        effect: EffectSummary,
    ) {
        if effect.is_pure() {
            return;
        }
        if let Some(site) = site {
            self.graph.effects.push(EffectFact {
                point: crate::ir::analysis::SemanticFlowPoint::after(site),
                domain,
                effect,
            });
        }
    }

    fn enter_scope(&mut self, repetitive: bool) {
        self.enclosing_scopes.push((self.scope, self.loop_owner));
        self.scope = self.graph.scopes.enter();
        if repetitive {
            self.loop_owner = self.scope;
        }
    }

    fn leave_scope(&mut self, repetitive: bool) -> Result<(), ValueRecoveryError> {
        let (outer, outer_loop) = self
            .enclosing_scopes
            .pop()
            .ok_or(ValueRecoveryError::ScopeUnderflow)?;
        if (self.loop_owner == self.scope) != repetitive {
            return Err(ValueRecoveryError::ScopeKindMismatch);
        }
        self.graph.scopes.close(self.scope);
        self.scope = outer;
        self.loop_owner = outer_loop;
        Ok(())
    }

    fn enter_protection(&mut self, protection: u32) {
        self.protections.push(protection);
    }

    fn leave_protection(&mut self, protection: u32) -> Result<(), ValueRecoveryError> {
        if self.protections.pop() != Some(protection) {
            return Err(ValueRecoveryError::ScopeKindMismatch);
        }
        Ok(())
    }
}

/// Source evaluation visits a subset of the raw operand positions, each at
/// most once. Its two borrowed slices give an exact lower bound on their
/// combined length. A full permutation has the same effect union in any order.
fn evaluation_operands(
    operation: &SemanticOperation,
) -> Result<
    (impl DoubleEndedIterator<Item = &SemanticExpression>, bool),
    crate::ir::analysis::SourceEvaluationError,
> {
    let operands = operation.evaluation_operands_iter()?;
    let raw_count = operation.operands().len() + usize::from(operation.compound_target().is_some());
    let covers_operation = operands.size_hint().0 == raw_count;
    Ok((operands, covers_operation))
}

/// The scheduler reads all children of one sequence together. A nonempty
/// sequence's child slice identifies that occurrence in the immutable tree.
#[derive(Default)]
struct SequenceFallthrough {
    starts: std::collections::BTreeMap<usize, usize>,
    states: Vec<ControlDomain>,
}

impl SequenceFallthrough {
    fn record(&mut self, children: &[SemanticNode], states: &[ControlDomain]) {
        if !children.is_empty() {
            self.starts
                .insert(children.as_ptr().addr(), self.states.len());
            self.states.extend_from_slice(states);
        }
    }

    fn children(&self, children: &[SemanticNode]) -> Result<&[ControlDomain], ValueRecoveryError> {
        if children.is_empty() {
            return Ok(&[]);
        }
        self.starts
            .get(&children.as_ptr().addr())
            .and_then(|&start| self.states.get(start..start.checked_add(children.len())?))
            .ok_or(ValueRecoveryError::IncompleteSemanticFlow)
    }
}

struct ControlFallthrough<'a> {
    logic: RefCell<&'a mut crate::analysis::value_recovery::domain::DomainLogic>,
}

impl<'a> ControlFallthrough<'a> {
    fn analyze(
        root: &SemanticNode,
        logic: &'a mut crate::analysis::value_recovery::domain::DomainLogic,
    ) -> Result<SequenceFallthrough, ValueRecoveryError> {
        let mut sequences = SequenceFallthrough::default();
        CompletionInterpreter::analyze_sequences(
            root,
            &Self {
                logic: RefCell::new(logic),
            },
            |children, states| sequences.record(children, states),
        )?;
        Ok(sequences)
    }

    fn predicate(
        &self,
        predicate: &SemanticPredicate,
    ) -> Result<ControlDomain, ValueRecoveryError> {
        self.logic.borrow_mut().predicate(predicate)
    }
}

impl CompletionDomain for ControlFallthrough<'_> {
    type State = ControlDomain;
    type Error = ValueRecoveryError;

    fn normal(&self) -> Result<Self::State, Self::Error> {
        Ok(self.logic.borrow().truth())
    }

    fn no_return_call(&self) -> Result<Self::State, Self::Error> {
        Ok(self.logic.borrow().falsity())
    }

    fn leave(&self, _leave: &SemanticLeave) -> Result<Self::State, Self::Error> {
        Ok(self.logic.borrow().falsity())
    }

    fn sequence(
        &self,
        children: impl IntoIterator<Item = Self::State>,
    ) -> Result<Self::State, Self::Error> {
        let mut logic = self.logic.borrow_mut();
        let mut state = logic.truth();
        for child in children {
            state = logic.and(state, child)?;
        }
        Ok(state)
    }

    fn branch(
        &self,
        condition: &SemanticPredicate,
        then_state: Self::State,
        else_state: Option<Self::State>,
    ) -> Result<Self::State, Self::Error> {
        let condition = self.predicate(condition)?;
        let mut logic = self.logic.borrow_mut();
        let then_path = logic.and(condition, then_state)?;
        let otherwise = logic.not(condition)?;
        let normal = logic.truth();
        let else_path = logic.and(otherwise, else_state.unwrap_or(normal))?;
        Ok(logic.or(then_path, else_path)?)
    }

    fn loop_node(
        &self,
        _control: SemanticLoopControl,
        kind: SemanticLoopKind,
        _condition: &SemanticPredicate,
        setup: Self::State,
        _body: Self::State,
    ) -> Result<Self::State, Self::Error> {
        Ok(match kind {
            SemanticLoopKind::PreTested | SemanticLoopKind::Endless => setup,
            SemanticLoopKind::PostTested => self.logic.borrow().truth(),
        })
    }

    fn for_node(
        &self,
        _control: SemanticLoopControl,
        _condition: &SemanticPredicate,
        _body: Self::State,
    ) -> Result<Self::State, Self::Error> {
        Ok(self.logic.borrow().truth())
    }

    fn for_each_node(
        &self,
        _control: SemanticLoopControl,
        _body: Self::State,
    ) -> Result<Self::State, Self::Error> {
        Ok(self.logic.borrow().truth())
    }

    fn switch_node(
        &self,
        _region: Option<crate::ir::RegionId>,
        _has_default: bool,
        _cases: impl IntoIterator<Item = Self::State>,
    ) -> Result<Self::State, Self::Error> {
        Ok(self.logic.borrow().truth())
    }

    fn try_node(
        &self,
        _catches: usize,
        _has_finally: bool,
        _children: impl DoubleEndedIterator<Item = Self::State>,
    ) -> Result<Self::State, Self::Error> {
        Ok(self.logic.borrow().truth())
    }

    fn synchronized(&self, body: Self::State) -> Result<Self::State, Self::Error> {
        Ok(body)
    }

    fn label(&self, _label: SemanticLabel, _body: Self::State) -> Result<Self::State, Self::Error> {
        Ok(self.logic.borrow().truth())
    }
}

enum FlowTask<'a> {
    Node(&'a SemanticNode, ControlDomain),
    Statement(&'a SemanticStatement, ControlDomain),
    Predicate(&'a SemanticPredicate, ControlDomain, Option<UseSite>),
    EnterScope(bool),
    LeaveScope(bool),
    EnterProtection(u32),
    LeaveProtection(u32),
}

enum EvaluationTask<'a> {
    Expression(&'a SemanticExpression, InsnType),
    Effect(&'a SemanticOperation),
}

#[cfg(test)]
#[path = "../../../../tests/internal/statement_effects.rs"]
mod tests;
