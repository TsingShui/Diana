//! Input-local declaration graph. Paths are witnesses, not runtime dispatch.

use super::declarations::{self, Location};
use super::{
    Event, FilterMatch, Issue, Matched, MethodRelation, QueryError, Relation, Site, Target, Xref,
    copy,
};
use crate::analysis::{at, dex, input::InputError};
use diana_decomp::p0::{ClassFacts, Fact};
use std::collections::{HashMap, HashSet, VecDeque};

type AncestorPaths = Vec<(Vec<u16>, Vec<usize>)>;

struct Node {
    location: Location,
    facts: ClassFacts,
}
struct Graph {
    nodes: Vec<Node>,
    names: HashMap<Vec<u16>, Vec<usize>>,
    gaps: HashSet<String>,
}

impl Graph {
    fn gap(
        &mut self,
        context: String,
        reason: String,
        emit: &mut dyn FnMut(Event) -> Result<(), QueryError>,
    ) -> Result<(), QueryError> {
        let key = format!("{context}: {reason}");
        if self.gaps.contains(&key) {
            return Ok(());
        }
        self.gaps.try_reserve(1).map_err(InputError::Allocation)?;
        self.gaps.insert(key);
        emit(Event::Issue(Issue {
            context,
            kind: "Unknown".to_owned(),
            reason,
            range_bytes: None,
        }))
    }
    fn resolve(
        &mut self,
        descriptor: &[u16],
        emit: &mut dyn FnMut(Event) -> Result<(), QueryError>,
    ) -> Result<Option<usize>, QueryError> {
        match self.names.get(descriptor).map(Vec::as_slice) {
            Some([index]) => Ok(Some(*index)),
            found => {
                let reason = if found.is_some() {
                    "ambiguous duplicate definitions"
                } else {
                    "declaration absent from input"
                };
                self.gap(
                    format!("type:{}", at::escape_units(descriptor)),
                    reason.to_owned(),
                    emit,
                )?;
                Ok(None)
            }
        }
    }
    fn edges(
        &mut self,
        index: usize,
        interfaces: bool,
        emit: &mut dyn FnMut(Event) -> Result<(), QueryError>,
    ) -> Result<Vec<Vec<u16>>, QueryError> {
        let node = &self.nodes[index];
        let mut edges = Vec::new();
        let mut unavailable = false;
        match node.facts.superclass() {
            Fact::Present(value) => {
                edges.try_reserve(1).map_err(InputError::Allocation)?;
                edges.push(copy(value.units())?);
            }
            Fact::Unavailable(_) | Fact::NotRead => unavailable = true,
            Fact::Absent => (),
        }
        if interfaces {
            match node.facts.interfaces() {
                Fact::Present(values) => {
                    for value in values {
                        match &value.descriptor {
                            Fact::Present(value) => {
                                edges.try_reserve(1).map_err(InputError::Allocation)?;
                                edges.push(copy(value.units())?);
                            }
                            _ => unavailable = true,
                        }
                    }
                }
                Fact::Unavailable(_) | Fact::NotRead => unavailable = true,
                Fact::Absent => (),
            }
        }
        if unavailable {
            self.gap(
                self.nodes[index].location.context(),
                "hierarchy facts unavailable".to_owned(),
                emit,
            )?;
        }
        Ok(edges)
    }
    fn paths(
        &mut self,
        start: usize,
        interfaces: bool,
        emit: &mut dyn FnMut(Event) -> Result<(), QueryError>,
    ) -> Result<AncestorPaths, QueryError> {
        let mut result = Vec::new();
        let mut queue = VecDeque::new();
        queue.try_reserve(1).map_err(InputError::Allocation)?;
        queue.push_back((start, vec![start]));
        let mut visited = HashSet::new();
        while let Some((index, path)) = queue.pop_front() {
            if visited.contains(&index) {
                continue;
            }
            visited.try_reserve(1).map_err(InputError::Allocation)?;
            visited.insert(index);
            for descriptor in self.edges(index, interfaces, emit)? {
                result.try_reserve(1).map_err(InputError::Allocation)?;
                let mut witness = Vec::new();
                witness
                    .try_reserve(path.len())
                    .map_err(InputError::Allocation)?;
                witness.extend_from_slice(&path);
                result.push((copy(&descriptor)?, witness));
                if let Some(next) = self.resolve(&descriptor, emit)? {
                    if path.contains(&next) {
                        self.gap(
                            self.nodes[next].location.context(),
                            "cyclic hierarchy path".to_owned(),
                            emit,
                        )?;
                        continue;
                    }
                    let mut next_path = Vec::new();
                    next_path
                        .try_reserve(path.len().saturating_add(1))
                        .map_err(InputError::Allocation)?;
                    next_path.extend_from_slice(&path);
                    next_path.push(next);
                    queue.try_reserve(1).map_err(InputError::Allocation)?;
                    queue.push_back((next, next_path));
                }
            }
        }
        Ok(result)
    }
    fn path_label(&self, path: &[usize]) -> String {
        path.iter()
            .map(|&index| at::escape_units(&self.nodes[index].location.descriptor))
            .collect::<Vec<_>>()
            .join(" → ")
    }
    fn same_package(left: &[u16], right: &[u16]) -> bool {
        let package = |value: &[u16]| {
            value
                .iter()
                .rposition(|&unit| unit == u16::from(b'/'))
                .unwrap_or(0)
        };
        left[..package(left)] == right[..package(right)]
    }
    fn returns_match(
        &mut self,
        child: &[u16],
        parent: &[u16],
        emit: &mut dyn FnMut(Event) -> Result<(), QueryError>,
    ) -> Result<bool, QueryError> {
        if child == parent {
            return Ok(true);
        }
        if child.first() != Some(&u16::from(b'L')) || parent.first() != Some(&u16::from(b'L')) {
            // Arrays with unequal return descriptors require component evidence;
            // do not guess covariance from absent component declarations.
            if child.first() == Some(&u16::from(b'[')) || parent.first() == Some(&u16::from(b'[')) {
                self.gap(
                    "method return".to_owned(),
                    "array covariance not established for unequal descriptors".to_owned(),
                    emit,
                )?;
            }
            return Ok(false);
        }
        let Some(child) = self.resolve(child, emit)? else {
            return Ok(false);
        };
        Ok(self
            .paths(child, true, emit)?
            .iter()
            .any(|(descriptor, _)| descriptor == parent))
    }
    fn signature_match(
        &mut self,
        child: &Signature,
        parent: &Signature,
        emit: &mut dyn FnMut(Event) -> Result<(), QueryError>,
    ) -> Result<bool, QueryError> {
        if child.name_units() != parent.name_units() || child.parameters != parent.parameters {
            return Ok(false);
        }
        self.returns_match(&child.returns, &parent.returns, emit)
    }
}

