use std::collections::{BTreeMap, BTreeSet};

use crate::ir::analysis::SsaVar;
use crate::ir::{
    ArgType, BlockId, RegionEdge, RegionGraph, RegionId, RegionKind, RegisterArg, SemanticCatch,
    SemanticExpression, SemanticExpressionTransform, SemanticFinally, SemanticFolder,
    SemanticInstructions, SemanticLeaveKind, SemanticNode, CFG,
};

use super::super::{
    continuation::{ContinuationBinding, ContinuationPort, ControlPort, ControlTransfer},
    StructureError,
};
use super::region_cfg::RegionCfg;
use super::{ExceptionEnvelope, ReducedPort, ReducedRegion, RegionChildren};

struct ExceptionForwarding {
    thrown: RegisterArg,
    origin: Option<crate::ir::BlockId>,
    edge: Option<crate::ir::RegionEdge>,
    caught: RegisterArg,
    body: SemanticNode,
}

impl ExceptionForwarding {
    fn collapse(
        node: SemanticNode,
        handler_exception: Option<&RegisterArg>,
    ) -> Result<SemanticNode, StructureError> {
        let Some(forwarding) = Self::analyze(&node, handler_exception) else {
            return Ok(node);
        };
        let Some(caught) = SsaVar::from_reg(&forwarding.caught) else {
            return Ok(node);
        };
        let Some(thrown) = SsaVar::from_reg(&forwarding.thrown) else {
            return Ok(node);
        };
        let mut body = forwarding.body;
        SemanticInstructions::transform(
            &mut body,
            &mut ExceptionValueSubstitution {
                caught,
                thrown: forwarding.thrown,
            },
        )?;
        let mut provenance = ForwardedThrowProvenance {
            thrown,
            origin: forwarding.origin,
            edge: forwarding.edge,
            count: 0,
        };
        body = provenance.fold_node(body)?;
        if provenance.count != 1 {
            return Ok(node);
        }
        Ok(body)
    }

    fn analyze(node: &SemanticNode, handler_exception: Option<&RegisterArg>) -> Option<Self> {
        let SemanticNode::Try {
            body,
            catches,
            finally: None,
            ..
        } = node
        else {
            return None;
        };
        let [catch] = catches.as_slice() else {
            return None;
        };
        if !catch
            .exception_types
            .iter()
            .any(|exception| exception.as_object() == Some("java/lang/Throwable"))
        {
            return None;
        }
        let thrown = ForwardedException::analyze(body)?;
        let handler_exception = handler_exception?;
        if SsaVar::from_reg(&thrown.value) != SsaVar::from_reg(handler_exception) {
            return None;
        }
        Some(Self {
            thrown: thrown.value,
            origin: thrown.origin,
            edge: thrown.edge,
            caught: catch.exception_value.clone()?,
            body: catch.body.clone(),
        })
    }
}

struct ForwardedThrow {
    value: RegisterArg,
    origin: Option<crate::ir::BlockId>,
    edge: Option<crate::ir::RegionEdge>,
}

struct ForwardedException;

impl ForwardedException {
    fn analyze(node: &SemanticNode) -> Option<ForwardedThrow> {
        let nodes = match node {
            SemanticNode::Sequence(nodes) => nodes.as_slice(),
            node => std::slice::from_ref(node),
        };
        let mut thrown = None;
        for node in nodes {
            match node {
                SemanticNode::Empty => {}
                SemanticNode::BasicBlock(block) if block.statements.is_empty() => {}
                SemanticNode::Leave(leave)
                    if leave.condition.is_none()
                        && leave.cleanup.is_empty()
                        && matches!(&leave.kind, SemanticLeaveKind::Throw(_)) =>
                {
                    let SemanticLeaveKind::Throw(SemanticExpression::Register(value)) = &leave.kind
                    else {
                        return None;
                    };
                    if thrown
                        .replace(ForwardedThrow {
                            value: value.clone(),
                            origin: leave.origin,
                            edge: leave.edge,
                        })
                        .is_some()
                    {
                        return None;
                    }
                }
                _ => return None,
            }
        }
        thrown
    }
}

struct ForwardedThrowProvenance {
    thrown: SsaVar,
    origin: Option<crate::ir::BlockId>,
    edge: Option<crate::ir::RegionEdge>,
    count: usize,
}

