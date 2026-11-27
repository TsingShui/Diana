use std::{
    cell::OnceCell,
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

use crate::ir::{
    analysis::{PhiMerge, SsaValueGraph, SsaVar},
    BoolVariable, InsnArg, InsnType, SemanticExpression, SemanticNode, SemanticOperation,
    SemanticPredicate, SemanticVisitor, StatementOrigin, Utf16String,
};

mod collector;
mod effect_versions;
mod effects;
mod facts;
mod gated;
mod occurrences;
mod planner;
mod reaching;

use super::constant::ConstantEvaluator;
use super::domain::{ControlDomain, DomainLogic};
use super::source::SourceFlowCache;
use super::ValueRecoveryError;
use collector::FlowCollector;
use effects::{EffectSummary, EvaluationPrefix, EvaluationPrefixes};
use gated::GatedPhiAnalysis;
pub(super) use gated::{GatedControlFlow, PhiExpressionPlacement};
use occurrences::OccurrenceRows;
use planner::ValuePlanner;

/// Scopes receive their number on entry. All descendants are visited before
/// exit, so a prefix of the old lexical path is exactly a containing interval.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct ScopeId(usize);

#[derive(Default)]
pub(super) struct LexicalScopes {
    last_descendant: Vec<usize>,
}

impl LexicalScopes {
    fn enter(&mut self) -> ScopeId {
        self.last_descendant.push(0);
        ScopeId(self.last_descendant.len())
    }

    fn close(&mut self, scope: ScopeId) {
        self.last_descendant[scope.0 - 1] = self.last_descendant.len();
    }

    pub(super) fn contains(&self, outer: ScopeId, inner: ScopeId) -> bool {
        outer.0 == 0 || (outer.0 <= inner.0 && inner.0 <= self.last_descendant[outer.0 - 1])
    }
}

#[derive(Debug, Clone)]
pub(super) struct DefinitionFact<'ir> {
    pub(super) key: SsaVar,
    pub(super) site: Option<crate::ir::SemanticSiteId>,
    pub(super) origin: Option<&'ir StatementOrigin>,
    pub(super) instruction: crate::ir::InstructionId,
    pub(super) result: &'ir crate::ir::RegisterArg,
    statement: &'ir crate::ir::SemanticStatement,
    pub(super) domain: ControlDomain,
    pub(super) scope: ScopeId,
    pub(super) protection: Option<u32>,
    pub(super) loop_owner: ScopeId,
    pub(super) event: usize,
    pub(super) repetitive: bool,
}

/// A source instruction with a result can be the root of a value without
/// having an enclosing SemanticExpression. Its operands remain in the IR.
#[derive(Clone, Copy)]
enum ValueRef<'a> {
    Register(&'a crate::ir::RegisterArg),
    Literal(&'a crate::ir::LiteralArg),
    Operation(&'a SemanticOperation),
    Select {
        condition: &'a SemanticPredicate,
        when_true: &'a SemanticExpression,
        when_false: &'a SemanticExpression,
    },
}

impl<'a> From<&'a SemanticExpression> for ValueRef<'a> {
    fn from(value: &'a SemanticExpression) -> Self {
        match value {
            SemanticExpression::Register(value) => Self::Register(value),
            SemanticExpression::Literal(value) => Self::Literal(value),
            SemanticExpression::Operation(value) => Self::Operation(value),
            SemanticExpression::Select {
                condition,
                when_true,
                when_false,
            } => Self::Select {
                condition,
                when_true,
                when_false,
            },
        }
    }
}

impl DefinitionFact<'_> {
    fn expression(&self) -> ValueRef<'_> {
        match &self.statement.kind {
            crate::ir::SemanticStatementKind::Definition { value, .. } => value.into(),
            crate::ir::SemanticStatementKind::Instruction(operation) => {
                ValueRef::Operation(operation)
            }
        }
    }

    fn owned_expression(&self) -> SemanticExpression {
        match &self.statement.kind {
            crate::ir::SemanticStatementKind::Definition { value, .. } => value.clone(),
            crate::ir::SemanticStatementKind::Instruction(operation) => {
                SemanticExpression::Operation(operation.clone())
            }
        }
    }

    pub(super) fn operation(&self) -> Option<&SemanticOperation> {
        match self.expression() {
            ValueRef::Operation(operation) => Some(operation),
            _ => None,
        }
    }

    fn effects(&self) -> EffectSummary {
        match &self.statement.kind {
            crate::ir::SemanticStatementKind::Definition { value, .. } => {
                EffectSummary::expression(value)
            }
            crate::ir::SemanticStatementKind::Instruction(operation) => {
                EffectSummary::operation(operation)
            }
        }
    }

    fn edge_copy(&self) -> bool {
        self.operation()
            .is_some_and(|operation| operation.payload.edge_copy)
    }
}

