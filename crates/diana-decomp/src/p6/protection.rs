mod frontiers;
mod loop_scope;
mod prefix;

use super::build::*;
use super::dominance::Structure;
use super::graph::Graph;
use super::*;
use crate::p2::{EdgeRole, EdgeTarget, OpKind};
use crate::p5::{CopyLocation, OperationEmission, StorageMethod, ThrowSite};

fn add(
    storage: &StorageMethod<'_>,
    f: &mut RegionFacts,
    g: &Graph,
    site: ThrowSite,
    node: NodeId,
    completion: crate::p2::ExceptionCompletion,
) -> Result<(), RegionError> {
    let mut clauses = Vec::new();
    let mut edges = Vec::new();
    for &id in &g.exceptions[node.0 as usize] {
        let edge = &storage.execution().edges()[id.0 as usize];
        if edge.source_op()
            != match site {
                ThrowSite::Operation(op) => Some(op),
                ThrowSite::Terminal(_) => None,
            }
        {
            continue;
        }
        let EdgeRole::Exception { order, catch, .. } = edge.role() else {
            return Err(bug("exception index contains normal edge"));
        };
        let target = match edge.target() {
            EdgeTarget::Block(b) => CatchTarget::Handler(Graph::entry(f, b)),
            EdgeTarget::Exit(_) => CatchTarget::Uncaught,
        };
        push(
            &mut clauses,
            HandlerClause {
                order,
                catch,
                target,
            },
        )?;
        push(&mut edges, id)?;
    }
    if edges.is_empty() {
        return Err(bug("throwing site lost its completion transfers"));
    }
    let chain = if let Some(c) = f.chains.iter().find(|c| c.clauses == clauses) {
        c.id
    } else {
        let id = ChainId(id(f.chains.len())?);
        push(&mut f.chains, HandlerChain { id, clauses })?;
        id
    };
    let index = ThrowSiteId(id(f.throws.len())?);
    for &e in &edges {
        let order = match storage.execution().edges()[e.0 as usize].role() {
            EdgeRole::Exception { order, .. } => order,
            _ => return Err(bug("throw edge changed role")),
        };
        f.edges[e.0 as usize].route = EdgeRoute::Exception {
            site: index,
            order,
            delivery: ExceptionDelivery::Explicit,
        };
    }
    let mut snapshots = Vec::new();
    for p in storage.facts().copy_plans() {
        if p.location() == CopyLocation::BeforeThrow(site) {
            push(&mut snapshots, p.id())?;
        }
    }
    let construction = match site {
        ThrowSite::Operation(op) => match storage.facts().operation_emission(op) {
            Some(OperationEmission::Construction { group, .. }) => Some(group),
            _ => None,
        },
        _ => None,
    };
    push(
        &mut f.throws,
        ThrowPlan {
            id: index,
            site,
            node,
            completion,
            chain,
            edges,
            snapshots,
            construction,
            protection: SiteProtection::Explicit,
        },
    )
}
pub(super) fn collect(
    storage: &StorageMethod<'_>,
    f: &mut RegionFacts,
    g: &Graph,
) -> Result<(), RegionError> {
    for op in storage.execution().operations() {
        if let Some(c) = op.exceptional() {
            add(
                storage,
                f,
                g,
                ThrowSite::Operation(op.id()),
                g.op_node[op.id().0 as usize],
                c,
            )?;
        }
    }
    for b in storage.execution().blocks() {
        if let Some(c) = b.exceptional() {
            add(
                storage,
                f,
                g,
                ThrowSite::Terminal(b.id()),
                g.terminal_node[b.id().0 as usize],
                c,
            )?;
        }
    }
    for edge in storage.execution().exception_edges() {
        if let EdgeTarget::Block(b) = edge.target() {
            let node = Graph::entry(f, b);
            if f.handlers.iter().any(|h| h.node == node) {
                continue;
            }
            let capture = storage.execution().blocks()[b.0 as usize]
                .operations()
                .next()
                .map(|op| &storage.execution().operations()[op as usize])
                .filter(|op| matches!(op.kind(), OpKind::MoveException))
                .map(|op| op.id());
            let mut receives = Vec::new();
            for p in storage.facts().copy_plans() {
                if matches!(p.location(),CopyLocation::HandlerEntry{block,..}if block==b) {
                    push(&mut receives, p.id())?;
                }
            }
            f.labels[node.0 as usize].kind = LabelKind::Handler;
            push(
                &mut f.handlers,
                HandlerEntry {
                    node,
                    capture,
                    receives,
                },
            )?;
        }
    }
    Ok(())
}
pub(super) fn handlers(chain: &HandlerChain) -> &[HandlerClause] {
    let end = chain
        .clauses
        .iter()
        .position(|c| c.target == CatchTarget::Uncaught)
        .unwrap_or(chain.clauses.len());
    &chain.clauses[..end]
}
pub(super) fn suffix(actual: &[HandlerClause], wanted: &[HandlerClause]) -> bool {
    actual.len() >= wanted.len()
        && actual[actual.len() - wanted.len()..]
            .iter()
            .zip(wanted)
            .all(|(a, b)| a.catch == b.catch && a.target == b.target)
}
/// Matches one unchanged ordered clause interval, without guessing hierarchy.
pub(super) fn matching_clauses(
    actual: &[HandlerClause],
    wanted: &[HandlerClause],
) -> Option<std::ops::Range<usize>> {
    if wanted.is_empty() || wanted.len() > actual.len() {
        return None;
    }
    let mut found = None;
    for (start, slice) in actual.windows(wanted.len()).enumerate() {
        if suffix(slice, wanted) {
            if found.is_some() {
                return None;
            }
            found = Some(start..start + wanted.len());
        }
    }
    found
}