impl SemanticFolder for ForwardedThrowProvenance {
    type Error = crate::ir::SemanticFoldError;

    fn finish_node(&mut self, node: SemanticNode) -> Result<SemanticNode, Self::Error> {
        let SemanticNode::Leave(mut leave) = node else {
            return Ok(node);
        };
        let SemanticLeaveKind::Throw(SemanticExpression::Register(value)) = &leave.kind else {
            return Ok(SemanticNode::Leave(leave));
        };
        if SsaVar::from_reg(value) == Some(self.thrown) {
            self.count += 1;
            leave.origin = self.origin;
            leave.edge = self.edge;
        }
        Ok(SemanticNode::Leave(leave))
    }
}

struct ExceptionValueSubstitution {
    caught: SsaVar,
    thrown: RegisterArg,
}

struct ForwardedHandlerBody {
    port: ContinuationPort,
    edge: Option<RegionEdge>,
    body: SemanticNode,
    replacements: usize,
}

impl ForwardedHandlerBody {
    fn inline(
        adapter: SemanticNode,
        port: ContinuationPort,
        edge: Option<RegionEdge>,
        body: SemanticNode,
    ) -> Result<Option<SemanticNode>, StructureError> {
        let mut forwarding = Self {
            port,
            edge,
            body,
            replacements: 0,
        };
        let candidate = forwarding
            .fold_node(adapter)
            .map_err(StructureError::from)?;
        Ok((forwarding.replacements == 1).then_some(candidate))
    }
}

impl SemanticFolder for ForwardedHandlerBody {
    type Error = crate::ir::SemanticFoldError;

    fn finish_node(&mut self, node: SemanticNode) -> Result<SemanticNode, Self::Error> {
        let SemanticNode::Leave(leave) = &node else {
            return Ok(node);
        };
        let target = match &leave.kind {
            SemanticLeaveKind::FallThrough(target) | SemanticLeaveKind::Jump(target) => *target,
            _ => return Ok(node),
        };
        if leave.target != self.port.scope
            || target != self.port.target
            || leave.condition.is_some()
            || !leave.cleanup.is_empty()
            || self.edge.is_some_and(|edge| leave.edge != Some(edge))
        {
            return Ok(node);
        }
        self.replacements += 1;
        Ok(self.body.clone())
    }
}

impl SemanticExpressionTransform for ExceptionValueSubstitution {
    fn transform_register(&mut self, register: RegisterArg) -> SemanticExpression {
        if SsaVar::from_reg(&register) == Some(self.caught) {
            SemanticExpression::Register(self.thrown.clone())
        } else {
            SemanticExpression::Register(register)
        }
    }
}

pub(super) struct HandlerContraction {
    pub(super) envelope: ExceptionEnvelope,
    pub(super) ports: BTreeMap<crate::ir::BlockId, HandlerPort>,
}

pub(super) struct HandlerPort {
    pub(super) handler: RegionId,
    pub(super) port: ReducedPort,
}

pub(super) struct HandlerReducer<'a> {
    regions: &'a RegionGraph,
}

#[derive(Clone, Copy)]
pub(super) struct CleanupContinuation {
    pub(super) handler: RegionId,
    port: ContinuationPort,
    edge: RegionEdge,
}

impl<'a> HandlerReducer<'a> {
    pub(super) fn new(regions: &'a RegionGraph) -> Self {
        Self { regions }
    }

    pub(super) fn body_region(
        &self,
        owner: RegionId,
        handler: RegionId,
    ) -> Result<RegionId, StructureError> {
        let tree = self.regions.tree();
        let handler_region = tree
            .region(handler)
            .ok_or(StructureError::UnknownRegion(handler))?;
        let Some(entry) = handler_region.entry else {
            let bodies = handler_region
                .children
                .iter()
                .copied()
                .filter(|child| {
                    tree.region(*child).is_some_and(|child| {
                        child.entry.is_some() && child.blocks == handler_region.blocks
                    })
                })
                .collect::<Vec<_>>();
            if let [body] = bodies.as_slice() {
                return Ok(*body);
            }
            return Ok(handler);
        };
        let mut body = handler;
        loop {
            let current = tree
                .region(body)
                .ok_or(StructureError::UnknownRegion(body))?;
            let Some(parent) = current.parent else {
                break;
            };
            if self.regions.handlers_of(parent).contains(&handler) {
                break;
            }
            if parent == owner {
                break;
            }
            let envelope = tree
                .region(parent)
                .ok_or(StructureError::UnknownRegion(parent))?;
            if envelope.entry != Some(entry)
                || envelope.blocks != current.blocks
                || !matches!(
                    &envelope.kind,
                    RegionKind::Try
                        | RegionKind::Synchronized(_)
                        | RegionKind::Loop(_)
                        | RegionKind::Switch(_)
                )
            {
                break;
            }
            body = parent;
        }
        Ok(body)
    }