#[derive(Debug, Clone)]
struct UseFact {
    location: UseLocation,
    domain: ControlDomain,
    scope: ScopeId,
    protection: Option<u32>,
    loop_owner: ScopeId,
    event: usize,
    repetitive: bool,
    evaluation_prefix: EvaluationPrefix,
    context: UseContext,
    consumer: Option<InsnType>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UseContext {
    Value,
    Predicate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum UseSite {
    Leave(crate::ir::SemanticSiteId),
    Statement(crate::ir::SemanticSiteId),
    SelectedArgument(crate::ir::SemanticSiteId, u32),
    SelectedPredicate(crate::ir::SemanticSiteId, u32),
    Predicate(crate::ir::SemanticSiteId),
    Argument(crate::ir::SemanticSiteId),
}

#[derive(Debug, Clone)]
pub(super) struct ValueExpression {
    pub(super) value: SemanticExpression,
}

/// A read can have a known evaluation point even when the scheduler cannot
/// substitute at that occurrence (the guard of a conditional transfer).
#[derive(Debug, Clone, Copy)]
enum UseLocation {
    Replaceable(UseSite),
    Observation(crate::ir::SemanticSiteId),
    Missing,
}

impl From<Option<UseSite>> for UseLocation {
    fn from(site: Option<UseSite>) -> Self {
        site.map_or(Self::Missing, Self::Replaceable)
    }
}

impl UseLocation {
    fn site(self) -> Option<UseSite> {
        match self {
            Self::Replaceable(site) => Some(site),
            Self::Observation(_) | Self::Missing => None,
        }
    }

    fn id(self) -> Option<crate::ir::SemanticSiteId> {
        match self {
            Self::Replaceable(site) => Some(site.id()),
            Self::Observation(site) => Some(site),
            Self::Missing => None,
        }
    }

    fn point(self) -> Option<crate::ir::analysis::SemanticFlowPoint> {
        self.id()
            .map(crate::ir::analysis::SemanticFlowPoint::before)
    }
}

#[derive(Debug, Clone)]
struct EffectFact {
    point: crate::ir::analysis::SemanticFlowPoint,
    domain: ControlDomain,
    effect: EffectSummary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct PredicateUseFact {
    operand_point: crate::ir::analysis::SemanticFlowPoint,
    effect_point: crate::ir::analysis::SemanticFlowPoint,
    domain: ControlDomain,
}

#[derive(Debug, Clone)]
pub(super) enum ValueAction {
    Propagate {
        event: usize,
        key: SsaVar,
        replacement: InsnArg,
    },
    Inline {
        event: usize,
        key: SsaVar,
        expression: SemanticExpression,
    },
    InlineAt {
        event: usize,
        key: SsaVar,
        remove_key: bool,
        remove_definitions: Vec<crate::ir::SemanticSiteId>,
        replacements: Vec<(UseSite, ValueExpression)>,
    },
    ReplaceAt {
        event: usize,
        key: SsaVar,
        replacements: Vec<(UseSite, InsnArg)>,
    },
    EliminateStatements {
        event: usize,
        statements: Vec<crate::ir::SemanticSiteId>,
    },
    Remove {
        event: usize,
        key: SsaVar,
    },
    DiscardResult {
        event: usize,
        key: SsaVar,
    },
}

#[derive(Default)]
pub(super) struct ValuePlan {
    has_select: bool,
    pub(super) actions: Vec<ValueAction>,
    pub(super) constants: BTreeMap<SsaVar, InsnArg>,
}

impl ValuePlan {
    pub(super) fn apply(
        self,
        root: &mut SemanticNode,
        identity: ValueIdentity,
    ) -> Result<bool, ValueRecoveryError> {
        // Applying an empty schedule can only simplify an existing select.
        if self.actions.is_empty() && !self.has_select {
            return Ok(false);
        }
        super::schedule::ValueSchedule::compile(self.actions, identity)?.apply(root)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RecoveryMode {
    Structural,
    Full,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ValueIdentity {
    Ssa,
    Source,
}

#[derive(Debug, Default)]
struct SsaCopyFlow {
    sources: BTreeMap<SsaVar, SsaVar>,
    omitted: BTreeSet<SsaVar>,
    all_origins: OnceCell<foldhash::HashMap<SsaVar, SsaVar>>,
    omitted_origins: OnceCell<foldhash::HashMap<SsaVar, SsaVar>>,
}

impl SsaCopyFlow {
    fn analyze(
        values: &SsaValueGraph,
        definitions: &OccurrenceRows<'_, DefinitionFact<'_>>,
    ) -> Self {
        let sources: BTreeMap<_, _> = values
            .copies()
            .iter()
            .map(|copy| (copy.result, copy.source))
            .collect();
        let omitted = sources
            .keys()
            .filter(|result| !definitions.contains_key(result))
            .copied()
            .collect();
        Self {
            sources,
            omitted,
            ..Default::default()
        }
    }

    fn source(&self, value: SsaVar) -> SsaVar {
        self.origin(value, false)
    }

    fn omitted_source(&self, value: SsaVar) -> SsaVar {
        self.origin(value, true)
    }

    fn origin(&self, value: SsaVar, omitted_only: bool) -> SsaVar {
        let next = |v: SsaVar| {
            if omitted_only && !self.omitted.contains(&v) {
                return None;
            }
            self.sources.get(&v).copied()
        };
        let Some(direct) = next(value) else {
            return value;
        };
        if direct == value || next(direct).is_none() {
            return direct;
        }
        let origins = if omitted_only {
            &self.omitted_origins
        } else {
            &self.all_origins
        };
        let origins = origins.get_or_init(|| {
            let mut resolved = foldhash::HashMap::default();
            let mut path = Vec::new();
            for &start in self.sources.keys() {
                if resolved.contains_key(&start) {
                    continue;
                }
                let mut cursor = start;
                let root = loop {
                    if let Some(&known) = resolved.get(&cursor) {
                        // Active vertices temporarily point to themselves. A cycle
                        // member's first repeated vertex is itself, while every
                        // incoming tail reaches the first cycle vertex it entered.
                        if known == cursor {
                            if let Some(cycle) = path.iter().position(|v| *v == cursor) {
                                path.truncate(cycle);
                            }
                        }
                        break known;
                    }
                    let Some(parent) = next(cursor) else {
                        break cursor;
                    };
                    resolved.insert(cursor, cursor);
                    path.push(cursor);
                    cursor = parent;
                };
                for vertex in path.drain(..) {
                    resolved.insert(vertex, root);
                }
            }
            resolved
        });
        origins[&value]
    }

    fn aliases(&self) -> impl Iterator<Item = (SsaVar, SsaVar)> + '_ {
        self.omitted
            .iter()
            .copied()
            .map(|result| (result, self.source(result)))
            .filter(|(result, source)| result != source)
    }
}

impl ValueIdentity {
    pub(super) fn key(self, register: &crate::ir::RegisterArg) -> Option<SsaVar> {
        match self {
            Self::Ssa => SsaVar::from_reg(register),
            Self::Source => register.code_var.map(|variable| SsaVar::new(variable, 0)),
        }
    }
}

impl UseSite {
    fn id(self) -> crate::ir::SemanticSiteId {
        match self {
            Self::Leave(site)
            | Self::Statement(site)
            | Self::SelectedArgument(site, _)
            | Self::SelectedPredicate(site, _)
            | Self::Predicate(site)
            | Self::Argument(site) => site,
        }
    }

    fn before(self) -> crate::ir::analysis::SemanticFlowPoint {
        crate::ir::analysis::SemanticFlowPoint::before(self.id())
    }
}

/// Scheduling rules for constants, dead definitions and retained values can
/// finish without a control-flow query. Build the ordinary graph only when a
/// movement, effect or reaching proof requests it.
struct ScheduledFlow<'ir> {
    root: &'ir SemanticNode,
    shared: Option<&'ir SourceFlowCache>,
    graph: OnceCell<Arc<crate::ir::analysis::SemanticFlowGraph>>,
}

impl<'ir> ScheduledFlow<'ir> {
    fn new(root: &'ir SemanticNode, shared: Option<&'ir SourceFlowCache>) -> Self {
        Self {
            root,
            shared,
            graph: OnceCell::new(),
        }
    }

    fn get(&self) -> &crate::ir::analysis::SemanticFlowGraph {
        self.graph.get_or_init(|| match self.shared {
            Some(cache) => cache.get_or_analyze(self.root),
            None => Arc::new(crate::ir::analysis::SemanticFlowGraph::analyze(self.root)),
        })
    }
}

pub(super) struct ValueFlowGraph<'ir> {
    has_select: bool,
    has_predicate_uses: bool,
    scopes: LexicalScopes,
    identity: ValueIdentity,
    logic: DomainLogic,
    definitions: OccurrenceRows<'ir, DefinitionFact<'ir>>,
    definitions_without_sites: BTreeSet<SsaVar>,
    /// Values introduced by lexical binders rather than executable statements.
    /// Their definition classes are intentionally open to statement scheduling.
    bindings: BTreeSet<SsaVar>,
    uses: OccurrenceRows<'ir, UseFact>,
    prefixes: EvaluationPrefixes,
    semantic_roots: OnceCell<BTreeSet<SsaVar>>,
    ssa_values: Option<&'ir SsaValueGraph>,
    excluded_phis: BTreeSet<SsaVar>,
    copies: SsaCopyFlow,
    retained_phi_inputs: BTreeSet<SsaVar>,
    retained_values: BTreeSet<SsaVar>,
    effects: Vec<EffectFact>,
    block_domains: foldhash::HashMap<crate::ir::BlockId, ControlDomain>,
    predicate_tests: PredicateTests,
    predicate_uses: BTreeMap<crate::ir::InstructionId, BTreeSet<PredicateUseFact>>,
    canonical: Option<&'ir BTreeMap<SsaVar, InsnArg>>,
    identity_statements: Vec<(crate::ir::SemanticSiteId, usize)>,
    semantic_flow: Option<ScheduledFlow<'ir>>,
}

impl<'ir> ValueFlowGraph<'ir> {
    pub(super) fn build(
        root: &'ir SemanticNode,
        values: &'ir SsaValueGraph,
        canonical: &'ir BTreeMap<SsaVar, InsnArg>,
    ) -> Result<Self, ValueRecoveryError> {
        Self::build_ssa(
            root,
            values,
            canonical,
            Some(ScheduledFlow::new(root, None)),
            None,
            ControlSymbolClosure::analyze(root),
        )
    }

    pub(super) fn plan_ssa(
        root: &'ir SemanticNode,
        values: &'ir SsaValueGraph,
        canonical: &'ir BTreeMap<SsaVar, InsnArg>,
        cache: &'ir SourceFlowCache,
        schedule: impl FnOnce(Self) -> Result<ValuePlan, ValueRecoveryError>,
    ) -> Result<ValuePlan, ValueRecoveryError> {
        let mut symbols = ControlSymbolClosure {
            ssa_inputs: Some((values, canonical)),
            plan_required: !values.phis().is_empty() || !values.copies().is_empty(),
            ..Default::default()
        };
        symbols.visit_node(root);
        if !symbols.plan_required {
            // A CFG constant can outlive all its semantic uses. It is still
            // part of the SSA result consumed by type and variable recovery.
            let constants = canonical
                .iter()
                .filter(|(_, value)| Self::is_constant(value))
                .map(|(&key, value)| (key, value.clone()))
                .collect();
            return Ok(ValuePlan {
                actions: Vec::new(),
                constants,
                has_select: symbols.has_select,
            });
        }
        schedule(Self::build_ssa(
            root,
            values,
            canonical,
            Some(ScheduledFlow::new(root, Some(cache))),
            Some(cache),
            symbols,
        )?)
    }

    pub(super) fn build_gated(
        root: &'ir SemanticNode,
        values: &'ir SsaValueGraph,
        canonical: &'ir BTreeMap<SsaVar, InsnArg>,
        cache: &SourceFlowCache,
    ) -> Result<Self, ValueRecoveryError> {
        Self::build_ssa(
            root,
            values,
            canonical,
            None,
            Some(cache),
            ControlSymbolClosure::analyze(root),
        )
    }

    fn build_ssa(
        root: &'ir SemanticNode,
        values: &'ir SsaValueGraph,
        canonical: &'ir BTreeMap<SsaVar, InsnArg>,
        semantic_flow: Option<ScheduledFlow<'ir>>,
        cache: Option<&SourceFlowCache>,
        symbols: ControlSymbolClosure<'_>,
    ) -> Result<Self, ValueRecoveryError> {
        let logic = DomainLogic::ordered(symbols.tests.variables());
        let graph = Self {
            identity: ValueIdentity::Ssa,
            logic,
            has_select: symbols.has_select,
            has_predicate_uses: false,
            scopes: LexicalScopes::default(),
            definitions: OccurrenceRows::ssa(values),
            definitions_without_sites: BTreeSet::new(),
            bindings: BTreeSet::new(),
            uses: OccurrenceRows::ssa(values),
            prefixes: EvaluationPrefixes::default(),
            semantic_roots: OnceCell::new(),
            ssa_values: Some(values),
            excluded_phis: BTreeSet::new(),
            copies: SsaCopyFlow::default(),
            retained_phi_inputs: BTreeSet::new(),
            retained_values: BTreeSet::new(),
            effects: Vec::new(),
            block_domains: Default::default(),
            predicate_tests: symbols.tests,
            predicate_uses: BTreeMap::new(),
            canonical: Some(canonical),
            identity_statements: Vec::new(),
            semantic_flow,
        };
        let mut graph = FlowCollector::new(graph).collect(root, cache)?;
        graph.copies = SsaCopyFlow::analyze(values, &graph.definitions);
        graph.validate()?;
        Ok(graph)
    }

    pub(super) fn plan_source(
        root: &'ir SemanticNode,
        bindings: &BTreeSet<SsaVar>,
        cache: &'ir SourceFlowCache,
        mode: RecoveryMode,
    ) -> Result<ValuePlan, ValueRecoveryError> {
        let symbols = ControlSymbolClosure::for_source(root);
        // Source graphs have no CFG copies, Phis or canonical SSA constants.
        // With no statement defining a source identity the planner has no
        // action, including no identity move to eliminate. If evaluation can
        // fail, the collector still owns the original error and its ordering.
        if !symbols.plan_required {
            return Ok(ValuePlan {
                has_select: symbols.has_select,
                ..Default::default()
            });
        }
        Self::build_allocated(root, bindings, Some(cache), symbols)?.schedule(mode)
    }

    #[cfg(test)]
    fn build_source(
        root: &'ir SemanticNode,
        bindings: &BTreeSet<SsaVar>,
    ) -> Result<Self, ValueRecoveryError> {
        Self::build_allocated(root, bindings, None, ControlSymbolClosure::analyze(root))
    }

    fn build_allocated(
        root: &'ir SemanticNode,
        bindings: &BTreeSet<SsaVar>,
        cache: Option<&'ir SourceFlowCache>,
        symbols: ControlSymbolClosure<'_>,
    ) -> Result<Self, ValueRecoveryError> {
        let logic = DomainLogic::ordered(symbols.tests.variables());
        let graph = Self {
            identity: ValueIdentity::Source,
            logic,
            has_select: symbols.has_select,
            has_predicate_uses: false,
            scopes: LexicalScopes::default(),
            definitions: OccurrenceRows::default(),
            definitions_without_sites: BTreeSet::new(),
            bindings: bindings.clone(),
            uses: OccurrenceRows::default(),
            prefixes: EvaluationPrefixes::default(),
            semantic_roots: OnceCell::new(),
            ssa_values: None,
            excluded_phis: BTreeSet::new(),
            copies: SsaCopyFlow::default(),
            retained_phi_inputs: BTreeSet::new(),
            retained_values: BTreeSet::new(),
            effects: Vec::new(),
            block_domains: Default::default(),
            predicate_tests: symbols.tests,
            predicate_uses: BTreeMap::new(),
            canonical: None,
            identity_statements: Vec::new(),
            semantic_flow: Some(ScheduledFlow::new(root, cache)),
        };
        FlowCollector::new(graph).collect(root, cache)
    }

    pub(super) fn attach_semantic_flow(
        &mut self,
        root: &'ir SemanticNode,
        cache: &'ir SourceFlowCache,
    ) {
        self.semantic_flow = Some(ScheduledFlow::new(root, Some(cache)));
    }

    pub(super) fn proof_budget_exhausted(&self) -> bool {
        self.logic.proof_budget_exhausted()
    }

    fn has_plan_inputs(&self) -> bool {
        !self.definitions.is_empty()
            || self.phis().next().is_some()
            || !self.copies.omitted.is_empty()
            || self.canonical.is_some_and(|values| !values.is_empty())
            || !self.identity_statements.is_empty()
    }

    pub(super) fn schedule(&self, mode: RecoveryMode) -> Result<ValuePlan, ValueRecoveryError> {
        if !self.has_plan_inputs() {
            return Ok(ValuePlan {
                has_select: self.has_select,
                ..Default::default()
            });
        }
        let planner = ValuePlanner::new(self, mode)?;
        planner.schedule()
    }

    pub(super) fn recover_gated_phis(
        &self,
        values: &SsaValueGraph,
        control: &GatedControlFlow,
    ) -> Result<gated::GatedPhiRecovery, ValueRecoveryError> {
        GatedPhiAnalysis::new(self, values, control).analyze()
    }

    fn phis(&self) -> impl Iterator<Item = &PhiMerge> {
        self.ssa_values
            .into_iter()
            .flat_map(SsaValueGraph::phis)
            .filter(move |phi| !self.excluded_phis.contains(&phi.result))
    }

    fn phi(&self, result: SsaVar) -> Option<&PhiMerge> {
        if self.excluded_phis.contains(&result) {
            return None;
        }
        self.ssa_values?.phi(result)
    }

    pub(super) fn exclude_phis(&mut self, recovered: &BTreeSet<SsaVar>) {
        if recovered.is_empty() {
            return;
        }
        let Some(values) = self.ssa_values else {
            return;
        };
        let excluded = recovered
            .iter()
            .copied()
            .filter(|&result| self.phi(result).is_some())
            .collect::<BTreeSet<_>>();
        let mut pending = excluded
            .iter()
            .filter(|result| {
                self.semantic_use_roots().contains(result) && !self.definitions.contains_key(result)
            })
            .copied()
            .collect::<Vec<_>>();
        let mut visited = BTreeSet::new();
        while let Some(result) = pending.pop() {
            if !visited.insert(result) {
                continue;
            }
            let Some(phi) = values.phi(result) else {
                continue;
            };
            for input in &phi.inputs {
                self.retained_phi_inputs.insert(input.value);
                let source = self.copies.omitted_source(input.value);
                self.retained_phi_inputs.insert(source);
                if excluded.contains(&input.value) {
                    pending.push(input.value);
                }
                if source != input.value && excluded.contains(&source) {
                    pending.push(source);
                }
            }
        }
        self.excluded_phis.extend(excluded);
    }

    pub(super) fn retain_values(&mut self, values: impl IntoIterator<Item = SsaVar>) {
        self.retained_values.extend(values);
    }

    fn is_retained(&self, value: SsaVar) -> bool {
        self.retained_values.contains(&value)
    }

    fn is_bound(&self, value: SsaVar) -> bool {
        self.bindings.contains(&value)
    }

    pub(super) fn identity(&self) -> ValueIdentity {
        self.identity
    }

    fn copy_argument(&self, value: SsaVar) -> InsnArg {
        let ty = self
            .ssa_values
            .and_then(|values| values.value(value))
            .and_then(|value| value.observed_types.iter().find(|ty| ty.is_known()))
            .cloned()
            .unwrap_or_else(crate::ir::ArgType::unknown);
        InsnArg::reg_ssa(value.reg_num, value.version, ty)
    }

    fn copy_source(&self, value: SsaVar) -> SsaVar {
        self.copies.source(value)
    }

    fn semantic_use_roots(&self) -> &BTreeSet<SsaVar> {
        self.semantic_roots.get_or_init(|| {
            self.uses
                .keys()
                .flat_map(|value| [*value, self.copy_source(*value)])
                .collect()
        })
    }

    fn key(&self, register: &crate::ir::RegisterArg) -> Option<SsaVar> {
        self.identity.key(register)
    }

    fn record_binding(&mut self, register: &crate::ir::RegisterArg) {
        if let Some(value) = self.key(register) {
            self.bindings.insert(value);
        }
    }

    fn is_identity_move(&self, instruction: &SemanticOperation) -> bool {
        if instruction.insn_type != InsnType::Move || instruction.operands().len() != 1 {
            return false;
        }
        let Some(result) = instruction
            .result
            .as_ref()
            .and_then(|result| self.key(result))
        else {
            return false;
        };
        Self::canonical_semantic_move_operand(&instruction.operands()[0])
            .as_register()
            .and_then(|source| self.key(source))
            == Some(result)
    }

    fn is_identity_definition(
        &self,
        result: &crate::ir::RegisterArg,
        value: &SemanticExpression,
    ) -> bool {
        let Some(result) = self.key(result) else {
            return false;
        };
        Self::canonical_semantic_move_operand(value)
            .as_register()
            .and_then(|source| self.key(source))
            == Some(result)
    }

    fn canonical_semantic_move_operand(mut value: &SemanticExpression) -> &SemanticExpression {
        loop {
            let SemanticExpression::Operation(operation) = value else {
                return value;
            };
            if operation.insn_type != InsnType::Move || operation.operands().len() != 1 {
                return value;
            }
            value = &operation.operands()[0];
        }
    }

    fn semantic_flow(&self) -> Option<&crate::ir::analysis::SemanticFlowGraph> {
        self.semantic_flow.as_ref().map(ScheduledFlow::get)
    }

    fn evaluation_prefix(&self, usage: &UseFact) -> &EffectSummary {
        self.prefixes.get(usage.evaluation_prefix)
    }

    fn movement_points(&self) -> impl Iterator<Item = crate::ir::analysis::SemanticFlowPoint> + '_ {
        self.definitions
            .values()
            .flatten()
            .filter_map(|definition| {
                definition
                    .site
                    .map(crate::ir::analysis::SemanticFlowPoint::after)
            })
            .chain(
                self.uses
                    .values()
                    .flatten()
                    .filter_map(|usage| usage.location.point()),
            )
            .chain(self.effects.iter().map(|effect| effect.point))
    }

    fn is_constant(value: &InsnArg) -> bool {
        matches!(value, InsnArg::Lit(_))
            || matches!(
                value,
                InsnArg::Wrapped(instruction)
                    if matches!(instruction.insn_type, InsnType::Const | InsnType::ConstStr)
            )
    }

    fn validate(&self) -> Result<(), ValueRecoveryError> {
        // The collector checked SSA membership when it created each row. Only
        // repeated definitions need an ordered pass to classify structural replicas.
        let unknown = self.definitions.first_unknown();
        if self.definitions.has_repeated() {
            for (value, definitions) in self.definitions.iter() {
                if definitions.len() > 1 && !Self::are_structural_replicas(definitions) {
                    return Err(ValueRecoveryError::DuplicateDefinition(*value));
                }
                if Some(*value) == unknown {
                    return Err(ValueRecoveryError::UnknownValue(*value));
                }
            }
        }
        if let Some(value) = unknown.or_else(|| self.uses.first_unknown()) {
            return Err(ValueRecoveryError::UnknownValue(value));
        }
        Ok(())
    }

    pub(super) fn are_structural_replicas(definitions: &[DefinitionFact]) -> bool {
        let Some(first) = definitions.first() else {
            return false;
        };
        let Some(origin) = first.origin else {
            return false;
        };
        definitions.iter().all(|definition| {
            definition.origin == Some(origin)
                && definition.instruction == first.instruction
                && definition.operation().map(|operation| operation.insn_type)
                    == first.operation().map(|operation| operation.insn_type)
        })
    }

    fn classes_by_last_definition(&self) -> Vec<DefinitionClass<'_>> {
        let mut classes = self
            .definitions
            .values()
            .filter_map(|definitions| {
                let last = definitions.last()?;
                Some(DefinitionClass {
                    definitions,
                    last_event: last.event,
                    bound: self.is_bound(last.key),
                })
            })
            .collect::<Vec<_>>();
        // Rows are inserted in first-event order, which also orders the last
        // event when each identity has a single defining statement.
        if self.definitions.has_repeated() {
            classes.sort_unstable_by_key(|class| class.last_event);
        }
        classes
    }

    fn canonical_values(&self, classes: &[DefinitionClass<'_>]) -> BTreeMap<SsaVar, InsnArg> {
        let mut values = self.canonical.cloned().unwrap_or_default();
        for (result, source) in self.copies.aliases() {
            values
                .entry(result)
                .or_insert_with(|| self.copy_argument(source));
        }
        loop {
            let mut changed = false;
            // Bound parameters and catch locals also have an incoming
            // definition outside their conditional statement assignments.
            for class in classes.iter().filter(|class| !class.bound) {
                let Some(value) = self.canonical_class_value(class.definitions, &values) else {
                    continue;
                };
                let key = class.definitions[0].key;
                let value_key = self.canonical_key(&value);
                if values.get(&key).and_then(|value| self.canonical_key(value)) != value_key {
                    values.insert(key, value);
                    changed = true;
                }
            }
            for phi in self.phis() {
                let Some(replacement) = self.trivial_phi_replacement(phi, &values) else {
                    continue;
                };
                let changed_value = values
                    .get(&phi.result)
                    .and_then(|value| self.canonical_key(value))
                    != self.canonical_key(&replacement);
                if changed_value {
                    values.insert(phi.result, replacement);
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        values
    }

    /// Computes the meet of every definition assigned to one value identity.
    /// Source allocation deliberately coalesces multiple SSA definitions into
    /// one local; a canonical value exists only when every reaching definition
    /// evaluates to the same lattice constant.
    fn canonical_class_value(
        &self,
        definitions: &[DefinitionFact],
        values: &BTreeMap<SsaVar, InsnArg>,
    ) -> Option<InsnArg> {
        let mut representative = None::<InsnArg>;
        for definition in definitions {
            if definition.edge_copy() {
                return None;
            }
            let value = self.resolve_canonical(self.canonical_value(definition, values)?, values);
            let key = self.canonical_key(&value)?;
            match &representative {
                None => representative = Some(value),
                Some(prior) if self.canonical_key(prior) == Some(key) => {}
                Some(_) => return None,
            }
        }
        representative
    }

    fn trivial_phi_replacement(
        &self,
        phi: &PhiMerge,
        values: &BTreeMap<SsaVar, InsnArg>,
    ) -> Option<InsnArg> {
        let self_key = CanonicalKey::Value(phi.result);
        let mut representative = None::<InsnArg>;
        for input in &phi.inputs {
            let value = self.resolve_canonical(
                InsnArg::reg_ssa(
                    input.value.reg_num,
                    input.value.version,
                    self.phi_value_type(input.value),
                ),
                values,
            );
            let key = self.canonical_key(&value)?;
            if key == self_key {
                continue;
            }
            match &representative {
                None => representative = Some(value),
                Some(prior) if self.canonical_key(prior) == Some(key) => {}
                Some(_) => return None,
            }
        }
        representative
    }

    fn canonical_value(
        &self,
        definition: &DefinitionFact,
        values: &BTreeMap<SsaVar, InsnArg>,
    ) -> Option<InsnArg> {
        if let ValueRef::Literal(literal) = definition.expression() {
            return Some(InsnArg::Lit(literal.clone()));
        }
        let expression = definition.operation()?;
        match expression.insn_type {
            InsnType::ConstStr => Some(InsnArg::wrap(
                expression.instruction_with_arguments(Vec::new()),
            )),
            InsnType::Move | InsnType::CheckCast => {
                let source = Self::semantic_leaf_argument(Self::canonical_semantic_move_operand(
                    expression.operands().first()?,
                ))?;
                if expression.insn_type == InsnType::CheckCast
                    && expression.conversion_type() != source.declared_type()
                {
                    return None;
                }
                match &source {
                    InsnArg::Lit(_) => Some(source),
                    InsnArg::Reg(register) => {
                        let key = self.key(register)?;
                        Some(values.get(&key).cloned().unwrap_or(source))
                    }
                    InsnArg::Wrapped(_) => None,
                }
            }
            kind if ConstantEvaluator::accepts(kind) => {
                let arguments = expression
                    .operands()
                    .iter()
                    .map(|operand| {
                        Self::semantic_leaf_argument(operand)
                            .map(|argument| self.resolve_canonical(argument, values))
                    })
                    .collect::<Option<Vec<_>>>()?;
                ConstantEvaluator::fold(
                    kind,
                    &expression.payload,
                    expression.result.as_ref(),
                    &arguments,
                )
            }
            _ => None,
        }
    }

    fn semantic_leaf_argument(expression: &SemanticExpression) -> Option<InsnArg> {
        match expression {
            SemanticExpression::Register(register) => Some(InsnArg::Reg(register.clone())),
            SemanticExpression::Literal(literal) => Some(InsnArg::Lit(literal.clone())),
            SemanticExpression::Operation(_) | SemanticExpression::Select { .. } => None,
        }
    }

    fn resolve_canonical(&self, value: InsnArg, values: &BTreeMap<SsaVar, InsnArg>) -> InsnArg {
        let mut visited = super::AliasPath::default();
        let mut current = &value;
        while let Some(variable) = current
            .as_register()
            .and_then(|register| self.key(register))
        {
            let Some(replacement) = values.get(&variable) else {
                break;
            };
            if !visited.insert(variable) {
                break;
            }
            current = replacement;
        }
        if std::ptr::eq(current, &value) {
            value
        } else {
            current.clone()
        }
    }

    fn phi_value_type(&self, value: SsaVar) -> crate::ir::ArgType {
        self.definitions
            .get(&value)
            .and_then(|definitions| definitions.first())
            .map(|definition| definition.result.ty.clone())
            .unwrap_or_else(crate::ir::ArgType::unknown)
    }

    fn canonical_key<'a>(&self, value: &'a InsnArg) -> Option<CanonicalKey<'a>> {
        match value {
            InsnArg::Lit(literal) => Some(CanonicalKey::Literal {
                value: literal.value,
                ty: &literal.ty,
            }),
            InsnArg::Reg(register) => self.key(register).map(CanonicalKey::Value),
            InsnArg::Wrapped(instruction) => match instruction.insn_type {
                InsnType::Const => {
                    let literal = instruction.args.first()?.as_literal()?;
                    Some(CanonicalKey::Literal {
                        value: literal.value,
                        ty: instruction
                            .result
                            .as_ref()
                            .map_or(&literal.ty, |result| &result.ty),
                    })
                }
                InsnType::ConstStr => Some(CanonicalKey::String(
                    instruction.payload.string_value.as_deref()?,
                )),
                _ => None,
            },
        }
    }

    fn required_phi_inputs(&self) -> BTreeSet<SsaVar> {
        let pending = self
            .phis()
            .filter(|phi| self.semantic_use_roots().contains(&phi.result))
            .map(|phi| phi.result)
            .collect::<Vec<_>>();
        required_phi_input_closure(
            |result| self.phi(result),
            &self.copies,
            pending,
            self.retained_phi_inputs.clone(),
        )
    }
}

fn required_phi_input_closure<'a>(
    phi_for: impl Fn(SsaVar) -> Option<&'a PhiMerge>,
    copies: &SsaCopyFlow,
    mut pending: Vec<SsaVar>,
    mut required: BTreeSet<SsaVar>,
) -> BTreeSet<SsaVar> {
    let mut visited = BTreeSet::new();
    while let Some(result) = pending.pop() {
        if !visited.insert(result) {
            continue;
        }
        let Some(phi) = phi_for(result) else {
            continue;
        };
        for input in &phi.inputs {
            required.insert(input.value);
            let source = copies.omitted_source(input.value);
            required.insert(source);
            if phi_for(input.value).is_some() {
                pending.push(input.value);
            }
            if source != input.value && phi_for(source).is_some() {
                pending.push(source);
            }
        }
    }
    required
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum CanonicalKey<'a> {
    Value(SsaVar),
    Literal {
        value: i64,
        ty: &'a crate::ir::ArgType,
    },
    String(&'a Utf16String),
}

struct DefinitionClass<'a> {
    definitions: &'a [DefinitionFact<'a>],
    last_event: usize,
    bound: bool,
}

/// CFG-ordered symbols seed the BDD. Gated and source selection also read
/// the last semantic test for each instruction. Ordinary SSA needs only IDs.
#[derive(Default)]
struct PredicateTests(BTreeMap<crate::ir::InstructionId, Option<SemanticOperation>>);

impl PredicateTests {
    fn variables(&self) -> impl ExactSizeIterator<Item = BoolVariable> + '_ {
        self.0.keys().copied().map(BoolVariable::Instruction)
    }