pub(super) fn reachable(
    storage: &StorageMethod<'_>,
    f: &RegionFacts,
    g: &Graph,
    seeds: &[NodeId],
    stop: Option<NodeId>,
    excluded: &[bool],
    exceptional: bool,
) -> Result<Vec<bool>, RegionError> {
    let mut result = filled(f.nodes.len(), false)?;
    let mut queue = Vec::new();
    for &s in seeds {
        if Some(s) != stop && !excluded[s.0 as usize] && !result[s.0 as usize] {
            result[s.0 as usize] = true;
            push(&mut queue, s)?;
        }
    }
    while let Some(p) = queue.pop() {
        for &l in &g.outgoing[p.0 as usize] {
            let q = f.links[l.0 as usize].to;
            if Some(q) != stop && !excluded[q.0 as usize] && !result[q.0 as usize] {
                result[q.0 as usize] = true;
                push(&mut queue, q)?;
            }
        }
        if exceptional {
            for &e in &g.exceptions[p.0 as usize] {
                if Graph::exception_possible(f, e)
                    && let Some(q) = Graph::exception_target(f, storage, e)
                    && Some(q) != stop
                    && !excluded[q.0 as usize]
                    && !result[q.0 as usize]
                {
                    result[q.0 as usize] = true;
                    push(&mut queue, q)?;
                }
            }
        }
    }
    Ok(result)
}
fn list(set: &[bool]) -> Result<Vec<NodeId>, RegionError> {
    let mut result = Vec::new();
    for (i, &inside) in set.iter().enumerate() {
        if inside {
            push(&mut result, NodeId(id(i)?))?;
        }
    }
    Ok(result)
}
pub(super) fn closed_entry(
    storage: &StorageMethod<'_>,
    f: &RegionFacts,
    g: &Graph,
    set: &[bool],
    entry: NodeId,
) -> bool {
    if g.external_entry(f, set, entry) {
        return false;
    }
    for (from, edges) in g.exceptions.iter().enumerate() {
        if set[from] {
            continue;
        }
        for &e in edges {
            if Graph::exception_possible(f, e)
                && let Some(to) = Graph::exception_target(f, storage, e)
                && set[to.0 as usize]
                && to != entry
            {
                return false;
            }
        }
    }
    true
}

