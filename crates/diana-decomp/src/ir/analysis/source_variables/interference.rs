use super::super::live_set::LiveSet;
use std::collections::{BTreeMap, BTreeSet};

use crate::ir::analysis::{SemanticFlowGraph, SemanticFlowPoint};
use crate::ir::{
    BlockId, InsnArg, InsnNode, InsnType, InstructionTree, InstructionVisitor, RegisterArg, CFG,
};
use crate::ir::{
    SemanticExpression, SemanticNode, SemanticPredicate, SemanticSiteId, SemanticStatement,
    SemanticVisitor,
};

use super::{edge_arguments::ContractedEdgeArguments, SourceVariableError, SsaClasses, SsaVar};

#[derive(Default)]
struct BlockLiveness {
    definitions: LiveSet,
    input: LiveSet,
    output: LiveSet,
    phi_definitions: LiveSet,
    transfers: std::ops::Range<usize>,
}

impl BlockLiveness {
    fn add_live_out(&mut self, word: usize, bits: u64) -> u64 {
        let new = self.output.add(word, bits);
        self.input.add(word, new & !self.definitions.word(word))
    }
}

/// Only instructions with a retained use or definition participate in
/// interference. Keep their positions for move-source exclusions and their
/// ordered operand indices in a common array across all physical blocks.
struct InstructionTransfer {
    position: usize,
    definition: Option<usize>,
    uses_end: usize,
}

pub(super) struct SsaLiveness {
    names: Box<[SsaVar]>,
    transfers: Vec<InstructionTransfer>,
    operands: Vec<usize>,
    blocks: BTreeMap<BlockId, BlockLiveness>,
    phi_uses: BTreeMap<(BlockId, BlockId, crate::ir::EdgeKind), LiveSet>,
}