pub(super) fn run(
    query: &Xref<'_>,
    emit: &mut dyn FnMut(Event) -> Result<(), QueryError>,
) -> Result<(), QueryError> {
    let mut graph = Graph {
        nodes: Vec::new(),
        names: HashMap::new(),
        gaps: HashSet::new(),
    };
    for (image, root) in query.roots.iter().enumerate() {
        let stopped = std::cell::Cell::new(false);
        let mut forward = |event| {
            let result = emit(event);
            if result.is_err() {
                stopped.set(true);
            }
            result
        };
        let result = root.bytes().map_err(QueryError::from).and_then(|bytes| {
            dex::visit_logical(bytes, |logical, index| {
                for ordinal in 0..index.class_count() {
                    let class_def = diana_decomp::ClassDefId(ordinal);
                    match declarations::collect(&index, class_def) {
                        Ok(facts) => {
                            let descriptor = copy(facts.descriptor().units())?;
                            let at = graph.nodes.len();
                            graph.names.try_reserve(1).map_err(InputError::Allocation)?;
                            let slots = graph.names.entry(copy(&descriptor)?).or_default();
                            slots.try_reserve(1).map_err(InputError::Allocation)?;
                            slots.push(at);
                            graph.nodes.try_reserve(1).map_err(InputError::Allocation)?;
                            graph.nodes.push(Node {
                                location: Location {
                                    image: dex::ImageIndex(image),
                                    logical,
                                    class_def,
                                    descriptor,
                                },
                                facts,
                            });
                        }
                        Err(error) => forward(Event::Issue(Issue::new(
                            format!("image:{image}:l{}:c{ordinal}", logical.0),
                            &error,
                            None,
                        )))?,
                    }
                }
                Ok(())
            })
        });
        if let Err(error) = result {
            if stopped.get() {
                return Err(error);
            }
            forward(Event::Issue(Issue::new(
                format!("image:{image}"),
                &error,
                None,
            )))?;
        }
    }
    if query.plan.method_relation != MethodRelation::References {
        return method_relations(&mut graph, query, emit);
    }
    for index in 0..graph.nodes.len() {
        if query
            .plan
            .class
            .as_deref()
            .is_some_and(|filter| filter != graph.nodes[index].location.descriptor)
        {
            continue;
        }
        let extend_only = query.plan.targets.iter().all(|target| {
            matches!(
                target,
                Target::Hierarchy {
                    relation: Relation::Extend,
                    ..
                }
            )
        });
        let paths = graph.paths(index, !extend_only, emit)?;
        let mut delivered = HashSet::new();
        for (descriptor, path) in paths {
            for target in &query.plan.targets {
                let Target::Hierarchy { relation, target } = target else {
                    continue;
                };
                let Some(selected) = target.evaluate_units(&descriptor)? else {
                    continue;
                };
                if *relation == Relation::Impl {
                    let Some(parent) = graph.names.get(&descriptor).and_then(|values| {
                        if values.len() == 1 {
                            values.first().copied()
                        } else {
                            None
                        }
                    }) else {
                        // A direct interface entry is evidence without a target
                        // definition. For indirect edges, require an interface
                        // edge on the witness rather than guessing node kind.
                        if !path_has_interface(&graph, &path, &descriptor) {
                            continue;
                        }
                        let key = copy(&descriptor)?;
                        delivered.try_reserve(1).map_err(InputError::Allocation)?;
                        if delivered.insert(key) {
                            emit_relation(
                                &graph,
                                index,
                                &descriptor,
                                &path,
                                *relation,
                                selected,
                                emit,
                            )?;
                        }
                        continue;
                    };
                    if graph.nodes[parent].facts.access_flags().raw() & 0x200 == 0 {
                        continue;
                    }
                }
                delivered.try_reserve(1).map_err(InputError::Allocation)?;
                if delivered.insert(copy(&descriptor)?) {
                    emit_relation(&graph, index, &descriptor, &path, *relation, selected, emit)?;
                }
            }
        }
    }
    Ok(())
}