// A scoped abrupt outlet does not constrain a shared normal continuation. This
// filters follow candidates only; region checks still see every original edge.
fn normal_follow_roots(
    f: &RegionFacts,
    g: &Graph,
    scope: SyncId,
    roots: &[NodeId],
) -> Result<Vec<NodeId>, RegionError> {
    let p = &f.synchronized[scope.0 as usize];
    let mut body = filled(f.nodes.len(), false)?;
    for &node in &p.body {
        body[node.0 as usize] = true;
    }
    let mut reaches = filled(f.nodes.len(), false)?;
    let mut queue = Vec::new();
    for exit in &p.exits {
        if matches!(exit.kind, SyncExitKind::Follow(_)) {
            let n = g.op_node[exit.release.0 as usize];
            reaches[n.0 as usize] = true;
            push(&mut queue, n)?;
        }
    }
    while let Some(node) = queue.pop() {
        for &link in &g.incoming[node.0 as usize] {
            let from = f.links[link.0 as usize].from;
            if !reaches[from.0 as usize] && body[from.0 as usize] {
                reaches[from.0 as usize] = true;
                push(&mut queue, from)?;
            }
        }
    }
    let mut selected = Vec::new();
    for &node in roots {
        if reaches[node.0 as usize] {
            push(&mut selected, node)?;
        }
    }
    Ok(selected)
}
// Foreign normal or exceptional arrivals mark shared content, not a private
// catch continuation. In particular an outer cleanup handler can also receive
// failures from the normal body. Nested private exception paths stay inside.
fn handler_frontiers(
    storage: &StorageMethod<'_>,
    f: &RegionFacts,
    g: &Graph,
) -> Result<Vec<Vec<NodeId>>, RegionError> {
    let mut frontiers = lists(f.nodes.len())?;
    let excluded = filled(f.nodes.len(), false)?;
    let mut shared = filled(f.nodes.len(), false)?;
    for handler in &f.handlers {
        shared.fill(false);
        let reach = reachable(storage, f, g, &[handler.node], None, &excluded, true)?;
        for node in &f.nodes {
            if node.id == handler.node || !reach[node.id.0 as usize] {
                continue;
            }
            if g.incoming[node.id.0 as usize]
                .iter()
                .any(|link| !reach[f.links[link.0 as usize].from.0 as usize])
            {
                push(&mut frontiers[handler.node.0 as usize], node.id)?;
                shared[node.id.0 as usize] = true;
            }
        }
        for (from, edges) in g.exceptions.iter().enumerate() {
            if reach[from] {
                continue;
            }
            for &edge in edges {
                if Graph::exception_possible(f, edge)
                    && let Some(to) = Graph::exception_target(f, storage, edge)
                    && to != handler.node
                    && reach[to.0 as usize]
                    && !shared[to.0 as usize]
                {
                    push(&mut frontiers[handler.node.0 as usize], to)?;
                    shared[to.0 as usize] = true;
                }
            }
        }
    }
    Ok(frontiers)
}
// A common handler continuation can carry one original capture local from
// distinct exception contexts. Its guarded sites need the complete context,
// not a follow derived only from the other seeds' normal successors. Distinct
// live primary/secondary locals are NOT this relationship; no captures merge.
fn shared_capture_contexts(
    storage: &StorageMethod<'_>,
    f: &RegionFacts,
    frontiers: &[Vec<NodeId>],
) -> Result<Vec<bool>, RegionError> {
    let mut locals = lists(f.nodes.len())?;
    let mut shared = filled(f.nodes.len(), false)?;
    for handler in &f.handlers {
        let Some(capture) = handler.capture else {
            continue;
        };
        let Some(local) = storage.facts().definitions().iter().find_map(|definition| {
            if definition.operation() != capture {
                return None;
            }
            match definition.materialization() {
                crate::p5::Materialization::ExceptionCapture(local) => Some(local),
                _ => None,
            }
        }) else {
            continue;
        };
        for &node in &frontiers[handler.node.0 as usize] {
            let seen = &mut locals[node.0 as usize];
            if seen.contains(&local) {
                shared[node.0 as usize] = true;
            } else {
                push(seen, local)?;
            }
        }
    }
    Ok(shared)
}
fn guarded_site(
    f: &RegionFacts,
    site: &ThrowPlan,
    wanted: &[HandlerClause],
    inside: impl Fn(NodeId) -> bool,
) -> bool {
    site.edges.iter().all(|edge| {
        matches!(
            f.edges[edge.0 as usize].route,
            EdgeRoute::Exception {
                delivery: ExceptionDelivery::ImpossibleRelease(_),
                ..
            }
        )
    }) || matching_clauses(handlers(&f.chains[site.chain.0 as usize]), wanted).is_some()
        || f.chains[site.chain.0 as usize]
            .clauses
            .iter()
            .all(|clause| matches!(clause.target, CatchTarget::Handler(node) if inside(node)))
}
fn guarded(f: &RegionFacts, wanted: &[HandlerClause], inside: impl Fn(NodeId) -> bool) -> bool {
    f.throws
        .iter()
        .filter(|site| inside(site.node))
        .all(|site| guarded_site(f, site, wanted, &inside))
}