    fn get(&self, instruction: &crate::ir::InstructionId) -> Option<&SemanticOperation> {
        self.0.get(instruction)?.as_ref()
    }
}

#[derive(Default)]
struct ControlSymbolClosure<'a> {
    ssa_inputs: Option<(&'a SsaValueGraph, &'a BTreeMap<SsaVar, InsnArg>)>,
    has_select: bool,
    tests: PredicateTests,
    plan_required: bool,
}

impl ControlSymbolClosure<'_> {
    fn analyze(root: &SemanticNode) -> Self {
        // SSA scheduling also has CFG facts that are absent from this tree.
        let mut closure = Self {
            plan_required: true,
            ..Self::default()
        };
        closure.visit_node(root);
        closure
    }

    fn for_source(root: &SemanticNode) -> Self {
        let mut closure = Self::default();
        closure.visit_node(root);
        closure
    }
}

impl SemanticVisitor for ControlSymbolClosure<'_> {
    fn visit_expression(&mut self, expression: &SemanticExpression) {
        self.has_select |= matches!(expression, SemanticExpression::Select { .. });
        crate::ir::semantic::walk_expression(self, expression);
    }

    fn visit_register(&mut self, register: &crate::ir::RegisterArg) {
        if let Some((values, constants)) = self.ssa_inputs.filter(|_| !self.plan_required) {
            self.plan_required = SsaVar::from_reg(register).is_some_and(|key| {
                values.value(key).is_none()
                    || constants.get(&key).is_some_and(ValueFlowGraph::is_constant)
            });
        }
    }

    fn visit_statement(&mut self, statement: &crate::ir::SemanticStatement) {
        if !self.plan_required {
            let identity = if self.ssa_inputs.is_some() {
                ValueIdentity::Ssa
            } else {
                ValueIdentity::Source
            };
            self.plan_required = statement
                .result()
                .and_then(|result| identity.key(result))
                .is_some();
        }
        match &statement.kind {
            crate::ir::SemanticStatementKind::Instruction(op) => self.visit_operation(op),
            crate::ir::SemanticStatementKind::Definition { value, .. } => {
                self.visit_expression(value)
            }
        }
    }

    fn visit_operation(&mut self, operation: &SemanticOperation) {
        use crate::ir::semantic::OperandShape;
        if self.plan_required || self.ssa_inputs.is_none() {
            match operation.operand_shape() {
                OperandShape::Plain => return,
                OperandShape::InvalidEvaluation => {
                    self.plan_required = true;
                    return;
                }
                OperandShape::Select => {}
            }
        }
        crate::ir::semantic::walk_operation(self, operation);
    }

    fn enter_operation(&mut self, operation: &SemanticOperation) {
        if !self.plan_required && operation.evaluation_operands_iter().is_err() {
            self.plan_required = true;
        }
    }

    fn visit_predicate(&mut self, predicate: &SemanticPredicate) {
        let mut pending = Vec::new();
        let mut predicate = predicate;
        loop {
            match predicate {
                SemanticPredicate::Test(instruction) => {
                    let test = self.ssa_inputs.is_none().then(|| instruction.clone());
                    self.tests.0.insert(instruction.id, test);
                    self.plan_required |= self.tests.0.len() > 1;
                    self.visit_operation(instruction);
                }
                SemanticPredicate::Not(inner) => {
                    predicate = inner;
                    continue;
                }
                SemanticPredicate::And(terms) | SemanticPredicate::Or(terms) => {
                    if let Some((last, preceding)) = terms.split_last() {
                        pending.extend(preceding);
                        predicate = last;
                        continue;
                    }
                }
                SemanticPredicate::True | SemanticPredicate::False => {}
            }
            let Some(next) = pending.pop() else { break };
            predicate = next;
        }
    }
}