fn path_has_interface(graph: &Graph, path: &[usize], descriptor: &[u16]) -> bool {
    path.last().is_some_and(|&index| match graph.nodes[index].facts.interfaces() {
        Fact::Present(values) => values.iter().any(|value| matches!(&value.descriptor, Fact::Present(value) if value.units() == descriptor)), _ => false,
    })
}
fn emit_relation(
    graph: &Graph,
    index: usize,
    descriptor: &[u16],
    path: &[usize],
    relation: Relation,
    selected: FilterMatch,
    emit: &mut dyn FnMut(Event) -> Result<(), QueryError>,
) -> Result<(), QueryError> {
    let direct = path.len() == 1;
    let label = match relation {
        Relation::Extend => "extends",
        Relation::Impl => {
            if graph.nodes[index].facts.access_flags().raw() & 0x200 != 0 {
                "extends-interface"
            } else {
                "implements"
            }
        }
    };
    let use_ = if direct {
        label.to_owned()
    } else {
        format!("{label}-indirect via {}", graph.path_label(&path[1..]))
    };
    let depth = u32::try_from(path.len()).map_err(|_| InputError::PlatformCapacity)?;
    emit(graph.nodes[index].location.row(
        Site::Class,
        Matched::Type(copy(descriptor)?),
        selected,
        use_,
        depth,
    )?)
}

struct Signature {
    name: Vec<u16>,
    prototype: Vec<u16>,
    parameters: Vec<Vec<u16>>,
    returns: Vec<u16>,
}
impl Signature {
    fn name_units(&self) -> &[u16] {
        &self.name
    }
    fn prototype_units(&self) -> &[u16] {
        &self.prototype
    }
}
struct MethodFact {
    owner: usize,
    id: diana_decomp::MethodId,
    flags: u32,
    declaration: Signature,
}
fn owned_methods(graph: &Graph) -> Result<Vec<MethodFact>, QueryError> {
    let mut methods = Vec::new();
    for (owner, node) in graph.nodes.iter().enumerate() {
        for entry in node.facts.methods() {
            let Fact::Present(declaration) = entry.declaration() else {
                continue;
            };
            if entry.access_flags().raw() & (0x2 | 0x8) != 0
                || declaration.name_units().first() == Some(&u16::from(b'<'))
            {
                continue;
            }
            // Serialize-free, fallible copies of original declaration leaves.
            let mut parameters = Vec::new();
            parameters
                .try_reserve(declaration.parameter_types.len())
                .map_err(InputError::Allocation)?;
            for value in &declaration.parameter_types {
                parameters.push(copy(value.units())?);
            }
            methods.try_reserve(1).map_err(InputError::Allocation)?;
            methods.push(MethodFact {
                owner,
                id: entry.method(),
                flags: entry.access_flags().raw(),
                declaration: Signature {
                    name: copy(declaration.name_units())?,
                    prototype: copy(declaration.prototype_units())?,
                    parameters,
                    returns: copy(declaration.return_type.units())?,
                },
            });
        }
    }
    Ok(methods)
}