struct Pattern {
    chain: ChainId,
    start: usize,
    end: usize,
    outer: Option<SyncId>,
}
pub(super) fn plan(
    storage: &StorageMethod<'_>,
    f: &mut RegionFacts,
    g: &Graph,
    s: &Structure,
) -> Result<(), RegionError> {
    let handler_frontiers = handler_frontiers(storage, f, g)?;
    let shared_contexts = shared_capture_contexts(storage, f, &handler_frontiers)?;
    let mut abrupt = filled(f.nodes.len(), false)?;
    for handler in &f.handlers {
        abrupt[handler.node.0 as usize] = loop_scope::abrupt(storage, f, g, s, handler.node);
    }
    let mut patterns: Vec<Pattern> = Vec::new();
    for context in 0..=f.synchronized.len() {
        let outer = if context == 0 {
            None
        } else {
            Some(SyncId(id(context - 1)?))
        };
        if outer.is_some_and(|id| f.synchronized[id.0 as usize].separation.is_some()) {
            continue;
        }
        for t in &f.throws {
            if outer.is_some_and(|id| !f.synchronized[id.0 as usize].body.contains(&t.node)) {
                continue;
            }
            let clauses = handlers(&f.chains[t.chain.0 as usize]);
            // An observed proper suffix supplies an enclosing protection layer.
            // Use its longest match as the immediate outer layer, not an arbitrary
            // search over all possible clause partitions.
            let inherited = f
                .chains
                .iter()
                .map(handlers)
                .filter(|outer| {
                    !outer.is_empty() && outer.len() < clauses.len() && suffix(clauses, outer)
                })
                .map(|outer| outer.len())
                .max();
            for end in std::iter::once(clauses.len()).chain(inherited.map(|n| clauses.len() - n)) {
                for start in 0..end {
                    let wanted = &clauses[start..end];
                    if patterns.iter().any(|p| {
                        p.outer == outer
                            && p.end - p.start == wanted.len()
                            && suffix(
                                &f.chains[p.chain.0 as usize].clauses[p.start..p.end],
                                wanted,
                            )
                    }) {
                        continue;
                    }
                    push(
                        &mut patterns,
                        Pattern {
                            chain: t.chain,
                            start,
                            end,
                            outer,
                        },
                    )?;
                }
            }
        }
    }
    for pattern in patterns {
        let Pattern {
            chain,
            start,
            end,
            outer,
        } = pattern;
        let wanted = &f.chains[chain.0 as usize].clauses[start..end];
        let mut targets = Vec::new();
        let mut handler_clauses: Vec<std::ops::Range<u32>> = Vec::new();
        let mut repeated = false;
        for (offset, clause) in wanted.iter().enumerate() {
            let CatchTarget::Handler(target) = clause.target else {
                continue;
            };
            let at = id(start + offset)?;
            if targets.last() == Some(&target)
                && clause.catch.is_some()
                && offset > 0
                && wanted[offset - 1].catch.is_some()
            {
                let group = handler_clauses
                    .last_mut()
                    .ok_or(bug("catch group lost its clauses"))?;
                group.end = id(start + offset + 1)?;
                continue;
            }
            // Nonadjacent aliases cannot be reordered without hierarchy facts;
            // a suffix cannot acquire a handler also used by its outer prefix.
            if targets.contains(&target)
                || f.chains[chain.0 as usize].clauses[..start]
                    .iter()
                    .chain(&f.chains[chain.0 as usize].clauses[end..])
                    .any(|c| c.target == clause.target)
            {
                repeated = true;
                break;
            }
            push(&mut targets, target)?;
            push(&mut handler_clauses, at..id(start + offset + 1)?)?;
        }
        if repeated || targets.is_empty() {
            continue;
        }
        let mut seeds = Vec::new();
        for t in &f.throws {
            if outer.is_some_and(|id| !f.synchronized[id.0 as usize].body.contains(&t.node)) {
                continue;
            }
            if matching_clauses(handlers(&f.chains[t.chain.0 as usize]), wanted).is_some()
                && !seeds.contains(&t.node)
            {
                push(&mut seeds, t.node)?;
            }
        }
        if seeds.is_empty() {
            continue;
        }
        // Bounded scope frontiers must not absorb unlocked continuation work.
        if f.synchronized.iter().any(|sync| {
            sync.separation.is_none()
                && outer.is_none_or(|parent| {
                    sync.members.len() < f.synchronized[parent.0 as usize].members.len()
                })
                && targets.iter().all(|n| sync.body.contains(n))
                && seeds.iter().all(|n| sync.body.contains(n))
        }) {
            continue;
        }
        // A follow is after each guarded node, never the throwing seed itself.
        // Abrupt handlers can leave just one normal completion constraint.
        let mut follows = Vec::new();
        for &seed in &seeds {
            for &link in &g.outgoing[seed.0 as usize] {
                let next = f.links[link.0 as usize].to;
                if !seeds.contains(&next) && !follows.contains(&next) {
                    push(&mut follows, next)?;
                }
            }
        }
        let normal_follow = s.common_follow(&follows);
        let handler_follows = outer
            .map(|scope| normal_follow_roots(f, g, scope, &targets))
            .transpose()?;
        for &target in handler_follows.as_deref().unwrap_or(&targets) {
            if !follows.contains(&target) {
                push(&mut follows, target)?;
            }
        }
        // A returning or throwing handler need not share the body's normal
        // continuation. Membership and entrance checks below still prove the
        // entire candidate around this boundary.
        let mut follow = s.common_follow(&follows).or(normal_follow);
        let mut abrupt_handlers = targets.iter().all(|root| abrupt[root.0 as usize]);
        let mut excluded = filled(f.nodes.len(), false)?;
        if let Some(scope) = outer {
            for (i, slot) in excluded.iter_mut().enumerate() {
                *slot = !f.synchronized[scope.0 as usize]
                    .body
                    .contains(&NodeId(id(i)?));
            }
        }
        for clause in &handlers(&f.chains[chain.0 as usize])[start..] {
            if let CatchTarget::Handler(target) = clause.target {
                excluded[target.0 as usize] = true;
            }
        }
        // An observed, already established outer suffix supplies the private
        // boundary for both this body and its handlers. Never acquire its
        // unguarded completion suffix through a handler's normal reach.
        let enclosing = f
            .tries
            .iter()
            .filter(|plan| {
                plan.outer_sync == outer
                    && seeds.iter().all(|node| plan.body.contains(node))
                    && targets.iter().all(|node| plan.body.contains(node))
                    && matching_clauses(
                        &f.chains[chain.0 as usize].clauses[end..],
                        &f.chains[plan.chain.0 as usize].clauses
                            [plan.clause_range.start as usize..plan.clause_range.end as usize],
                    )
                    .is_some()
            })
            .min_by_key(|plan| plan.body.len())
            .map(|plan| plan.body.as_slice());
        if let Some(body) = enclosing {
            for (index, slot) in excluded.iter_mut().enumerate() {
                *slot |= !body.contains(&NodeId(id(index)?));
            }
        }
        let completion = if outer.is_none() && abrupt_handlers {
            loop_scope::complete(storage, f, g, &seeds, wanted, &excluded, follow)?
        } else {
            None
        };
        let (mut body, loop_entry) = match completion {
            Some(completion) => {
                follow = Some(completion.follow);
                (completion.body, Some(completion.entry))
            }
            None => {
                if outer.is_none()
                    && wanted.len() == 1
                    && wanted[0].catch.is_none()
                    && (follow.is_none()
                        || seeds.iter().any(|node| shared_contexts[node.0 as usize]))
                {
                    let Some(frontier) = frontiers::complete(
                        storage,
                        f,
                        g,
                        &seeds,
                        wanted,
                        &excluded,
                        &handler_frontiers[targets[0].0 as usize],
                    )?
                    else {
                        continue;
                    };
                    follow = frontier.follow;
                    (frontier.body, None)
                } else {
                    (
                        reachable(storage, f, g, &seeds, follow, &excluded, true)?,
                        None,
                    )
                }
            }
        };
        // A completed loop already has its proved header entrance. Optional
        // prefixes must not acquire initialization preceding that whole scope.
        let Some(entry) = (match loop_entry {
            Some(entry) => Some(entry),
            None => prefix::complete(storage, f, g, &mut body, follow, &excluded)?,
        }) else {
            continue;
        };
        if !guarded(f, wanted, |n| body[n.0 as usize]) {
            continue;
        }
        let mut members = body;
        let body_nodes = list(&members)?;
        let mut valid = true;
        let mut handler_bodies = Vec::new();
        for &target in &targets {
            let mut exclude = filled(f.nodes.len(), false)?;
            if let Some(scope) = outer {
                for (i, slot) in exclude.iter_mut().enumerate() {
                    *slot = !f.synchronized[scope.0 as usize]
                        .body
                        .contains(&NodeId(id(i)?));
                }
            }
            if let Some(body) = enclosing {
                for (index, slot) in exclude.iter_mut().enumerate() {
                    *slot |= !body.contains(&NodeId(id(index)?));
                }
            }
            for (i, &inside) in members.iter().enumerate() {
                exclude[i] |= inside;
            }
            for &other in &targets {
                if other != target {
                    exclude[other.0 as usize] = true;
                }
            }
            // Later clauses belong to the observed outer protection layer.
            // A loop can make its cleanup reachable again from this handler;
            // that does not make the outer capture private catch content.
            for clause in &handlers(&f.chains[chain.0 as usize])[end..] {
                if let CatchTarget::Handler(outer) = clause.target {
                    exclude[outer.0 as usize] = true;
                }
            }
            for &frontier in &handler_frontiers[target.0 as usize] {
                exclude[frontier.0 as usize] = true;
            }
            let arm = reachable(storage, f, g, &[target], follow, &exclude, true)?;
            if !arm[target.0 as usize]
                || f.throws.iter().any(|t| {
                    arm[t.node.0 as usize]
                        && matching_clauses(handlers(&f.chains[t.chain.0 as usize]), wanted)
                            .is_some()
                })
            {
                valid = false;
                break;
            }
            push(&mut handler_bodies, list(&arm)?)?;
            for (i, &inside) in arm.iter().enumerate() {
                members[i] |= inside;
            }
        }
        if valid {
            for (&target, nodes) in targets.iter().zip(&handler_bodies) {
                let mut arm = filled(f.nodes.len(), false)?;
                for node in nodes {
                    arm[node.0 as usize] = true;
                    let n = &f.nodes[node.0 as usize];
                    if g.outgoing[node.0 as usize].is_empty()
                        && !(n.terminal
                            && matches!(
                                storage.execution().blocks()[n.block.0 as usize].terminal(),
                                Some(
                                    crate::p2::Terminal::Return { .. }
                                        | crate::p2::Terminal::Throw { .. }
                                )
                            ))
                    {
                        abrupt_handlers = false;
                    }
                }
                for l in &f.links {
                    // Include private nested exceptional paths in the published
                    // proof, not only paths dominated by the capture root.
                    if arm[l.from.0 as usize] && !arm[l.to.0 as usize] {
                        abrupt_handlers = false;
                    }
                    if arm[l.to.0 as usize] && (l.to == target || !arm[l.from.0 as usize]) {
                        valid = false;
                        break;
                    }
                }
                for (from, edges) in g.exceptions.iter().enumerate() {
                    for &e in edges {
                        if Graph::exception_possible(f, e)
                            && let Some(to) = Graph::exception_target(f, storage, e)
                            && arm[to.0 as usize]
                            && (to == target || !arm[from])
                            && !(to == target && body_nodes.contains(&NodeId(id(from)?)))
                        {
                            valid = false;
                            break;
                        }
                    }
                    if !valid {
                        break;
                    }
                }
                if !valid {
                    break;
                }
            }
        }
        if !valid
            || (loop_entry.is_some() && !abrupt_handlers)
            || !closed_entry(storage, f, g, &members, entry)
        {
            continue;
        }
        let plan = TryPlan {
            id: TryId(id(f.tries.len())?),
            chain,
            clause_range: id(start)?..id(end)?,
            outer_sync: outer,
            entry,
            body: body_nodes,
            handlers: targets,
            handler_bodies,
            handler_clauses,
            abrupt_handlers,
            bindings: Vec::new(),
            finally: None,
            members: list(&members)?,
            follow,
        };
        push(&mut f.tries, plan)?;
    }
    Ok(())
}