    pub(super) fn reduce(
        &self,
        owner: RegionId,
        children: &RegionChildren,
        region_cfg: &RegionCfg,
        reduced: &mut BTreeMap<RegionId, ReducedRegion>,
    ) -> Result<HandlerContraction, StructureError> {
        let parent = self
            .regions
            .tree()
            .region(owner)
            .ok_or(StructureError::UnknownRegion(owner))?
            .parent
            .unwrap_or(owner);
        let mut envelope = ExceptionEnvelope::default();
        let mut ports = BTreeMap::new();
        for child in &children.handlers {
            if children.releases.contains(child)
                || self.regions.is_synchronization_release_handler(*child)
            {
                reduced
                    .get(child)
                    .ok_or(StructureError::MissingReduction(*child))?;
                continue;
            }
            let handler = self
                .regions
                .tree()
                .region(*child)
                .ok_or(StructureError::UnknownRegion(*child))?
                .clone();
            match handler.kind {
                RegionKind::Catch(catch) => {
                    let body_region = self.body_region(owner, *child)?;
                    let (reduction, alternate) = self.reduction(reduced, body_region)?;
                    let reduction = self.inline_forwarded_catch(*child, reduction, reduced)?;
                    envelope.catches.push(SemanticCatch {
                        region: *child,
                        exception_types: catch.exception_types,
                        exception_value: catch.exception_value,
                        body: self.bind(owner, parent, *child, region_cfg, reduction)?,
                    });
                    self.expose(owner, parent, *child, region_cfg, alternate, &mut ports)?;
                }
                RegionKind::Finally => {
                    let body_region = self.body_region(owner, *child)?;
                    let (reduction, alternate) = self.reduction(reduced, body_region)?;
                    let body = self.bind(owner, parent, *child, region_cfg, reduction)?;
                    if let Some(existing) = &envelope.finally {
                        return Err(StructureError::MultipleFinallyRegions {
                            owner,
                            first: existing.region,
                            second: *child,
                        });
                    }
                    envelope.finally = Some(SemanticFinally {
                        region: *child,
                        body: Box::new(body),
                    });
                    self.expose(owner, parent, *child, region_cfg, alternate, &mut ports)?;
                }
                RegionKind::Cleanup(cleanup) => {
                    let source_finally = self.source_finally_body(owner, *child)?;
                    let body_region = source_finally.unwrap_or(self.body_region(owner, *child)?);
                    let (mut reduction, alternate) = self.reduction(reduced, body_region)?;
                    if let Some(forwarding) = children.forwarded_cleanups.get(child) {
                        reduction = self.inline_forwarded_body(
                            reduction,
                            forwarding.handler,
                            forwarding.port,
                            Some(forwarding.edge),
                            reduced,
                        )?;
                    }
                    let exception_value = cleanup.exception_value;
                    let semantic_handler = source_finally.unwrap_or(*child);
                    let body = self.bind(owner, parent, semantic_handler, region_cfg, reduction)?;
                    let body = ExceptionForwarding::collapse(body, exception_value.as_ref())?;
                    if let Some(finally_region) = source_finally {
                        if let Some(existing) = &envelope.finally {
                            return Err(StructureError::MultipleFinallyRegions {
                                owner,
                                first: existing.region,
                                second: finally_region,
                            });
                        }
                        envelope.finally = Some(SemanticFinally {
                            region: finally_region,
                            body: Box::new(body),
                        });
                    } else {
                        envelope.catches.push(SemanticCatch {
                            region: *child,
                            exception_types: cleanup.exception_types,
                            exception_value,
                            body,
                        });
                    }
                    self.expose(
                        owner,
                        parent,
                        semantic_handler,
                        region_cfg,
                        alternate,
                        &mut ports,
                    )?;
                }
                _ => {
                    return Err(StructureError::InvalidHandlerRegion {
                        owner,
                        handler: *child,
                    });
                }
            }
        }
        Ok(HandlerContraction { envelope, ports })
    }