fn method_relations(
    graph: &mut Graph,
    query: &Xref<'_>,
    emit: &mut dyn FnMut(Event) -> Result<(), QueryError>,
) -> Result<(), QueryError> {
    let methods = owned_methods(graph)?;
    let mut by_name: HashMap<&[u16], Vec<usize>> = HashMap::new();
    for (index, method) in methods.iter().enumerate() {
        by_name.try_reserve(1).map_err(InputError::Allocation)?;
        let entry = by_name.entry(method.declaration.name_units()).or_default();
        entry.try_reserve(1).map_err(InputError::Allocation)?;
        entry.push(index);
    }
    for target in &methods {
        let interface = graph.nodes[target.owner].facts.access_flags().raw() & 0x200 != 0;
        if (query.plan.method_relation == MethodRelation::Implementations) != interface {
            continue;
        }
        if !interface && target.flags & 0x10 != 0 {
            continue;
        }
        let value = Matched::Method {
            owner: copy(&graph.nodes[target.owner].location.descriptor)?,
            name: copy(target.declaration.name_units())?,
            prototype: copy(target.declaration.prototype_units())?,
        };
        let Some((_, selected)) = declarations::matched_value(value, &query.plan.targets)? else {
            continue;
        };
        for class in 0..graph.nodes.len() {
            if query
                .plan
                .class
                .as_deref()
                .is_some_and(|filter| filter != graph.nodes[class].location.descriptor)
            {
                continue;
            }
            let paths = graph.paths(class, interface, emit)?;
            let target_path = paths.iter().find(|(descriptor, _)| {
                descriptor == &graph.nodes[target.owner].location.descriptor
            });
            let Some((_, witness)) = target_path else {
                continue;
            };
            let target_public = target.flags & (0x1 | 0x4) != 0;
            if !target_public
                && !witness.iter().all(|&index| {
                    Graph::same_package(
                        &graph.nodes[index].location.descriptor,
                        &graph.nodes[target.owner].location.descriptor,
                    )
                })
            {
                graph.gap(
                    graph.nodes[class].location.context(),
                    "package-private inheritance across package boundary is not established"
                        .to_owned(),
                    emit,
                )?;
                continue;
            }
            let mut owners = HashMap::new();
            owners
                .try_reserve(paths.len().saturating_add(1))
                .map_err(InputError::Allocation)?;
            owners.insert(class, 0usize);
            for (descriptor, path) in &paths {
                if let Some(indices) = graph.names.get(descriptor)
                    && let [index] = indices.as_slice()
                {
                    owners.entry(*index).or_insert(path.len());
                }
            }
            let mut candidates = Vec::new();
            for &candidate_index in by_name
                .get(target.declaration.name_units())
                .into_iter()
                .flatten()
            {
                let candidate = &methods[candidate_index];
                if query.plan.method_relation == MethodRelation::Overrides
                    && candidate.owner != class
                {
                    continue;
                }
                let Some(&distance) = owners.get(&candidate.owner) else {
                    continue;
                };
                if candidate.owner == target.owner && candidate.id == target.id {
                    continue;
                }
                if interface && candidate.flags & 0x400 != 0 {
                    continue;
                }
                if !graph.signature_match(&candidate.declaration, &target.declaration, emit)? {
                    continue;
                }
                let owner_interface =
                    graph.nodes[candidate.owner].facts.access_flags().raw() & 0x200 != 0;
                candidates.try_reserve(1).map_err(InputError::Allocation)?;
                candidates.push((owner_interface, distance, candidate));
            }
            candidates.sort_unstable_by_key(|(interface, distance, _)| (*interface, *distance));
            if let Some((owner_interface, distance, _)) = candidates.first().copied() {
                let chosen = candidates
                    .iter()
                    .filter(|(kind, depth, _)| *kind == owner_interface && *depth == distance)
                    .collect::<Vec<_>>();
                if owner_interface
                    && chosen
                        .iter()
                        .map(|(_, _, method)| method.owner)
                        .collect::<HashSet<_>>()
                        .len()
                        > 1
                {
                    graph.gap(
                        graph.nodes[class].location.context(),
                        "ambiguous interface default implementations".to_owned(),
                        emit,
                    )?;
                    continue;
                }
                for (_, _, candidate) in chosen {
                    let site = declarations::method_site(
                        &graph.nodes[candidate.owner].facts,
                        candidate.id,
                    )?;
                    let label = if interface {
                        "implements-method"
                    } else {
                        "overrides-method"
                    };
                    let use_ = format!(
                        "{label} for {} via {}",
                        at::escape_units(&graph.nodes[class].location.descriptor),
                        graph.path_label(witness)
                    );
                    let matched = Matched::Method {
                        owner: copy(&graph.nodes[target.owner].location.descriptor)?,
                        name: copy(target.declaration.name_units())?,
                        prototype: copy(target.declaration.prototype_units())?,
                    };
                    emit(
                        graph.nodes[candidate.owner]
                            .location
                            .row(site, matched, selected, use_, 1)?,
                    )?;
                }
            }
        }
    }
    Ok(())
}
