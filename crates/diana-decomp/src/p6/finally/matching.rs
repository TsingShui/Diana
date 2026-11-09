//! Lockstep cleanup events over original nodes. Block cuts and copy-free gotos
//! are transparent; branch roles and join correspondence are not. The active
//! pair set rejects cycles without enumerating complete execution paths.

use super::*;
use crate::p2::{BranchOp, EdgeRole, NormalEdge};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct Position {
    node: NodeId,
    operation: u32,
}
impl Position {
    fn entry(f: &RegionFacts, node: NodeId) -> Self {
        Self {
            node,
            operation: f.nodes[node.0 as usize].operations.start,
        }
    }
    fn after(self) -> Result<Self, RegionError> {
        Ok(Self {
            operation: self
                .operation
                .checked_add(1)
                .ok_or(crate::RequestError::PlatformCapacity)?,
            ..self
        })
    }
}
#[derive(Clone, Copy)]
enum Event {
    Operation(OpId),
    Branch(BranchOp, [NodeId; 2]),
    Throw,
}
struct Cursor<'a> {
    allowed: &'a [bool],
    visited: Vec<bool>,
    order: Vec<NodeId>,
}
impl Cursor<'_> {
    fn touch(&mut self, node: NodeId) -> Result<(), RegionError> {
        if !self.visited[node.0 as usize] {
            self.visited[node.0 as usize] = true;
            push(&mut self.order, node)?;
        }
        Ok(())
    }
}
fn branch(
    f: &RegionFacts,
    g: &Graph,
    storage: &StorageMethod<'_>,
    node: NodeId,
) -> Option<[NodeId; 2]> {
    let mut arms = [None; 2];
    if g.outgoing[node.0 as usize].len() != 2 {
        return None;
    }
    for &id in &g.outgoing[node.0 as usize] {
        let link = &f.links[id.0 as usize];
        let LinkOrigin::Edge(edge) = link.origin else {
            return None;
        };
        let slot = match storage.execution().edges()[edge.0 as usize].role() {
            EdgeRole::Normal(NormalEdge::ConditionalTrue) => 0,
            EdgeRole::Normal(NormalEdge::ConditionalFalse) => 1,
            _ => return None,
        };
        if arms[slot].replace(link.to).is_some() {
            return None;
        }
    }
    Some([arms[0]?, arms[1]?])
}
fn event(
    storage: &StorageMethod<'_>,
    f: &RegionFacts,
    g: &Graph,
    cursor: &mut Cursor<'_>,
    mut at: Position,
    capture: Option<OpId>,
) -> Result<Option<(Position, Event)>, RegionError> {
    // Each transparent step crosses an original node. Repeating without an
    // observable event is a cycle, not an empty cleanup.
    for _ in 0..=f.nodes.len() {
        if !cursor.allowed[at.node.0 as usize] || !f.lock_entries.empty_at(at.node) {
            return Ok(None);
        }
        cursor.touch(at.node)?;
        let n = &f.nodes[at.node.0 as usize];
        if Some(OpId(at.operation)) == capture && at.operation < n.operations.end {
            at = at.after()?;
        }
        if at.operation < n.operations.end {
            return Ok(Some((at, Event::Operation(OpId(at.operation)))));
        }
        if at.operation != n.operations.end {
            return Err(bug("cleanup cursor exceeds its node"));
        }
        if n.terminal {
            match storage.execution().blocks()[n.block.0 as usize].terminal() {
                Some(Terminal::Throw { .. }) => return Ok(Some((at, Event::Throw))),
                Some(Terminal::Branch { op, .. }) => {
                    return Ok(
                        branch(f, g, storage, at.node).map(|arms| (at, Event::Branch(*op, arms)))
                    );
                }
                Some(Terminal::Goto { .. }) => {}
                _ => return Ok(None),
            }
        }
        let [link] = g.outgoing[at.node.0 as usize].as_slice() else {
            return Ok(None);
        };
        at = Position::entry(f, f.links[link.0 as usize].to);
    }
    Ok(None)
}
// Finish at the first real continuation, not at a branch-specific empty goto.
// Consumed routing remains in the normal cleanup product so its copies/owners
// receive the same admission checks as every other folded node.
fn normal_continuation(
    storage: &StorageMethod<'_>,
    f: &RegionFacts,
    g: &Graph,
    cursor: &mut Cursor<'_>,
    mut at: Position,
) -> Result<Option<Position>, RegionError> {
    for _ in 0..=f.nodes.len() {
        if !cursor.allowed[at.node.0 as usize] {
            return Ok(None);
        }
        let node = &f.nodes[at.node.0 as usize];
        if at.operation != node.operations.start {
            if at.operation != node.operations.end || node.terminal {
                return Ok(None);
            }
        } else {
            let transparent = node.operations.is_empty()
                && (!node.terminal
                    || matches!(
                        storage.execution().blocks()[node.block.0 as usize].terminal(),
                        Some(Terminal::Goto { .. })
                    ));
            if !transparent {
                return Ok(Some(at));
            }
            if !f.lock_entries.empty_at(at.node) {
                return Ok(None);
            }
            cursor.touch(at.node)?;
        }
        let [link] = g.outgoing[at.node.0 as usize].as_slice() else {
            return Ok(None);
        };
        at = Position::entry(f, f.links[link.0 as usize].to);
    }
    Ok(None)
}
// A paired definition is valid only after matching its original operation and
// operands. This is not a register environment or a Phi equivalence class.
#[derive(Clone, Copy)]
struct DefinitionPair {
    normal: (crate::p5::LocalId, crate::p3::StateId),
    canonical: (crate::p5::LocalId, crate::p3::StateId),
}
// View compatibility is used only alongside an established state/definition
// correspondence or equal literal bits; it never establishes value equality.
fn same_view(a: crate::p4::Interpretation, b: crate::p4::Interpretation) -> bool {
    a == b
        || matches!(
            (a, b),
            (
                crate::p4::Interpretation::IntegralView { .. },
                crate::p4::Interpretation::IntegralView { .. }
            )
        )
}
fn same_read(a: StorageRead, b: StorageRead, definitions: &[DefinitionPair]) -> bool {
    if a == b {
        return true;
    }
    match (a, b) {
        (
            StorageRead::Value {
                local: al,
                state: a,
                view: av,
                ..
            },
            StorageRead::Value {
                local: bl,
                state: b,
                view: bv,
                ..
            },
        ) => {
            // IntegralView is producer-proved value preserving, not a
            // conversion. Its method-local declaration name ID is provenance,
            // not a different numeric read of the same established state.
            same_view(av, bv)
                && ((al, a) == (bl, b)
                    || definitions
                        .iter()
                        .any(|p| p.normal == (al, a) && p.canonical == (bl, b)))
        }
        (
            StorageRead::Literal {
                bits: a, view: av, ..
            },
            StorageRead::Literal {
                bits: b, view: bv, ..
            },
        ) => a == b && same_view(av, bv),
        _ => false,
    }
}
fn same_uses(
    a: &[crate::p5::UseAccess],
    b: &[crate::p5::UseAccess],
    definitions: &[DefinitionPair],
) -> bool {
    a.len() == b.len()
        && a.iter().zip(b).all(|(a, b)| {
            a.initialization() == b.initialization()
                && matches!((a.read(), b.read()), (Ok(a), Ok(b)) if same_read(a, b, definitions))
        })
}
fn same_operation(
    storage: &StorageMethod<'_>,
    f: &RegionFacts,
    normal: OpId,
    canonical: OpId,
    definitions: &mut Vec<DefinitionPair>,
) -> Result<bool, RegionError> {
    let flow = storage.execution();
    let a = &flow.operations()[normal.0 as usize];
    let b = &flow.operations()[canonical.0 as usize];
    if a.kind() != b.kind()
        || a.effects() != b.effects()
        || a.normal().commit() != b.normal().commit()
        || a.normal().heap() != b.normal().heap()
        || a.normal().lock() != b.normal().lock()
        || a.normal().read_order() != b.normal().read_order()
        || !operation_allowed(storage, f, normal)
        || !operation_allowed(storage, f, canonical)
    {
        return Ok(false);
    }
    match (a.exceptional(), b.exceptional()) {
        (Some(a), Some(b))
            if a.frame() == b.frame() && a.heap() == b.heap() && a.lock() == b.lock() => {}
        (None, None) => {}
        _ => return Ok(false),
    }
    let (Some(Ok(a)), Some(Ok(b))) = (
        storage.facts().operation_uses(normal),
        storage.facts().operation_uses(canonical),
    ) else {
        return Ok(false);
    };
    if !same_uses(a, b, definitions) {
        return Ok(false);
    }
    let (Some(a), Some(b)) = (
        storage.facts().operation_definitions(normal),
        storage.facts().operation_definitions(canonical),
    ) else {
        return Ok(false);
    };
    if a.len() != b.len() {
        return Ok(false);
    }
    for (a, b) in a.iter().zip(b) {
        match (a.materialization(), b.materialization()) {
            (Materialization::Store(al), Materialization::Store(bl)) => {
                if storage.facts().locals()[al.0 as usize].storage_type()
                    != storage.facts().locals()[bl.0 as usize].storage_type()
                {
                    return Ok(false);
                }
                push(
                    definitions,
                    DefinitionPair {
                        normal: (al, a.state()),
                        canonical: (bl, b.state()),
                    },
                )?;
            }
            (Materialization::Literal(a), Materialization::Literal(b)) if a == b => {}
            _ => return Ok(false),
        }
    }
    Ok(true)
}
fn private_definitions(
    storage: &StorageMethod<'_>,
    g: &Graph,
    definitions: &[DefinitionPair],
    normal: &[bool],
    canonical: &[bool],
) -> bool {
    for pair in definitions {
        for ((local, state), inside) in [(pair.normal, normal), (pair.canonical, canonical)] {
            // A literal-folded Phi incoming can have no executable CopyPlan.
            // It still observes this result; only the incoming identity names
            // that dependency, so copying/uses alone cannot prove it private.
            if storage
                .facts()
                .transfers()
                .iter()
                .any(|incoming| incoming.source() == state && incoming.destination_state() != state)
            {
                return false;
            }
            for u in storage.facts().uses() {
                match u.read() {
                    Err(_) => return false,
                    Ok(read) if read.local() == Some(local) => {
                        let node = match u.site() {
                            crate::p3::UseSite::Operation { operation, .. } => {
                                g.op_node[operation.0 as usize]
                            }
                            crate::p3::UseSite::Terminal { block, .. } => {
                                g.terminal_node[block.0 as usize]
                            }
                        };
                        if !inside[node.0 as usize]
                            || !matches!(read, StorageRead::Value { state: at, .. } if at == state)
                        {
                            return false;
                        }
                    }
                    _ => {}
                }
            }
            if storage.facts().copy_plans().iter().any(|p| {
                !p.is_complete()
                    || p.assignments().iter().any(|a| {
                        a.destination().ok() == Some(local)
                            || a.source().ok().and_then(|r| r.local()) == Some(local)
                    })
            }) {
                return false;
            }
        }
    }
    true
}
enum Work {
    Pair(Position, Position),
    Finish(Position),
}
pub(super) struct Matched {
    pub preparation: Vec<NodeId>,
    pub entry: NodeId,
    pub normal: Vec<NodeId>,
    pub operations: Vec<(OpId, OpId)>,
    pub predicates: Vec<(NodeId, NodeId)>,
    pub rethrow: NodeId,
    pub follow: NodeId,
}
pub(super) fn construct(
    storage: &StorageMethod<'_>,
    f: &RegionFacts,
    g: &Graph,
    plan: &TryPlan,
    capture: OpId,
    definition: &crate::p5::DefinitionPlan,
) -> Result<Option<Matched>, RegionError> {
    let Some(&handler) = plan.handlers.first() else {
        return Ok(None);
    };
    let Materialization::ExceptionCapture(local) = definition.materialization() else {
        return Ok(None);
    };
    let mut canonical_set = filled(f.nodes.len(), false)?;
    let mut normal_set = filled(f.nodes.len(), true)?;
    for &n in &plan.handler_bodies[0] {
        canonical_set[n.0 as usize] = true;
    }
    for &n in &plan.members {
        normal_set[n.0 as usize] = false;
    }
    for h in &f.handlers {
        normal_set[h.node.0 as usize] = false;
    }
    let mut canonical = Cursor {
        allowed: &canonical_set,
        visited: filled(f.nodes.len(), false)?,
        order: Vec::new(),
    };
    let Some((first, first_event)) = event(
        storage,
        f,
        g,
        &mut canonical,
        Position::entry(f, handler),
        Some(capture),
    )?
    else {
        return Ok(None);
    };
    let Some(preparation) = super::preparation::construct(storage, f, g, plan, |node| {
        if !normal_set[node.0 as usize] {
            return false;
        }
        let n = &f.nodes[node.0 as usize];
        match first_event {
            Event::Operation(operation) if !n.operations.is_empty() => {
                storage.execution().operations()[n.operations.start as usize].kind()
                    == storage.execution().operations()[operation.0 as usize].kind()
            }
            Event::Branch(op, _) if n.operations.is_empty() && n.terminal => matches!(
                storage.execution().blocks()[n.block.0 as usize].terminal(),
                Some(Terminal::Branch { op: normal, .. }) if *normal == op
            ),
            _ => false,
        }
    })?
    else {
        return Ok(None);
    };
    let normal_entry = preparation.entry;
    for node in &preparation.nodes {
        normal_set[node.0 as usize] = false;
    }
    let mut normal = Cursor {
        allowed: &normal_set,
        visited: filled(f.nodes.len(), false)?,
        order: Vec::new(),
    };
    let mut pairs: HashMap<Position, (Position, bool)> = HashMap::new();
    let mut reverse: HashMap<Position, Position> = HashMap::new();
    let mut work = Vec::new();
    let mut operations = Vec::new();
    let mut predicates = Vec::new();
    let mut definitions = Vec::new();
    let mut end = None;
    push(
        &mut work,
        Work::Pair(first, Position::entry(f, normal_entry)),
    )?;
    while let Some(task) = work.pop() {
        let (a, mut b) = match task {
            Work::Finish(a) => {
                let state = pairs.get_mut(&a).ok_or(bug("cleanup lost active pair"))?;
                state.1 = true;
                continue;
            }
            Work::Pair(a, b) => (a, b),
        };
        let Some((a, ae)) = event(storage, f, g, &mut canonical, a, Some(capture))? else {
            return Ok(None);
        };
        if matches!(ae, Event::Throw) {
            let n = &f.nodes[a.node.0 as usize];
            let Some([u]) = storage.facts().terminal_uses(n.block) else {
                return Ok(None);
            };
            if !matches!(u.read(), Ok(StorageRead::Value { local: l, state,
                view: crate::p4::Interpretation::Direct(crate::p4::Family::Reference), .. })
                if l == local && state == definition.state())
            {
                return Ok(None);
            }
            // No unpaired instruction is swallowed to align block cuts.
            let Some(continuation) = normal_continuation(storage, f, g, &mut normal, b)? else {
                return Ok(None);
            };
            b = continuation;
            if end.is_some_and(|old| old != (a.node, b.node)) {
                return Ok(None);
            }
            end = Some((a.node, b.node));
            continue;
        }
        let Some((b, be)) = event(storage, f, g, &mut normal, b, None)? else {
            return Ok(None);
        };
        if let Some(&(previous, complete)) = pairs.get(&a) {
            if previous != b || !complete {
                return Ok(None);
            }
            continue;
        }
        if reverse.get(&b).is_some_and(|&previous| previous != a) {
            return Ok(None);
        }
        pairs
            .try_reserve(1)
            .map_err(crate::RequestError::Allocation)?;
        reverse
            .try_reserve(1)
            .map_err(crate::RequestError::Allocation)?;
        pairs.insert(a, (b, false));
        reverse.insert(b, a);
        push(&mut work, Work::Finish(a))?;
        match (ae, be) {
            (Event::Operation(c), Event::Operation(n)) => {
                if !same_operation(storage, f, n, c, &mut definitions)? {
                    return Ok(None);
                }
                push(&mut operations, (n, c))?;
                push(&mut work, Work::Pair(a.after()?, b.after()?))?;
            }
            (Event::Branch(ac, aa), Event::Branch(bc, ba))
                if ac == bc
                    && matches!(
                        (storage.facts().terminal_uses(f.nodes[b.node.0 as usize].block),
                         storage.facts().terminal_uses(f.nodes[a.node.0 as usize].block)),
                        (Some(b), Some(a)) if !a.is_empty() && same_uses(b, a, &definitions)
                    ) =>
            {
                push(&mut predicates, (b.node, a.node))?;
                for i in (0..2).rev() {
                    push(
                        &mut work,
                        Work::Pair(Position::entry(f, aa[i]), Position::entry(f, ba[i])),
                    )?;
                }
            }
            _ => return Ok(None),
        }
    }
    let Some((rethrow, follow)) = end else {
        return Ok(None);
    };
    if operations.is_empty()
        || normal.visited[follow.0 as usize]
        || plan.members.contains(&follow)
        || plan.handler_bodies[0]
            .iter()
            .any(|n| !canonical.visited[n.0 as usize])
    {
        return Ok(None);
    }
    if !private_definitions(
        storage,
        g,
        &definitions,
        &normal.visited,
        &canonical.visited,
    ) {
        return Ok(None);
    }
    Ok(Some(Matched {
        preparation: preparation.nodes,
        entry: normal_entry,
        normal: normal.order,
        operations,
        predicates,
        rethrow,
        follow,
    }))
}