    /// DEX can split one source catch into register-state adapters that enter
    /// a shared lexical body. Once exception analysis proves that relation,
    /// retain the adapter path but bind its sole continuation to the already
    /// reduced ancestor catch instead of exposing the body entry to the
    /// enclosing ordinary control-flow graph.
    fn inline_forwarded_catch(
        &self,
        handler: RegionId,
        reduction: ReducedPort,
        reduced: &BTreeMap<RegionId, ReducedRegion>,
    ) -> Result<ReducedPort, StructureError> {
        let Some((target_handler, target)) = self.forwarded_catch_target(handler)? else {
            return Ok(reduction);
        };
        let forwarding = ContinuationPort {
            scope: target_handler,
            target,
        };
        self.inline_forwarded_body(reduction, target_handler, forwarding, None, reduced)
    }

    fn inline_forwarded_body(
        &self,
        reduction: ReducedPort,
        target_handler: RegionId,
        forwarding: ContinuationPort,
        edge: Option<RegionEdge>,
        reduced: &BTreeMap<RegionId, ReducedRegion>,
    ) -> Result<ReducedPort, StructureError> {
        if reduction.continuations.ports().len() != 1
            || !reduction.continuations.ports().contains(&forwarding)
            || !reduction.continuations.controls().is_empty()
        {
            return Ok(reduction);
        }
        let Some(target_body) = reduced
            .get(&target_handler)
            .and_then(|reduction| reduction.ports.get(&forwarding.target))
        else {
            return Ok(reduction);
        };
        if edge.is_some()
            && (!target_body.continuations.ports().is_empty()
                || !target_body.continuations.controls().is_empty())
        {
            return Ok(reduction);
        }
        let original = reduction.body;
        let Some(body) = ForwardedHandlerBody::inline(
            original.clone(),
            forwarding,
            edge,
            target_body.body.clone(),
        )?
        else {
            return Ok(ReducedPort::new(original));
        };
        Ok(ReducedPort::new(body))
    }

    pub(super) fn forwarded_catch_target(
        &self,
        handler: RegionId,
    ) -> Result<Option<(RegionId, crate::ir::BlockId)>, StructureError> {
        let tree = self.regions.tree();
        let descriptor = tree
            .region(handler)
            .ok_or(StructureError::UnknownRegion(handler))?;
        let RegionKind::Catch(source_catch) = &descriptor.kind else {
            return Ok(None);
        };
        let Some(entry) = descriptor.entry else {
            return Ok(None);
        };
        let Some(target) = self.regions.handler_adapters().get(&entry).copied() else {
            return Ok(None);
        };
        let Some(target_handler) = self.regions.owner_of(target) else {
            return Ok(None);
        };
        if !tree
            .is_ancestor(target_handler, handler)
            .map_err(|_| StructureError::UnknownRegion(target_handler))?
        {
            return Ok(None);
        }
        let target_descriptor = tree
            .region(target_handler)
            .ok_or(StructureError::UnknownRegion(target_handler))?;
        let RegionKind::Catch(target_catch) = &target_descriptor.kind else {
            return Ok(None);
        };
        Ok((target_descriptor.entry == Some(target)
            && source_catch.exception_types == target_catch.exception_types
            && source_catch.exception_value == target_catch.exception_value)
            .then_some((target_handler, target)))
    }

