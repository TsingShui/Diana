use std::collections::{BTreeMap, BTreeSet};

use crate::ir::analysis::{SemanticFlowGraph, SemanticFlowPoint};
use crate::ir::{
    BlockId, InsnArg, InsnNode, InsnType, InstructionTree, InstructionVisitor, RegisterArg, CFG,
};
use crate::ir::{
    SemanticExpression, SemanticExpressionFacts, SemanticNode, SemanticPredicate, SemanticSiteId,
    SemanticStatement, SemanticStatementKind,
};

use super::{edge_arguments::ContractedEdgeArguments, SourceVariableError, SsaClasses, SsaVar};

#[derive(Default)]
struct BlockLiveness {
    definitions: BTreeSet<SsaVar>,
    input: BTreeSet<SsaVar>,
    output: BTreeSet<SsaVar>,
    phi_definitions: BTreeSet<SsaVar>,
}

impl BlockLiveness {
    /// True when this outgoing value also becomes newly live at entry.
    fn add_live_out(&mut self, value: SsaVar) -> bool {
        self.output.insert(value) && !self.definitions.contains(&value) && self.input.insert(value)
    }
}

pub(super) struct SsaLiveness {
    retained: BTreeSet<SsaVar>,
    blocks: BTreeMap<BlockId, BlockLiveness>,
    phi_uses: BTreeMap<(BlockId, BlockId, crate::ir::EdgeKind), BTreeSet<SsaVar>>,
}

impl SsaLiveness {
    pub(super) fn analyze(
        cfg: &CFG,
        retained: &BTreeSet<SsaVar>,
    ) -> Result<Self, SourceVariableError> {
        let mut blocks = BTreeMap::new();
        let mut phi_uses =
            BTreeMap::<(BlockId, BlockId, crate::ir::EdgeKind), BTreeSet<SsaVar>>::new();
        for (&block, body) in &cfg.blocks {
            let mut facts = BlockLiveness::default();
            for instruction in &body.insns {
                if instruction.insn_type == InsnType::Phi {
                    if let Some(result) = instruction.result.as_ref().and_then(SsaVar::from_reg) {
                        if retained.contains(&result) {
                            facts.definitions.insert(result);
                            facts.phi_definitions.insert(result);
                        }
                    }
                    for (&(predecessor, edge_kind), argument) in
                        instruction.payload.phi_edges.iter().zip(&instruction.args)
                    {
                        if let Some(value) = argument.as_register().and_then(SsaVar::from_reg) {
                            if retained.contains(&value) {
                                phi_uses
                                    .entry((predecessor, block, edge_kind))
                                    .or_default()
                                    .insert(value);
                            }
                        }
                    }
                    continue;
                }
                for usage in InstructionUses::collect(instruction)
                    .into_iter()
                    .filter(|usage| retained.contains(usage))
                {
                    if !facts.definitions.contains(&usage) {
                        facts.input.insert(usage);
                    }
                }
                if let Some(result) = instruction.result.as_ref().and_then(SsaVar::from_reg) {
                    if retained.contains(&result) {
                        facts.definitions.insert(result);
                    }
                }
            }
            blocks.insert(block, facts);
        }

        let mut pending = Vec::new();
        for (&source, facts) in blocks.iter_mut().rev() {
            pending.extend(facts.input.iter().map(|&value| (source, value)));
            for &(target, kind) in cfg.successors_with_kind(source) {
                if !cfg.blocks.contains_key(&target) {
                    return Err(SourceVariableError::MissingBlock(target));
                }
                if let Some(arguments) = phi_uses.get(&(source, target, kind)) {
                    for &value in arguments {
                        if facts.add_live_out(value) {
                            pending.push((source, value));
                        }
                    }
                }
            }
        }
        // The input/output equations are monotone. Propagate each new entry
        // fact backward once. Phi operands above belong to their own edge,
        // and phi definitions at a successor kill the incoming value.
        while let Some((target, value)) = pending.pop() {
            if blocks[&target].phi_definitions.contains(&value) {
                continue;
            }
            for &(source, _) in cfg.predecessors_with_kind(target) {
                if let Some(facts) = blocks.get_mut(&source) {
                    if facts.add_live_out(value) {
                        pending.push((source, value));
                    }
                }
            }
        }
        // Transfer equations are complete. Keep the live sets in their
        // original block table and materialize no per-edge copies.
        for facts in blocks.values_mut() {
            facts.definitions.clear();
        }
        Ok(Self {
            retained: retained.clone(),
            blocks,
            phi_uses,
        })
    }