#[cfg(test)]
#[path = "../../../tests/internal/source_plan_inputs.rs"]
mod source_plan_inputs;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::analysis::PhiInput;
    use crate::ir::{
        BlockId, EdgeKind, InsnNode, InsnType, InstructionId, RegionId, SemanticCatch,
        SemanticExpression,
    };

    fn operation(
        id: usize,
        insn_type: InsnType,
        operands: Vec<SemanticExpression>,
    ) -> SemanticOperation {
        let mut instruction = InsnNode::new(insn_type, operands.len());
        instruction.id = InstructionId::new(id);
        SemanticOperation::from_parts(instruction, operands, None)
    }

    #[test]
    fn control_symbol_closure_includes_selects_inside_predicate_operands() {
        let nested = SemanticPredicate::Test(operation(21, InsnType::If, Vec::new()));
        let selected = SemanticExpression::select(
            nested,
            SemanticExpression::Operation(operation(22, InsnType::Const, Vec::new())),
            SemanticExpression::Operation(operation(23, InsnType::Const, Vec::new())),
        );
        let outer = SemanticPredicate::Test(operation(20, InsnType::If, vec![selected]));

        let mut closure = ControlSymbolClosure::default();
        closure.visit_predicate(&outer);

        assert_eq!(
            closure.tests.variables().collect::<BTreeSet<_>>(),
            BTreeSet::from([
                BoolVariable::Instruction(InstructionId::new(20)),
                BoolVariable::Instruction(InstructionId::new(21)),
            ])
        );
        assert_eq!(
            closure
                .tests
                .0
                .values()
                .flatten()
                .map(|test| test.id)
                .collect::<BTreeSet<_>>(),
            BTreeSet::from([InstructionId::new(20), InstructionId::new(21)])
        );
    }

    #[test]
    fn catch_parameters_are_lexical_value_bindings() {
        let mut exception =
            crate::ir::RegisterArg::new(0, crate::ir::ArgType::object("java/lang/Throwable"));
        exception.code_var = Some(7);
        let root = SemanticNode::Try {
            region: RegionId::new(1),
            body: Box::new(SemanticNode::Empty),
            catches: vec![SemanticCatch {
                region: RegionId::new(2),
                exception_types: vec![crate::ir::ArgType::object("java/lang/Exception")],
                exception_value: Some(exception),
                body: SemanticNode::Empty,
            }],
            finally: None,
        };

        let graph = ValueFlowGraph::build_source(&root, &BTreeSet::new()).unwrap();

        assert!(graph.is_bound(SsaVar::new(7, 0)));
    }

    #[test]
    fn method_inputs_are_lexical_value_bindings() {
        let binding = SsaVar::new(7, 0);
        let graph =
            ValueFlowGraph::build_source(&SemanticNode::Empty, &BTreeSet::from([binding])).unwrap();

        assert!(graph.is_bound(binding));
    }

    #[test]
    fn required_phi_inputs_retain_transitive_copy_sources() {
        let headers = SsaVar::new(12, 1);
        let handler_copy = SsaVar::new(13, 3);
        let null = SsaVar::new(13, 0);
        let merged = SsaVar::new(13, 4);
        let phis = vec![PhiMerge {
            block: BlockId::new(113),
            instruction: InstructionId::new(200),
            result: merged,
            inputs: vec![
                PhiInput {
                    predecessor: BlockId::new(108),
                    edge_kind: EdgeKind::Normal,
                    value: handler_copy,
                },
                PhiInput {
                    predecessor: BlockId::new(110),
                    edge_kind: EdgeKind::Normal,
                    value: null,
                },
            ],
        }];
        let mut copies = SsaCopyFlow::default();
        copies.sources.insert(handler_copy, headers);
        copies.omitted.insert(handler_copy);

        let values = crate::ir::analysis::SsaValueGraph::with_phis_for_test(phis);
        let required = required_phi_input_closure(
            |key| values.phi(key),
            &copies,
            vec![merged],
            BTreeSet::new(),
        );

        assert!(required.contains(&handler_copy));
        assert!(required.contains(&headers));
        assert!(required.contains(&null));
    }
}