    /// A captured exception may reach shared cleanup code on a normal
    /// adapter edge. The receiver can be emitted in this catch only if every
    /// throwing operation still reaches its original outward handler.
    pub(super) fn forwarded_cleanup(
        &self,
        cfg: &CFG,
        owner: RegionId,
        handler: RegionId,
    ) -> Result<Option<CleanupContinuation>, StructureError> {
        let tree = self.regions.tree();
        let source = tree
            .region(handler)
            .ok_or(StructureError::UnknownRegion(handler))?;
        if !matches!(source.kind, RegionKind::Cleanup(_))
            || self.source_finally_body(owner, handler)?.is_some()
        {
            return Ok(None);
        }
        let mut exits = self.regions.transfers().iter().filter(|transfer| {
            transfer.source_region == handler && !source.blocks.contains(&transfer.target_block)
        });
        let Some(exit) = exits.next() else {
            return Ok(None);
        };
        if exits.next().is_some()
            || self.regions.handler_adapters().get(&exit.source_block) != Some(&exit.target_block)
            || exit.target_block != exit.destination_block
        {
            return Ok(None);
        }
        let edge = RegionEdge {
            source: exit.source_block,
            target: exit.target_block,
            kind: exit.edge_kind,
        };
        // The copies on this edge belong to its source block when there is
        // only one normal successor. Its anchor remains in the adapter body.
        if !cfg.normal_successors(edge.source).eq([edge.target]) {
            return Ok(None);
        }
        let Some(leave) = self.regions.leave_for_edge(edge) else {
            return Ok(None);
        };
        if !leave.cleanup_regions.is_empty()
            || !matches!(leave.leave.exit, crate::ir::RegionExit::FallThrough(target) if target == edge.target)
        {
            return Ok(None);
        }
        let target = tree
            .region(exit.target_region)
            .ok_or(StructureError::UnknownRegion(exit.target_region))?;
        if !matches!(target.kind, RegionKind::Cleanup(_))
            || target.entry != Some(edge.target)
            || tree
                .is_ancestor(target.id, owner)
                .map_err(StructureError::from)?
        {
            return Ok(None);
        }

        let mut ambient = None;
        for ancestor in tree
            .parent_chain(owner)
            .map_err(StructureError::from)?
            .into_iter()
            .skip(1)
        {
            let clauses = self.regions.handlers_of(ancestor);
            let outer = match clauses {
                [] => continue,
                [outer] => *outer,
                _ => return Ok(None),
            };
            if tree
                .is_ancestor(outer, owner)
                .map_err(StructureError::from)?
            {
                return Ok(None);
            }
            let descriptor = tree
                .region(outer)
                .ok_or(StructureError::UnknownRegion(outer))?;
            match &descriptor.kind {
                RegionKind::Cleanup(catch) | RegionKind::Catch(catch)
                    if catch.exception_types.as_slice() == [ArgType::throwable()] =>
                {
                    ambient = Some(outer)
                }
                _ => return Ok(None),
            }
            break;
        }

        let mut visited = BTreeSet::new();
        let mut pending = vec![edge.target];
        while let Some(block) = pending.pop() {
            if !visited.insert(block) {
                continue;
            }
            if !target.blocks.contains(&block)
                || !self.matches_cleanup_dispatch(cfg, block, ambient)?
            {
                return Ok(None);
            }
            pending.extend(cfg.normal_successors(block));
        }
        Ok(Some(CleanupContinuation {
            handler: target.id,
            edge,
            port: ContinuationPort {
                scope: leave.leave.target,
                target: edge.target,
            },
        }))
    }

    fn matches_cleanup_dispatch(
        &self,
        cfg: &CFG,
        block: BlockId,
        ambient: Option<RegionId>,
    ) -> Result<bool, StructureError> {
        let body = cfg
            .block(block)
            .ok_or(StructureError::MissingBlock(block))?;
        if !body.insns.iter().any(|insn| insn.can_throw()) {
            return Ok(true);
        }
        let mut dispatch = cfg
            .successors_with_kind(block)
            .iter()
            .filter_map(|&(entry, kind)| kind.is_exception().then_some(entry));
        let physical = match (ambient, dispatch.next()) {
            (None, None) => None,
            (Some(outer), Some(physical)) if dispatch.next().is_none() => {
                let canonical = self
                    .regions
                    .handler_adapters()
                    .get(&physical)
                    .copied()
                    .unwrap_or(physical);
                let Some(entry) = cfg.block(physical).filter(|entry| !entry.synthetic) else {
                    return Ok(false);
                };
                if self.regions.enclosing_exception_handler(canonical) != Some(outer) {
                    return Ok(false);
                }
                Some(entry.offset)
            }
            _ => return Ok(false),
        };
        Ok(body
            .insns
            .iter()
            .filter(|insn| insn.can_throw())
            .all(|insn| {
                let mut clauses = cfg
                    .handlers
                    .iter()
                    .filter(|clause| clause.covers(insn.offset));
                match (physical, clauses.next()) {
                    (None, None) => true,
                    (Some(offset), Some(clause)) => {
                        clause.is_catch_all()
                            && clause.handler == offset
                            && clauses.next().is_none()
                    }
                    _ => false,
                }
            }))
    }