    /// Borrow the live names on a given CFG successor edge. Both consumers
    /// already walk actual edges. A phi operand is included once even if it
    /// also flows through the target block.
    pub(super) fn edge_live(
        &self,
        source: BlockId,
        target: BlockId,
        kind: crate::ir::EdgeKind,
    ) -> Result<impl Iterator<Item = &SsaVar>, SourceVariableError> {
        let facts = self
            .blocks
            .get(&target)
            .ok_or(SourceVariableError::MissingBlock(target))?;
        let operands = self
            .phi_uses
            .get(&(source, target, kind))
            .into_iter()
            .flatten();
        Ok(facts.input.difference(&facts.phi_definitions).chain(
            operands.filter(|value| {
                !facts.input.contains(value) || facts.phi_definitions.contains(value)
            }),
        ))
    }

    pub(super) fn normal_continuation_live(
        &self,
        cfg: &CFG,
        predecessor: BlockId,
    ) -> Result<BTreeSet<SsaVar>, SourceVariableError> {
        let mut live = BTreeSet::new();
        for &(successor, kind) in cfg.successors_with_kind(predecessor) {
            if kind == crate::ir::EdgeKind::Exception {
                continue;
            }
            live.extend(self.edge_live(predecessor, successor, kind)?.copied());
        }
        if let Some(result) = cfg.block(predecessor).and_then(|body| {
            body.insns.last().and_then(|instruction| {
                instruction
                    .can_throw()
                    .then(|| instruction.result.as_ref().and_then(SsaVar::from_reg))
                    .flatten()
            })
        }) {
            live.remove(&result);
        }
        Ok(live)
    }
}

type Edges = foldhash::HashMap<SsaVar, foldhash::HashSet<SsaVar>>;

#[derive(Default)]
pub(super) struct InterferenceGraph {
    hard_edges: Edges,
    semantic_edges: Edges,
}

impl InterferenceGraph {
    pub(super) fn build(
        cfg: &CFG,
        semantic: &SemanticNode,
        liveness: &SsaLiveness,
        required_phis: &BTreeSet<SsaVar>,
        flow: impl FnOnce() -> std::sync::Arc<SemanticFlowGraph>,
    ) -> Result<Self, SourceVariableError> {
        let mut graph = Self::default();
        if let Some(facts) = liveness.blocks.get(&cfg.entry) {
            graph.add_clique(&facts.input);
        }
        for (&block, body) in &cfg.blocks {
            let facts = liveness
                .blocks
                .get(&block)
                .ok_or(SourceVariableError::MissingBlock(block))?;
            let mut live = facts.output.clone();
            for instruction in body.insns.iter().rev() {
                let definition = instruction
                    .result
                    .as_ref()
                    .and_then(SsaVar::from_reg)
                    .filter(|value| liveness.retained.contains(value));
                let mut uses = if instruction.insn_type == InsnType::Phi {
                    Default::default()
                } else {
                    InstructionUses::collect(instruction)
                };
                uses.retain(|value| liveness.retained.contains(value));
                for (index, &left) in uses.iter().enumerate() {
                    for &right in &uses[index + 1..] {
                        graph.add(left, right);
                    }
                }
                if let Some(definition) = definition {
                    let move_source = (instruction.insn_type == InsnType::Move)
                        .then(|| instruction.args.first())
                        .flatten()
                        .and_then(InsnArg::as_register)
                        .and_then(SsaVar::from_reg);
                    for other in live
                        .iter()
                        .copied()
                        .filter(|value| *value != definition && Some(*value) != move_source)
                    {
                        graph.add(definition, other);
                    }
                    live.remove(&definition);
                }
                live.extend(uses);
            }
            if facts.input != live {
                return Err(SourceVariableError::LivenessMismatch(block));
            }
        }
        let semantic_liveness =
            SemanticLiveness::analyze(semantic, &liveness.retained, &mut graph, flow)?;
        semantic_liveness.add_recovered_phi_interference(cfg, required_phis, &mut graph);
        Ok(graph)
    }

