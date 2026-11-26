use std::{borrow::Cow, cell::RefCell};

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
    fallthrough_domains: std::collections::BTreeMap<usize, ControlDomain>,
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
            fallthrough_domains: std::collections::BTreeMap::new(),
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
        let definition = match &statement.kind {
            SemanticStatementKind::Definition { result, value, .. } => {
                if self.graph.is_identity_definition(result, value) {
                    if let Some(site) = statement.site {
                        self.graph.identity_statements.push((site, event));
                    }
                    return Ok(());
                }
                let mut argument = 0;
                let mut predicate = 0;
                self.visit_semantic_value_uses(
                    value,
                    execution_domain,
                    event,
                    &mut evaluation_prefix,
                    statement.site,
                    &mut argument,
                    &mut predicate,
                    origin,
                )?;
                Some((result, Cow::Borrowed(value)))
            }
            SemanticStatementKind::Instruction(instruction) => {
                if self.graph.is_identity_move(instruction) {
                    if let Some(site) = statement.site {
                        self.graph.identity_statements.push((site, event));
                    }
                    return Ok(());
                }
                let site = statement.site.map(UseSite::Statement);
                self.visit_operation_uses(
                    instruction,
                    execution_domain,
                    event,
                    &mut evaluation_prefix,
                    UseContext::Value,
                    site,
                    origin,
                )?;
                instruction.result.as_ref().map(|result| {
                    (
                        result,
                        Cow::Owned(SemanticExpression::Operation(Box::new(instruction.clone()))),
                    )
                })
            }
        };
        if let Some((result, value)) = definition {
            self.record_definition(statement, result, value, execution_domain, event);
        }
        self.record_effect(
            statement.site,
            execution_domain,
            EffectSummary::statement(statement),
        );
        Ok(())
    }

    fn record_definition(
        &mut self,
        statement: &'ir SemanticStatement,
        result: &'ir crate::ir::RegisterArg,
        value: Cow<'ir, SemanticExpression>,
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
                value,
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
    ) -> Result<(), ValueRecoveryError> {
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
                Ok(())
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

    fn fallthrough_domain(
        &mut self,
        node: &SemanticNode,
    ) -> Result<ControlDomain, ValueRecoveryError> {
        let key = std::ptr::from_ref(node).addr();
        if let Some(domain) = self.fallthrough_domains.get(&key).copied() {
            return Ok(domain);
        }
        Err(ValueRecoveryError::IncompleteSemanticFlow)
    }

    fn visit_predicate_uses(
        &mut self,
        predicate: &SemanticPredicate,
        domain: ControlDomain,
        event: usize,
        site: Option<UseSite>,
        origin: Option<&crate::ir::analysis::SemanticFlowPoint>,
    ) -> Result<EffectSummary, ValueRecoveryError> {
        let mut pending = vec![(predicate, domain)];
        let mut evaluation_prefix = EffectSummary::pure();
        while let Some((predicate, domain)) = pending.pop() {
            match predicate {
                SemanticPredicate::True | SemanticPredicate::False => {}
                SemanticPredicate::Test(insn) => {
                    if let Some(operand_point) = origin.copied().filter(|_| insn.id.is_valid()) {
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
                SemanticPredicate::Not(inner) => pending.push((inner, domain)),
                SemanticPredicate::And(terms) | SemanticPredicate::Or(terms) => {
                    let conjunction = matches!(predicate, SemanticPredicate::And(_));
                    let mut evaluation_domain = domain;
                    let mut visits = Vec::with_capacity(terms.len());
                    for term in terms {
                        visits.push((term, evaluation_domain));
                        let term_domain = self.graph.logic.predicate(term)?;
                        let continuation = if conjunction {
                            term_domain
                        } else {
                            self.graph.logic.not(term_domain)?
                        };
                        evaluation_domain =
                            self.graph.logic.and(evaluation_domain, continuation)?;
                    }
                    pending.extend(visits.into_iter().rev());
                }
            }
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
    ) -> Result<(), ValueRecoveryError> {
        let operands = operation.evaluation_operands_iter()?;
        if operands.clone().all(|operand| {
            matches!(
                operand,
                SemanticExpression::Register(_) | SemanticExpression::Literal(_)
            )
        }) {
            for operand in operands {
                if let SemanticExpression::Register(register) = operand {
                    self.record_use(
                        register,
                        domain,
                        event,
                        prefix,
                        context,
                        Some(operation.insn_type),
                        site,
                        origin,
                    );
                }
            }
            *prefix = std::mem::take(prefix).join(EffectSummary::direct(operation));
            return Ok(());
        }
        let mut pending = vec![EvaluationTask::Effect(operation)];
        pending.extend(
            operands
                .rev()
                .map(|expression| EvaluationTask::Expression(expression, operation.insn_type)),
        );
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
                    pending.extend(
                        child.evaluation_operands_iter()?.rev().map(|expression| {
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
        Ok(())
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
    ) -> Result<(), ValueRecoveryError> {
        match expression {
            SemanticExpression::Register(register) => {
                self.record_use(
                    register, domain, event, prefix, context, consumer, site, origin,
                );
                Ok(())
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
                Ok(())
            }
            SemanticExpression::Literal(_) => Ok(()),
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

struct ControlFallthrough<'a> {
    logic: RefCell<&'a mut crate::analysis::value_recovery::domain::DomainLogic>,
}

impl<'a> ControlFallthrough<'a> {
    fn analyze(
        root: &SemanticNode,
        logic: &'a mut crate::analysis::value_recovery::domain::DomainLogic,
    ) -> Result<std::collections::BTreeMap<usize, ControlDomain>, ValueRecoveryError> {
        CompletionInterpreter::analyze_facts(
            root,
            &Self {
                logic: RefCell::new(logic),
            },
        )
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