    pub(super) fn source_finally_body(
        &self,
        owner: RegionId,
        cleanup: RegionId,
    ) -> Result<Option<RegionId>, StructureError> {
        if !self.regions.handlers_of(owner).contains(&cleanup) {
            return Ok(None);
        }
        let tree = self.regions.tree();
        let wrapper = tree
            .region(cleanup)
            .ok_or(StructureError::UnknownRegion(cleanup))?;
        let candidates = wrapper
            .children
            .iter()
            .copied()
            .filter(|child| {
                tree.region(*child).is_some_and(|region| {
                    matches!(region.kind, RegionKind::Finally)
                        && region.entry == wrapper.entry
                        && region.blocks.is_subset(&wrapper.blocks)
                })
            })
            .collect::<Vec<_>>();
        Ok(match candidates.as_slice() {
            [finally] => Some(*finally),
            _ => None,
        })
    }

    fn reduction(
        &self,
        reduced: &mut BTreeMap<RegionId, ReducedRegion>,
        region: RegionId,
    ) -> Result<(ReducedPort, BTreeMap<crate::ir::BlockId, ReducedPort>), StructureError> {
        let reduction = reduced
            .get(&region)
            .ok_or(StructureError::MissingReduction(region))?;
        let entry = self
            .regions
            .tree()
            .region(region)
            .ok_or(StructureError::UnknownRegion(region))?
            .entry;
        let primary = match entry {
            Some(entry) => reduction
                .ports
                .get(&entry)
                .ok_or(StructureError::RegionEntryMissing { region, entry }),
            None => reduction
                .detached
                .as_ref()
                .ok_or(StructureError::MissingEntry(region)),
        }?;
        let mut alternate = BTreeMap::new();
        for (&alternate_entry, port) in &reduction.ports {
            if Some(alternate_entry) != entry && self.entry_belongs_to(region, alternate_entry)? {
                alternate.insert(alternate_entry, port.clone());
            }
        }
        Ok((primary.clone(), alternate))
    }

    fn entry_belongs_to(
        &self,
        body: RegionId,
        entry: crate::ir::BlockId,
    ) -> Result<bool, StructureError> {
        let owner = self
            .regions
            .owner_of(entry)
            .ok_or(StructureError::RegionOwnerMissing {
                region: body,
                block: entry,
            })?;
        Ok(owner == body
            || self
                .regions
                .tree()
                .is_ancestor(body, owner)
                .map_err(|_| StructureError::UnknownRegion(owner))?)
    }

    fn expose(
        &self,
        owner: RegionId,
        parent: RegionId,
        handler: RegionId,
        region_cfg: &RegionCfg,
        alternate: BTreeMap<crate::ir::BlockId, ReducedPort>,
        ports: &mut BTreeMap<crate::ir::BlockId, HandlerPort>,
    ) -> Result<(), StructureError> {
        for (entry, port) in alternate {
            let body = self.bind(owner, parent, handler, region_cfg, port)?;
            if ports
                .insert(
                    entry,
                    HandlerPort {
                        handler,
                        port: ReducedPort::new(body),
                    },
                )
                .is_some()
            {
                return Err(StructureError::ConflictingEntryPort { owner, entry });
            }
        }
        Ok(())
    }