    pub(super) fn add_exceptional_edge_interference(
        &mut self,
        cfg: &CFG,
        liveness: &SsaLiveness,
        arguments: &ContractedEdgeArguments<'_>,
    ) -> Result<(), SourceVariableError> {
        for (&successor, block) in &cfg.blocks {
            for phi in block
                .insns
                .iter()
                .filter(|instruction| instruction.insn_type == InsnType::Phi)
            {
                let Some(destination) = phi
                    .result
                    .as_ref()
                    .and_then(SsaVar::from_reg)
                    .filter(|value| liveness.retained.contains(value))
                else {
                    continue;
                };
                for argument in arguments.resolve_all(successor, phi)? {
                    if argument.edge_kind != crate::ir::EdgeKind::Exception {
                        continue;
                    }
                    let source = argument
                        .value
                        .explicit()
                        .and_then(InsnArg::as_register)
                        .and_then(SsaVar::from_reg);
                    for live in liveness.normal_continuation_live(cfg, argument.predecessor)? {
                        if live != destination && Some(live) != source {
                            self.add(destination, live);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn add(&mut self, left: SsaVar, right: SsaVar) {
        self.hard_edges.entry(left).or_default().insert(right);
        self.hard_edges.entry(right).or_default().insert(left);
    }

    fn add_semantic(&mut self, left: SsaVar, right: SsaVar) {
        self.semantic_edges.entry(left).or_default().insert(right);
        self.semantic_edges.entry(right).or_default().insert(left);
    }

    fn add_clique(&mut self, values: &BTreeSet<SsaVar>) {
        let mut remaining = values.iter().copied();
        while let Some(left) = remaining.next() {
            for right in remaining.clone() {
                self.add(left, right);
            }
        }
    }

    fn add_semantic_clique(&mut self, values: &BTreeSet<SsaVar>) {
        let mut remaining = values.iter().copied();
        while let Some(left) = remaining.next() {
            for right in remaining.clone() {
                self.add_semantic(left, right);
            }
        }
    }
}

#[derive(Default)]
struct SemanticSiteFacts {
    uses: BTreeSet<SsaVar>,
    definitions: BTreeSet<SsaVar>,
    blocks: BTreeSet<BlockId>,
}

struct SemanticLiveness {
    sites: BTreeMap<SemanticSiteId, SemanticSiteFacts>,
}

enum SemanticPointDefinitions<'a> {
    Before(&'a BTreeSet<SsaVar>),
    After(&'a BTreeSet<SsaVar>),
    None,
}

impl SemanticLiveness {
    fn analyze(
        root: &SemanticNode,
        retained: &BTreeSet<SsaVar>,
        interference: &mut InterferenceGraph,
        flow: impl FnOnce() -> std::sync::Arc<SemanticFlowGraph>,
    ) -> Result<Self, SourceVariableError> {
        let sites = SemanticSiteCollector::new(retained).collect(root)?;
        let flow = flow();
        if !flow.is_complete() {
            return Err(SourceVariableError::IncompleteSemanticFlow);
        }
        let points = flow.index();
        let mut definitions = Vec::with_capacity(points.len());
        let mut uses = BTreeMap::<SsaVar, Vec<usize>>::new();
        for index in 0..points.len() {
            let point = points.point(index);
            definitions.push(match point {
                SemanticFlowPoint::Before(site) => {
                    let facts = sites
                        .get(&site)
                        .ok_or(SourceVariableError::MissingSemanticSite("flow point"))?;
                    for value in &facts.uses {
                        uses.entry(*value).or_default().push(index);
                    }
                    SemanticPointDefinitions::Before(&facts.definitions)
                }
                SemanticFlowPoint::After(site) => sites
                    .get(&site)
                    .map_or(SemanticPointDefinitions::None, |facts| {
                        SemanticPointDefinitions::After(&facts.definitions)
                    }),
                SemanticFlowPoint::Synthetic(_) => SemanticPointDefinitions::None,
            });
        }
        // Each use makes one SSA value live backward until its definition. All
        // uses seed the search, including a use at a site that also defines
        // the value. A live After point contributes exactly the interference
        // edges with the definitions at that occurrence.
        let mut visited = vec![usize::MAX; definitions.len()];
        let mut pending = Vec::new();
        for (stamp, (value, starts)) in uses.into_iter().enumerate() {
            for index in starts {
                visited[index] = stamp;
                pending.push(index);
            }
            while let Some(index) = pending.pop() {
                if let SemanticPointDefinitions::After(defs) = definitions[index] {
                    for &definition in defs {
                        if definition != value {
                            interference.add_semantic(definition, value);
                        }
                    }
                }
                for &predecessor in points.predecessors(index) {
                    if visited[predecessor] == stamp {
                        continue;
                    }
                    visited[predecessor] = stamp;
                    if !matches!(definitions[predecessor], SemanticPointDefinitions::Before(kills) if kills.contains(&value))
                    {
                        pending.push(predecessor);
                    }
                }
            }
        }
        for facts in sites.values() {
            interference.add_semantic_clique(&facts.uses);
        }
        Ok(Self { sites })
    }

    fn add_recovered_phi_interference(
        &self,
        cfg: &CFG,
        required_phis: &BTreeSet<SsaVar>,
        graph: &mut InterferenceGraph,
    ) {
        if required_phis.is_empty() {
            return;
        }
        let mut uses_by_block = BTreeMap::<BlockId, BTreeSet<SsaVar>>::new();
        for facts in self.sites.values() {
            for block in &facts.blocks {
                uses_by_block
                    .entry(*block)
                    .or_default()
                    .extend(facts.uses.iter().copied());
            }
        }

        for (block, uses) in uses_by_block {
            let Some(body) = cfg.block(block) else {
                continue;
            };
            for phi in body
                .insns
                .iter()
                .filter(|instruction| instruction.insn_type == InsnType::Phi)
            {
                let Some(result) = phi.result.as_ref().and_then(SsaVar::from_reg) else {
                    continue;
                };
                if !required_phis.contains(&result) {
                    continue;
                }
                let inputs = phi
                    .args
                    .iter()
                    .filter_map(InsnArg::as_register)
                    .filter_map(SsaVar::from_reg)
                    .collect::<BTreeSet<_>>();
                // Value recovery can synthesize a use of a pre-Phi SSA value
                // in a statement originating in the Phi block. The CFG never
                // contained that use, so physical liveness alone would allow
                // the Phi result to share its source variable and its edge
                // copy would overwrite the old value before the recovered
                // expression reads it.
                for usage in uses
                    .intersection(&inputs)
                    .copied()
                    .filter(|usage| *usage != result)
                {
                    graph.add(result, usage);
                }
            }
        }
    }
}

struct SemanticSiteCollector<'a> {
    retained: &'a BTreeSet<SsaVar>,
    sites: BTreeMap<SemanticSiteId, SemanticSiteFacts>,
}

impl<'a> SemanticSiteCollector<'a> {
    fn new(retained: &'a BTreeSet<SsaVar>) -> Self {
        Self {
            retained,
            sites: BTreeMap::new(),
        }
    }

    fn collect(
        mut self,
        root: &SemanticNode,
    ) -> Result<BTreeMap<SemanticSiteId, SemanticSiteFacts>, SourceVariableError> {
        let mut pending = vec![root];
        while let Some(node) = pending.pop() {
            match node {
                SemanticNode::Empty => {}
                SemanticNode::BasicBlock(block) => {
                    for statement in &block.statements {
                        self.record_statement(statement)?;
                    }
                }
                SemanticNode::Sequence(children) => pending.extend(children),
                SemanticNode::If {
                    condition,
                    then_node,
                    else_node,
                } => {
                    self.record_predicate(condition.site, condition)?;
                    pending.push(then_node);
                    pending.extend(else_node.as_deref());
                }
                SemanticNode::Loop { test, body, .. } => {
                    self.record_predicate(test.condition.site, &test.condition)?;
                    pending.push(&test.setup);
                    pending.push(body);
                }
                SemanticNode::For {
                    init,
                    condition,
                    update,
                    body,
                    ..
                } => {
                    self.record_statement(init)?;
                    self.record_predicate(condition.site, condition)?;
                    self.record_statement(update)?;
                    pending.push(body);
                }
                SemanticNode::ForEach {
                    variable,
                    iterable,
                    body,
                    ..
                } => {
                    self.record_expression(iterable.site, iterable)?;
                    if let (Some(site), Some(variable)) =
                        (iterable.site, SsaVar::from_reg(variable))
                    {
                        self.site(site).definitions.insert(variable);
                    }
                    pending.push(body);
                }
                SemanticNode::Switch {
                    selector, cases, ..
                } => {
                    self.record_expression(selector.site, selector)?;
                    pending.extend(cases.iter().map(|case| &case.body));
                }
                SemanticNode::Try {
                    body,
                    catches,
                    finally,
                    ..
                } => {
                    pending.push(body);
                    pending.extend(catches.iter().map(|catch| &catch.body));
                    if let Some(finally) = finally {
                        pending.push(&finally.body);
                    }
                }
                SemanticNode::Synchronized { lock, body, .. } => {
                    self.record_expression(lock.site, lock)?;
                    pending.push(body);
                }
                SemanticNode::Label { body, .. } => pending.push(body),
                SemanticNode::Leave(leave) => {
                    let site = leave
                        .site
                        .ok_or(SourceVariableError::MissingSemanticSite("leave"))?;
                    self.site(site);
                    if let Some(condition) = &leave.condition {
                        self.add_uses(site, Self::predicate_uses(condition));
                    }
                    if let Some(value) = leave.value() {
                        self.add_uses(site, Self::expression_uses(value));
                    }
                }
            }
        }
        Ok(self.sites)
    }

    fn record_statement(
        &mut self,
        statement: &SemanticStatement,
    ) -> Result<(), SourceVariableError> {
        let site = statement
            .site
            .ok_or(SourceVariableError::MissingSemanticSite("statement"))?;
        self.site(site);
        if let Some(origin) = &statement.origin {
            self.site(site).blocks.insert(origin.block);
        }
        match &statement.kind {
            SemanticStatementKind::Instruction(operation) => {
                self.add_uses(
                    site,
                    SemanticExpressionFacts::of_operation(operation).used_ssa_variables(),
                );
                if let Some(result) = operation.result.as_ref().and_then(SsaVar::from_reg) {
                    self.site(site).definitions.insert(result);
                }
            }
            SemanticStatementKind::Definition { result, value, .. } => {
                self.add_uses(site, Self::expression_uses(value));
                if let Some(result) = SsaVar::from_reg(result) {
                    self.site(site).definitions.insert(result);
                }
            }
        }
        Ok(())
    }

    fn record_predicate(
        &mut self,
        site: Option<SemanticSiteId>,
        predicate: &SemanticPredicate,
    ) -> Result<(), SourceVariableError> {
        let site = site.ok_or(SourceVariableError::MissingSemanticSite("predicate"))?;
        self.site(site);
        self.add_uses(site, Self::predicate_uses(predicate));
        Ok(())
    }

    fn record_expression(
        &mut self,
        site: Option<SemanticSiteId>,
        expression: &SemanticExpression,
    ) -> Result<(), SourceVariableError> {
        let site = site.ok_or(SourceVariableError::MissingSemanticSite("expression"))?;
        self.site(site);
        self.add_uses(site, Self::expression_uses(expression));
        Ok(())
    }

    fn predicate_uses(predicate: &SemanticPredicate) -> BTreeSet<SsaVar> {
        SemanticExpressionFacts::of_predicate(predicate)
            .used_ssa_variables()
            .collect()
    }

    fn expression_uses(expression: &SemanticExpression) -> BTreeSet<SsaVar> {
        SemanticExpressionFacts::of_expression(expression)
            .used_ssa_variables()
            .collect()
    }

    fn add_uses(&mut self, site: SemanticSiteId, uses: impl IntoIterator<Item = SsaVar>) {
        let retained = self.retained;
        self.site(site)
            .uses
            .extend(uses.into_iter().filter(|value| retained.contains(value)));
    }

    fn site(&mut self, site: SemanticSiteId) -> &mut SemanticSiteFacts {
        self.sites.entry(site).or_default()
    }
}

pub(super) struct ClassInterference {
    hard_edges: Edges,
    semantic_edges: Edges,
}

impl ClassInterference {
    /// The allocator creates one class per SSA name. The original symmetric
    /// edges therefore already refer to class roots. Keep names introduced by
    /// the structured body, including those without a CFG declaration.
    pub(super) fn from_singletons(
        interference: InterferenceGraph,
        classes: &mut SsaClasses,
    ) -> Self {
        for &value in interference
            .hard_edges
            .keys()
            .chain(interference.semantic_edges.keys())
        {
            classes.root(value);
        }
        Self {
            hard_edges: interference.hard_edges,
            semantic_edges: interference.semantic_edges,
        }
    }

    pub(super) fn interferes(&self, left: SsaVar, right: SsaVar) -> bool {
        self.hard_interferes(left, right)
            || self
                .semantic_edges
                .get(&left)
                .is_some_and(|neighbors| neighbors.contains(&right))
    }

    pub(super) fn hard_interferes(&self, left: SsaVar, right: SsaVar) -> bool {
        self.hard_edges
            .get(&left)
            .is_some_and(|neighbors| neighbors.contains(&right))
    }

    pub(super) fn merge(&mut self, root: SsaVar, absorbed: SsaVar) {
        Self::merge_edges(&mut self.hard_edges, root, absorbed);
        Self::merge_edges(&mut self.semantic_edges, root, absorbed);
    }

    fn merge_edges(edges: &mut Edges, root: SsaVar, absorbed: SsaVar) {
        let Some(mut neighbors) = edges.remove(&absorbed) else {
            return;
        };
        neighbors.remove(&root);
        neighbors.remove(&absorbed);
        for neighbor in &neighbors {
            let adjacent = edges.entry(*neighbor).or_default();
            adjacent.remove(&absorbed);
            adjacent.insert(root);
        }
        let adjacent = edges.entry(root).or_default();
        adjacent.remove(&absorbed);
        adjacent.extend(neighbors);
    }
}

#[derive(Default)]
pub(super) struct InstructionUses {
    values: smallvec::SmallVec<[SsaVar; 4]>,
}

impl InstructionUses {
    pub(super) fn collect(instruction: &InsnNode) -> smallvec::SmallVec<[SsaVar; 4]> {
        let mut uses = Self::default();
        if instruction.insn_type == InsnType::Constructor {
            for argument in instruction.args.get(1..).unwrap_or_default() {
                InstructionTree::visit_arg(argument, &mut uses);
            }
            if let Some(target) = &instruction.payload.compound_target {
                InstructionTree::visit_arg(target, &mut uses);
            }
        } else {
            InstructionTree::visit_args(instruction, &mut uses);
        }
        uses.values.sort_unstable();
        uses.values.dedup();
        uses.values
    }
}

impl InstructionVisitor for InstructionUses {
    fn visit_register(&mut self, register: &RegisterArg) {
        self.values.extend(SsaVar::from_reg(register));
    }
}

#[cfg(test)]
mod tests {
    use super::{InterferenceGraph, SsaLiveness};
    use crate::ir::{analysis::SsaVar, ArgType, Block, IfOp, InsnArg, InsnNode, RegisterArg, CFG};
    use std::collections::BTreeSet;

    #[test]
    fn simultaneous_operands_interfere_without_local_definitions() {
        let left = SsaVar::new(0, 0);
        let right = SsaVar::new(1, 0);
        let mut block = Block::new(0u32);
        block.push(InsnNode::if_cmp(
            IfOp::Ne,
            InsnArg::reg_ssa(0, 0, ArgType::INT),
            InsnArg::reg_ssa(1, 0, ArgType::INT),
            0,
        ));
        let mut cfg = CFG::new("simultaneous-inputs");
        cfg.add_block(block);

        let retained = BTreeSet::from([left, right]);
        let liveness = SsaLiveness::analyze(&cfg, &retained).unwrap();
        let interference = InterferenceGraph::build(
            &cfg,
            &crate::ir::SemanticNode::Empty,
            &liveness,
            &BTreeSet::new(),
            || {
                std::sync::Arc::new(crate::ir::analysis::SemanticFlowGraph::analyze(
                    &crate::ir::SemanticNode::Empty,
                ))
            },
        )
        .unwrap();

        assert!(interference
            .hard_edges
            .get(&left)
            .is_some_and(|neighbors| neighbors.contains(&right)));
    }

    #[test]
    fn recovered_same_block_use_interferes_with_required_phi_result() {
        use crate::ir::{
            BlockId, InstructionId, SemanticBlock, SemanticNode, SemanticSiteNumbering,
            SemanticStatement, StatementOrigin,
        };

        let predecessor = BlockId::new(0);
        let join = BlockId::new(1);
        let old = RegisterArg::new_ssa(0, 0, ArgType::INT);
        let unrelated = RegisterArg::new_ssa(2, 0, ArgType::INT);
        let phi_result = RegisterArg::new_ssa(0, 1, ArgType::INT);
        let old_value = SsaVar::from_reg(&old).unwrap();
        let unrelated_value = SsaVar::from_reg(&unrelated).unwrap();
        let phi_value = SsaVar::from_reg(&phi_result).unwrap();

        let mut predecessor_block = Block::new(predecessor);
        predecessor_block.push(InsnNode::const_value(old.clone(), 0));
        predecessor_block.push(InsnNode::const_value(unrelated.clone(), 1));
        let mut join_block = Block::new(join);
        join_block.push(InsnNode::phi(
            phi_result,
            vec![(predecessor.raw(), InsnArg::Reg(old.clone()))],
        ));
        let mut cfg = CFG::new("recovered-phi-use");
        cfg.entry = predecessor;
        cfg.add_block(predecessor_block);
        cfg.add_block(join_block);
        cfg.add_edge(predecessor, join, crate::ir::EdgeKind::Normal);

        let mut statement = SemanticStatement::instruction(InsnNode::mov(
            RegisterArg::new_ssa(1, 0, ArgType::INT),
            InsnArg::Reg(old),
        ))
        .unwrap();
        statement.origin = Some(StatementOrigin {
            block: join,
            instruction: InstructionId::new(0),
        });
        let mut unrelated_statement = SemanticStatement::instruction(InsnNode::mov(
            RegisterArg::new_ssa(3, 0, ArgType::INT),
            InsnArg::Reg(unrelated),
        ))
        .unwrap();
        unrelated_statement.origin = Some(StatementOrigin {
            block: join,
            instruction: InstructionId::new(0),
        });
        let mut semantic = SemanticNode::BasicBlock(SemanticBlock {
            id: join,
            statements: vec![statement, unrelated_statement],
        });
        SemanticSiteNumbering::assign(&mut semantic).unwrap();

        let retained = BTreeSet::from([old_value, unrelated_value, phi_value]);
        let liveness = SsaLiveness::analyze(&cfg, &retained).unwrap();
        let interference = InterferenceGraph::build(
            &cfg,
            &semantic,
            &liveness,
            &BTreeSet::from([phi_value]),
            || std::sync::Arc::new(crate::ir::analysis::SemanticFlowGraph::analyze(&semantic)),
        )
        .unwrap();

        assert!(interference
            .hard_edges
            .get(&phi_value)
            .is_some_and(|neighbors| neighbors.contains(&old_value)));
        assert!(!interference
            .hard_edges
            .get(&phi_value)
            .is_some_and(|neighbors| neighbors.contains(&unrelated_value)));
    }
}