impl SsaLiveness {
    pub(super) fn analyze(
        cfg: &CFG,
        retained: &BTreeSet<SsaVar>,
    ) -> Result<Self, SourceVariableError> {
        let names: Box<[_]> = retained.iter().copied().collect();
        let id = |name: SsaVar| names.binary_search(&name).ok();
        let mut blocks = BTreeMap::new();
        let mut phi_uses = BTreeMap::<_, LiveSet>::new();
        let mut transfers = Vec::new();
        let mut operands = Vec::new();
        for (&block, body) in &cfg.blocks {
            let mut facts = BlockLiveness::default();
            let start = transfers.len();
            for (position, instruction) in body.insns.iter().enumerate() {
                let uses_start = operands.len();
                let definition = instruction
                    .result
                    .as_ref()
                    .and_then(SsaVar::from_reg)
                    .and_then(id);

                if instruction.insn_type == InsnType::Phi {
                    if let Some(result) = definition {
                        facts.phi_definitions.insert(result);
                    }
                    for (&(predecessor, edge_kind), argument) in
                        instruction.payload.phi_edges.iter().zip(&instruction.args)
                    {
                        if let Some(value) = argument
                            .as_register()
                            .and_then(SsaVar::from_reg)
                            .and_then(id)
                        {
                            phi_uses
                                .entry((predecessor, block, edge_kind))
                                .or_default()
                                .insert(value);
                        }
                    }
                } else {
                    for usage in InstructionUses::collect(instruction)
                        .into_iter()
                        .filter_map(id)
                    {
                        if !facts.definitions.contains(usage) {
                            facts.input.insert(usage);
                        }
                        operands.push(usage);
                    }
                }
                if definition.is_some() || operands.len() != uses_start {
                    transfers.push(InstructionTransfer {
                        position,
                        definition,
                        uses_end: operands.len(),
                    });
                }
                if let Some(result) = definition {
                    facts.definitions.insert(result);
                }
            }
            facts.transfers = start..transfers.len();
            blocks.insert(block, facts);
        }

        let mut pending = Vec::new();
        for (&source, facts) in blocks.iter_mut().rev() {
            pending.extend(facts.input.words().map(|(word, bits)| (source, word, bits)));
            for &(target, kind) in cfg.successors_with_kind(source) {
                if !cfg.blocks.contains_key(&target) {
                    return Err(SourceVariableError::MissingBlock(target));
                }
                if let Some(arguments) = phi_uses.get(&(source, target, kind)) {
                    for (word, bits) in arguments.words() {
                        let added = facts.add_live_out(word, bits);
                        if added != 0 {
                            pending.push((source, word, added));
                        }
                    }
                }
            }
        }
        // Propagate whole words of new entry facts. Phi operands seed their
        // concrete edge above, while a Phi definition kills that incoming name.
        while let Some((target, word, bits)) = pending.pop() {
            let through = bits & !blocks[&target].phi_definitions.word(word);
            if through == 0 {
                continue;
            }
            for &(source, _) in cfg.predecessors_with_kind(target) {
                if let Some(facts) = blocks.get_mut(&source) {
                    let added = facts.add_live_out(word, through);
                    if added != 0 {
                        pending.push((source, word, added));
                    }
                }
            }
        }
        for facts in blocks.values_mut() {
            facts.definitions = LiveSet::default();
        }
        Ok(Self {
            names,
            transfers,
            operands,
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
            .flat_map(LiveSet::ids);
        Ok(facts
            .input
            .ids()
            .filter(|&value| !facts.phi_definitions.contains(value))
            .chain(operands.filter(|&value| {
                !facts.input.contains(value) || facts.phi_definitions.contains(value)
            }))
            .map(|id| &self.names[id]))
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
type Groups = foldhash::HashMap<SsaVar, SsaVar>;

#[derive(Default)]
pub(super) struct InterferenceGraph {
    groups: Option<Groups>,
    hard_edges: Edges,
    semantic_edges: Edges,
}

impl InterferenceGraph {
    pub(super) fn for_groups(groups: Option<Groups>) -> Self {
        Self {
            groups,
            ..Self::default()
        }
    }

    fn participates(&self, value: SsaVar) -> bool {
        self.groups
            .as_ref()
            .is_none_or(|groups| groups.contains_key(&value))
    }

    fn has_candidates(&self) -> bool {
        !self.groups.as_ref().is_some_and(|groups| groups.is_empty())
    }

    pub(super) fn build(
        cfg: &CFG,
        semantic: &SemanticNode,
        liveness: &SsaLiveness,
        retained: &BTreeSet<SsaVar>,
        required_phis: &BTreeSet<SsaVar>,
        groups: Option<Groups>,
        flow: impl FnOnce() -> std::sync::Arc<SemanticFlowGraph>,
    ) -> Result<Self, SourceVariableError> {
        let mut graph = Self::for_groups(groups);
        let needs_interference = graph.has_candidates();
        if needs_interference {
            if let Some(facts) = liveness.blocks.get(&cfg.entry) {
                Self::add_clique(
                    graph.groups.as_ref(),
                    &mut graph.hard_edges,
                    facts.input.ids().map(|id| liveness.names[id]),
                );
            }
        }
        for (&block, body) in &cfg.blocks {
            let facts = liveness
                .blocks
                .get(&block)
                .ok_or(SourceVariableError::MissingBlock(block))?;
            let mut live = facts.output.clone();
            for index in facts.transfers.clone().rev() {
                let step = &liveness.transfers[index];
                let start = index
                    .checked_sub(1)
                    .map_or(0, |i| liveness.transfers[i].uses_end);
                let uses = &liveness.operands[start..step.uses_end];
                if needs_interference {
                    Self::add_clique(
                        graph.groups.as_ref(),
                        &mut graph.hard_edges,
                        uses.iter().map(|&id| liveness.names[id]),
                    );
                }
                if let Some(id) = step.definition {
                    let definition = liveness.names[id];
                    if needs_interference && graph.participates(definition) {
                        let instruction = body
                            .insns
                            .get(step.position)
                            .ok_or(SourceVariableError::LivenessMismatch(block))?;
                        let move_source = (instruction.insn_type == InsnType::Move)
                            .then(|| instruction.args.first())
                            .flatten()
                            .and_then(InsnArg::as_register)
                            .and_then(SsaVar::from_reg);
                        for other in live
                            .ids()
                            .map(|id| liveness.names[id])
                            .filter(|value| *value != definition && Some(*value) != move_source)
                        {
                            graph.add(definition, other);
                        }
                    }
                    live.remove(id);
                }
                for &id in uses {
                    live.insert(id);
                }
            }
            if facts.input != live {
                return Err(SourceVariableError::LivenessMismatch(block));
            }
        }
        let semantic_liveness = SemanticLiveness::analyze(semantic, retained, &mut graph, flow)?;
        semantic_liveness.add_recovered_phi_interference(cfg, required_phis, &mut graph);
        Ok(graph)
    }

    pub(super) fn add_exceptional_edge_interference(
        &mut self,
        cfg: &CFG,
        liveness: &SsaLiveness,
        retained: &BTreeSet<SsaVar>,
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
                    .filter(|value| retained.contains(value))
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

    fn same_group(groups: Option<&Groups>, left: SsaVar, right: SsaVar) -> bool {
        groups.is_none_or(|groups| {
            groups
                .get(&left)
                .is_some_and(|group| groups.get(&right) == Some(group))
        })
    }

    fn insert_pair(edges: &mut Edges, left: SsaVar, right: SsaVar) {
        edges.entry(left).or_default().insert(right);
        edges.entry(right).or_default().insert(left);
    }

    fn add(&mut self, left: SsaVar, right: SsaVar) {
        if Self::same_group(self.groups.as_ref(), left, right) {
            Self::insert_pair(&mut self.hard_edges, left, right);
        }
    }

    fn add_semantic(&mut self, left: SsaVar, right: SsaVar) {
        if Self::same_group(self.groups.as_ref(), left, right) {
            Self::insert_pair(&mut self.semantic_edges, left, right);
        }
    }

    fn add_clique(
        groups: Option<&Groups>,
        edges: &mut Edges,
        mut values: impl Iterator<Item = SsaVar> + Clone,
    ) {
        // Short operand lists need no scratch table. Wide live-in sets can
        // contain many independent components: enumerate only pairs that
        // a candidate union could ever bring into the same source variable.
        if let Some(groups) = groups.filter(|_| {
            let (min, max) = values.size_hint();
            min > 8 || max.is_none_or(|n| n > 8) && values.clone().nth(8).is_some()
        }) {
            let mut members =
                foldhash::HashMap::<SsaVar, smallvec::SmallVec<[SsaVar; 4]>>::default();
            for value in values {
                if let Some(&group) = groups.get(&value) {
                    let peers = members.entry(group).or_default();
                    for &peer in peers.iter() {
                        Self::insert_pair(edges, peer, value);
                    }
                    peers.push(value);
                }
            }
        } else {
            while let Some(left) = values.next() {
                for right in values.clone() {
                    if Self::same_group(groups, left, right) {
                        Self::insert_pair(edges, left, right);
                    }
                }
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
        let sites =
            SemanticSiteCollector::new(retained, interference.has_candidates()).collect(root)?;
        let flow = flow();
        if !flow.is_complete() {
            return Err(SourceVariableError::IncompleteSemanticFlow);
        }
        let sites = match sites {
            SiteRecords::Facts(facts) => facts,
            SiteRecords::Identities(mut ids) => {
                ids.sort_unstable();
                let mut known = ids.into_iter().peekable();
                for point in flow.ordered_points() {
                    let SemanticFlowPoint::Before(site) = point else {
                        continue;
                    };
                    while known.next_if(|value| *value < site).is_some() {}
                    if known.next() != Some(site) {
                        return Err(SourceVariableError::MissingSemanticSite("flow point"));
                    }
                }
                return Ok(Self {
                    sites: BTreeMap::new(),
                });
            }
        };
        for facts in sites.values() {
            InterferenceGraph::add_clique(
                interference.groups.as_ref(),
                &mut interference.semantic_edges,
                facts.uses.iter().copied(),
            );
        }
        if let Some(points) = flow.reverse_linear_points() {
            let mut live = BTreeSet::new();
            for point in points {
                match point {
                    SemanticFlowPoint::Before(site) => {
                        let facts = sites
                            .get(&site)
                            .ok_or(SourceVariableError::MissingSemanticSite("flow point"))?;
                        // A statement can both read and define a name. Its
                        // incoming uses survive the kill of its outgoing value.
                        for definition in &facts.definitions {
                            live.remove(definition);
                        }
                        live.extend(
                            facts
                                .uses
                                .iter()
                                .copied()
                                .filter(|&value| interference.participates(value)),
                        );
                    }
                    SemanticFlowPoint::After(site) => {
                        if let Some(facts) = sites.get(&site) {
                            for &definition in &facts.definitions {
                                if !interference.participates(definition) {
                                    continue;
                                }
                                for &other in &live {
                                    if other != definition {
                                        interference.add_semantic(definition, other);
                                    }
                                }
                            }
                        }
                    }
                    SemanticFlowPoint::Synthetic(_) => {}
                }
            }
            return Ok(Self { sites });
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
                    for &value in &facts.uses {
                        if interference.participates(value) {
                            uses.entry(value).or_default().push(index);
                        }
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
                for predecessor in points.predecessors(index) {
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
        Ok(Self { sites })
    }

    fn add_recovered_phi_interference(
        &self,
        cfg: &CFG,
        required_phis: &BTreeSet<SsaVar>,
        graph: &mut InterferenceGraph,
    ) {
        if required_phis.is_empty() || !graph.has_candidates() {
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
                if !required_phis.contains(&result) || !graph.participates(result) {
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

// Site validation is required even when there are no names to coalesce.
// Only an interference query consumes the definitions and reads at each site.
enum SiteRecords {
    Identities(smallvec::SmallVec<[SemanticSiteId; 8]>),
    Facts(BTreeMap<SemanticSiteId, SemanticSiteFacts>),
}

struct SemanticSiteCollector<'a> {
    retained: &'a BTreeSet<SsaVar>,
    sites: SiteRecords,
}

impl<'a> SemanticSiteCollector<'a> {
    fn new(retained: &'a BTreeSet<SsaVar>, with_facts: bool) -> Self {
        let sites = if with_facts {
            SiteRecords::Facts(BTreeMap::new())
        } else {
            SiteRecords::Identities(Default::default())
        };
        Self { retained, sites }
    }

    fn collect(mut self, root: &SemanticNode) -> Result<SiteRecords, SourceVariableError> {
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
                    if let (SiteRecords::Facts(sites), Some(site), Some(value)) =
                        (&mut self.sites, iterable.site, SsaVar::from_reg(variable))
                    {
                        sites.entry(site).or_default().definitions.insert(value);
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
                    let Some(mut reader) = self.reads(site) else {
                        continue;
                    };
                    if let Some(condition) = &leave.condition {
                        reader.visit_predicate(condition);
                    }
                    if let Some(value) = leave.value() {
                        reader.visit_expression(value);
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
        let retained = self.retained;
        let Some(facts) = self.site(site) else {
            return Ok(());
        };
        if let Some(origin) = &statement.origin {
            facts.blocks.insert(origin.block);
        }
        if let Some(result) = statement.result().and_then(SsaVar::from_reg) {
            facts.definitions.insert(result);
        }
        RetainedReads {
            retained,
            uses: &mut facts.uses,
        }
        .visit_statement(statement);
        Ok(())
    }

    fn record_predicate(
        &mut self,
        site: Option<SemanticSiteId>,
        predicate: &SemanticPredicate,
    ) -> Result<(), SourceVariableError> {
        let site = site.ok_or(SourceVariableError::MissingSemanticSite("predicate"))?;
        if let Some(mut reads) = self.reads(site) {
            reads.visit_predicate(predicate);
        }
        Ok(())
    }

    fn record_expression(
        &mut self,
        site: Option<SemanticSiteId>,
        expression: &SemanticExpression,
    ) -> Result<(), SourceVariableError> {
        let site = site.ok_or(SourceVariableError::MissingSemanticSite("expression"))?;
        if let Some(mut reads) = self.reads(site) {
            reads.visit_expression(expression);
        }
        Ok(())
    }

    fn reads(&mut self, site: SemanticSiteId) -> Option<RetainedReads<'_>> {
        let retained = self.retained;
        Some(RetainedReads {
            retained,
            uses: &mut self.site(site)?.uses,
        })
    }

    fn site(&mut self, site: SemanticSiteId) -> Option<&mut SemanticSiteFacts> {
        match &mut self.sites {
            SiteRecords::Identities(ids) => {
                ids.push(site);
                None
            }
            SiteRecords::Facts(sites) => Some(sites.entry(site).or_default()),
        }
    }
}

struct RetainedReads<'a> {
    retained: &'a BTreeSet<SsaVar>,
    uses: &'a mut BTreeSet<SsaVar>,
}

impl SemanticVisitor for RetainedReads<'_> {
    fn visit_register(&mut self, register: &RegisterArg) {
        if let Some(value) = SsaVar::from_reg(register) {
            if self.retained.contains(&value) {
                self.uses.insert(value);
            }
        }
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
            &retained,
            &BTreeSet::new(),
            None,
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
            &retained,
            &BTreeSet::from([phi_value]),
            None,
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

#[cfg(test)]
#[path = "../../../../tests/internal/linear_semantic_liveness.rs"]
mod linear_semantic_liveness;

#[cfg(test)]
#[path = "../../../../tests/internal/ssa_live_words.rs"]
mod ssa_live_words;