    fn bind(
        &self,
        scope: RegionId,
        parent: RegionId,
        handler_region: RegionId,
        region_cfg: &RegionCfg,
        handler: ReducedPort,
    ) -> Result<SemanticNode, StructureError> {
        let targets = handler
            .continuations
            .ports()
            .iter()
            .copied()
            .map(|port| {
                let target = region_cfg
                    .mapping
                    .get(&port.target)
                    .copied()
                    .unwrap_or(port.target);
                Ok((port.target, target))
            })
            .collect::<Result<BTreeMap<_, _>, StructureError>>()?;
        let control_targets = handler
            .continuations
            .controls()
            .iter()
            .copied()
            .map(|port| self.detached_control_target(scope, handler_region, port))
            .collect::<Result<Vec<_>, StructureError>>()?
            .into_iter()
            .flatten()
            .collect::<BTreeMap<_, _>>();
        ContinuationBinding::defer_handler(
            &handler.continuations,
            scope,
            parent,
            &targets,
            &control_targets,
        )?
        .apply(handler.body)
    }

    fn detached_control_target(
        &self,
        owner: RegionId,
        handler: RegionId,
        port: ControlPort,
    ) -> Result<Option<(ControlPort, crate::ir::BlockId)>, StructureError> {
        let tree = self.regions.tree();
        if tree
            .is_ancestor(handler, port.target)
            .map_err(|_| StructureError::UnknownRegion(port.target))?
            || tree
                .is_ancestor(port.target, owner)
                .map_err(|_| StructureError::UnknownRegion(port.target))?
        {
            return Ok(None);
        }
        let region = tree
            .region(port.target)
            .ok_or(StructureError::UnknownRegion(port.target))?;
        let target = match port.transfer {
            ControlTransfer::Continue => region.entry,
            ControlTransfer::Break => region.kind.follow(),
        }
        .ok_or(StructureError::MissingControlTarget(port.target))?;
        Ok(Some((port, target)))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::{ForwardedHandlerBody, SemanticNode};
    use crate::ir::structure::continuation::{ContinuationFacts, ContinuationPort};
    use crate::ir::{
        BlockId, EdgeKind, RegionEdge, RegionId, SemanticBlock, SemanticLeave, SemanticLeaveKind,
    };

    fn fallthrough(scope: RegionId, target: BlockId) -> SemanticNode {
        SemanticNode::Leave(SemanticLeave {
            site: None,
            condition: None,
            kind: SemanticLeaveKind::FallThrough(target),
            edge: None,
            origin: None,
            source: scope,
            destination: scope,
            target: scope,
            cleanup: Vec::new(),
        })
    }

    #[test]
    fn forwarded_handler_body_replaces_the_adapter_continuation() {
        let handler = RegionId::new(5);
        let enclosing = RegionId::new(0);
        let entry = BlockId::new(65);
        let continuation = BlockId::new(70);
        let source = BlockId::new(58);
        let edge = RegionEdge {
            source,
            target: entry,
            kind: EdgeKind::Normal,
        };
        let port = ContinuationPort {
            scope: handler,
            target: entry,
        };
        let SemanticNode::Leave(mut leave) = fallthrough(handler, entry) else {
            unreachable!("fallthrough produces a leave");
        };
        leave.edge = Some(edge);
        leave.origin = Some(source);
        let adapter = SemanticNode::sequence([
            SemanticNode::BasicBlock(SemanticBlock {
                id: source,
                statements: Vec::new(),
            }),
            SemanticNode::Leave(leave),
        ]);
        let body = SemanticNode::sequence([
            SemanticNode::BasicBlock(SemanticBlock {
                id: entry,
                statements: Vec::new(),
            }),
            fallthrough(enclosing, continuation),
        ]);

        let inlined = ForwardedHandlerBody::inline(adapter.clone(), port, Some(edge), body)
            .expect("forwarding rewrite")
            .expect("one matching adapter continuation");
        assert_eq!(
            ContinuationFacts::analyze(&inlined).ports(),
            &BTreeSet::from([ContinuationPort {
                scope: enclosing,
                target: continuation,
            }])
        );
        assert!(ForwardedHandlerBody::inline(
            adapter.clone(),
            ContinuationPort {
                scope: handler,
                target: BlockId::new(66),
            },
            None,
            SemanticNode::Empty,
        )
        .expect("non-matching rewrite")
        .is_none());
        assert!(ForwardedHandlerBody::inline(
            adapter,
            port,
            Some(RegionEdge {
                source: BlockId::new(57),
                ..edge
            }),
            SemanticNode::Empty,
        )
        .expect("non-matching physical edge")
        .is_none());
    }
}
