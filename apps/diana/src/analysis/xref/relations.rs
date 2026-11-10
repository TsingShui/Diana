//! Input-local declaration relations. Witnesses are original header paths, not
//! runtime dispatch or a claim that the input is a valid Java program.
use super::declarations::{self, Location};
use super::{
    Event, Issue, Matched, MethodRelation, QueryError, Relation, Site, Target, Xref, copy,
};
use crate::analysis::{at, dex, input::InputError};
use diana_decomp::p0::Fact;
use std::collections::{HashMap, HashSet, VecDeque};

type Paths = Vec<(usize, Vec<usize>)>;
struct Edge {
    descriptor: Vec<u16>,
    interface: bool,
}
struct Node {
    location: Location,
    flags: u32,
    parents: Vec<Edge>,
}
struct Signature {
    name: Vec<u16>,
    prototype: Vec<u16>,
    parameters: Vec<Vec<u16>>,
    returns: Vec<u16>,
}
struct Method {
    owner: usize,
    id: diana_decomp::MethodId,
    flags: u32,
    signature: Signature,
}
struct Graph {
    nodes: Vec<Node>,
    methods: Vec<Method>,
    names: HashMap<Vec<u16>, Vec<usize>>,
    children: HashMap<Vec<u16>, Vec<(usize, bool)>>,
    gaps: HashSet<String>,
}
fn push<T>(values: &mut Vec<T>, value: T) -> Result<(), QueryError> {
    values.try_reserve(1).map_err(InputError::Allocation)?;
    values.push(value);
    Ok(())
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
            values => {
                let reason = if values.is_some() {
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
    fn load(
        query: &Xref<'_>,
        emit: &mut dyn FnMut(Event) -> Result<(), QueryError>,
    ) -> Result<Self, QueryError> {
        let mut graph = Self {
            nodes: Vec::new(),
            methods: Vec::new(),
            names: HashMap::new(),
            children: HashMap::new(),
            gaps: HashSet::new(),
        };
        // One pass over declarations: the payload lives in this scratch instead
        // of every root retaining its own.
        let mut scratch = Vec::new();
        for (image, root) in query.roots.iter().enumerate() {
            let stopped = std::cell::Cell::new(false);
            let mut forward = |event| {
                let result = emit(event);
                if result.is_err() {
                    stopped.set(true);
                }
                result
            };
            let result = root
                .bytes_into(&mut scratch)
                .map_err(QueryError::from)
                .and_then(|bytes| {
                    dex::visit_logical(bytes, |logical, index| {
                        for ordinal in 0..index.class_count() {
                            let class_def = diana_decomp::ClassDefId(ordinal);
                            let facts = match declarations::collect(&index, class_def) {
                                Ok(facts) => facts,
                                Err(error) => {
                                    forward(Event::Issue(Issue::new(
                                        format!("image:{image}:l{}:c{ordinal}", logical.0),
                                        &error,
                                        None,
                                    )))?;
                                    continue;
                                }
                            };
                            let location = Location {
                                image: dex::ImageIndex(image),
                                logical,
                                class_def,
                                descriptor: copy(facts.descriptor().units())?,
                            };
                            let mut parents = Vec::new();
                            match facts.superclass() {
                                Fact::Present(value) => push(
                                    &mut parents,
                                    Edge {
                                        descriptor: copy(value.units())?,
                                        interface: false,
                                    },
                                )?,
                                Fact::Unavailable(error) => forward(Event::Issue(
                                    Issue::from_request(location.context(), error),
                                ))?,
                                Fact::NotRead => forward(Event::Issue(Issue::unknown(
                                    location.context(),
                                    "superclass not read",
                                )))?,
                                Fact::Absent => (),
                            }
                            match facts.interfaces() {
                                Fact::Present(values) => {
                                    for value in values {
                                        match &value.descriptor {
                                            Fact::Present(value) => push(
                                                &mut parents,
                                                Edge {
                                                    descriptor: copy(value.units())?,
                                                    interface: true,
                                                },
                                            )?,
                                            Fact::Unavailable(error) => forward(Event::Issue(
                                                Issue::from_request(location.context(), error),
                                            ))?,
                                            _ => forward(Event::Issue(Issue::unknown(
                                                location.context(),
                                                "interface descriptor unavailable",
                                            )))?,
                                        }
                                    }
                                }
                                Fact::Unavailable(error) => forward(Event::Issue(
                                    Issue::from_request(location.context(), error),
                                ))?,
                                Fact::NotRead => forward(Event::Issue(Issue::unknown(
                                    location.context(),
                                    "interface list not read",
                                )))?,
                                Fact::Absent => (),
                            }
                            let owner = graph.nodes.len();
                            if query.plan.method_relation != MethodRelation::References {
                                if let Some(error) = facts.methods_issue() {
                                    forward(Event::Issue(Issue::from_request(
                                        location.context(),
                                        error,
                                    )))?;
                                }
                                for method in facts.methods() {
                                    let declaration = match method.declaration() {
                                        Fact::Present(value) => value,
                                        Fact::Unavailable(error) => {
                                            forward(Event::Issue(Issue::from_request(
                                                format!(
                                                    "{}:m{}",
                                                    location.context(),
                                                    method.method().0
                                                ),
                                                error,
                                            )))?;
                                            continue;
                                        }
                                        _ => {
                                            forward(Event::Issue(Issue::unknown(
                                                format!(
                                                    "{}:m{}",
                                                    location.context(),
                                                    method.method().0
                                                ),
                                                "method declaration unavailable",
                                            )))?;
                                            continue;
                                        }
                                    };
                                    let flags = method.access_flags().raw();
                                    if flags & (0x2 | 0x8) != 0
                                        || declaration.name_units().first()
                                            == Some(&u16::from(b'<'))
                                    {
                                        continue;
                                    }
                                    let mut parameters = Vec::new();
                                    parameters
                                        .try_reserve_exact(declaration.parameter_types.len())
                                        .map_err(InputError::Allocation)?;
                                    for parameter in &declaration.parameter_types {
                                        parameters.push(copy(parameter.units())?);
                                    }
                                    push(
                                        &mut graph.methods,
                                        Method {
                                            owner,
                                            id: method.method(),
                                            flags,
                                            signature: Signature {
                                                name: copy(declaration.name_units())?,
                                                prototype: copy(declaration.prototype_units())?,
                                                parameters,
                                                returns: copy(declaration.return_type.units())?,
                                            },
                                        },
                                    )?;
                                }
                            }
                            graph.names.try_reserve(1).map_err(InputError::Allocation)?;
                            push(
                                graph.names.entry(copy(&location.descriptor)?).or_default(),
                                owner,
                            )?;
                            for edge in &parents {
                                graph
                                    .children
                                    .try_reserve(1)
                                    .map_err(InputError::Allocation)?;
                                push(
                                    graph.children.entry(copy(&edge.descriptor)?).or_default(),
                                    (owner, edge.interface),
                                )?;
                            }
                            push(
                                &mut graph.nodes,
                                Node {
                                    location,
                                    flags: facts.access_flags().raw(),
                                    parents,
                                },
                            )?;
                            // P0 metadata/preparation and unneeded directories are dropped
                            // here. The graph retains only the facts its queries consume.
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
        Ok(graph)
    }
    fn path_label(&self, path: &[usize]) -> String {
        path.iter()
            .map(|&index| at::escape_units(&self.nodes[index].location.descriptor))
            .collect::<Vec<_>>()
            .join(" → ")
    }
    fn descendants(
        &mut self,
        descriptor: &[u16],
        relation: Relation,
        emit: &mut dyn FnMut(Event) -> Result<(), QueryError>,
    ) -> Result<Paths, QueryError> {
        let mut queue = VecDeque::new();
        queue.try_reserve(1).map_err(InputError::Allocation)?;
        queue.push_back((copy(descriptor)?, Vec::<usize>::new()));
        let mut visited = HashSet::new();
        let mut found = Vec::new();
        while let Some((parent, path)) = queue.pop_front() {
            let mut edges = Vec::new();
            if let Some(values) = self.children.get(&parent) {
                edges
                    .try_reserve_exact(values.len())
                    .map_err(InputError::Allocation)?;
                edges.extend_from_slice(values);
            }
            for (node, interface_edge) in edges {
                if relation == Relation::Extend && interface_edge
                    || relation == Relation::Impl && path.is_empty() && !interface_edge
                {
                    continue;
                }
                if path.contains(&node) {
                    self.gap(
                        self.nodes[node].location.context(),
                        "cyclic hierarchy path".to_owned(),
                        emit,
                    )?;
                    continue;
                }
                if visited.contains(&node) {
                    continue;
                }
                visited.try_reserve(1).map_err(InputError::Allocation)?;
                visited.insert(node);
                let mut witness = Vec::new();
                witness
                    .try_reserve(
                        path.len()
                            .checked_add(1)
                            .ok_or(InputError::PlatformCapacity)?,
                    )
                    .map_err(InputError::Allocation)?;
                witness.push(node);
                witness.extend_from_slice(&path);
                let mut retained = Vec::new();
                retained
                    .try_reserve_exact(witness.len())
                    .map_err(InputError::Allocation)?;
                retained.extend_from_slice(&witness);
                push(&mut found, (node, retained))?;
                let name = copy(&self.nodes[node].location.descriptor)?;
                if name == descriptor {
                    self.gap(
                        self.nodes[node].location.context(),
                        "cyclic hierarchy path".to_owned(),
                        emit,
                    )?;
                    continue;
                }
                if self.resolve(&name, emit)?.is_some() {
                    queue.try_reserve(1).map_err(InputError::Allocation)?;
                    queue.push_back((name, witness));
                }
            }
        }
        Ok(found)
    }
    fn ancestors(
        &mut self,
        start: usize,
        interfaces: bool,
        emit: &mut dyn FnMut(Event) -> Result<(), QueryError>,
    ) -> Result<Paths, QueryError> {
        let mut queue = VecDeque::new();
        queue.try_reserve(1).map_err(InputError::Allocation)?;
        queue.push_back((start, Vec::<usize>::new()));
        let mut visited = HashSet::new();
        let mut found = Vec::new();
        while let Some((node, path)) = queue.pop_front() {
            if visited.contains(&node) {
                continue;
            }
            visited.try_reserve(1).map_err(InputError::Allocation)?;
            visited.insert(node);
            let mut names = Vec::new();
            for edge in &self.nodes[node].parents {
                if interfaces || !edge.interface {
                    push(&mut names, copy(&edge.descriptor)?)?;
                }
            }
            for name in names {
                let Some(parent) = self.resolve(&name, emit)? else {
                    continue;
                };
                if parent == start || path.contains(&parent) {
                    self.gap(
                        self.nodes[parent].location.context(),
                        "cyclic hierarchy path".to_owned(),
                        emit,
                    )?;
                    continue;
                }
                let mut witness = Vec::new();
                witness
                    .try_reserve(
                        path.len()
                            .checked_add(1)
                            .ok_or(InputError::PlatformCapacity)?,
                    )
                    .map_err(InputError::Allocation)?;
                witness.extend_from_slice(&path);
                witness.push(parent);
                let mut retained = Vec::new();
                retained
                    .try_reserve_exact(witness.len())
                    .map_err(InputError::Allocation)?;
                retained.extend_from_slice(&witness);
                push(&mut found, (parent, retained))?;
                queue.try_reserve(1).map_err(InputError::Allocation)?;
                queue.push_back((parent, witness));
            }
        }
        Ok(found)
    }
    fn covariant(
        &mut self,
        mut child: &[u16],
        mut parent: &[u16],
        emit: &mut dyn FnMut(Event) -> Result<(), QueryError>,
    ) -> Result<bool, QueryError> {
        loop {
            if child == parent {
                return Ok(true);
            }
            if let Some(tail) = child.strip_prefix(&[u16::from(b'[')]) {
                if let Some(parent_tail) = parent.strip_prefix(&[u16::from(b'[')]) {
                    child = tail;
                    parent = parent_tail;
                    continue;
                }
                // These are intrinsic array supertypes, not external declarations.
                if [
                    "Ljava/lang/Object;",
                    "Ljava/lang/Cloneable;",
                    "Ljava/io/Serializable;",
                ]
                .iter()
                .any(|name| name.encode_utf16().eq(parent.iter().copied()))
                {
                    return Ok(true);
                }
                return Ok(false);
            }
            if child.first() != Some(&u16::from(b'L')) || parent.first() != Some(&u16::from(b'L')) {
                return Ok(false);
            }
            let Some(start) = self.resolve(child, emit)? else {
                return Ok(false);
            };
            // Matching an encoded direct parent requires no target definition.
            let mut pending = vec![start];
            let mut visited = HashSet::new();
            while let Some(node) = pending.pop() {
                if visited.contains(&node) {
                    continue;
                }
                visited.try_reserve(1).map_err(InputError::Allocation)?;
                visited.insert(node);
                let mut edges = Vec::new();
                for edge in &self.nodes[node].parents {
                    if edge.descriptor == parent {
                        return Ok(true);
                    }
                    push(&mut edges, copy(&edge.descriptor)?)?;
                }
                for name in edges {
                    if let Some(next) = self.resolve(&name, emit)? {
                        push(&mut pending, next)?;
                    }
                }
            }
            return Ok(false);
        }
    }
    fn compatible(
        &mut self,
        child: &Signature,
        parent: &Signature,
        emit: &mut dyn FnMut(Event) -> Result<(), QueryError>,
    ) -> Result<bool, QueryError> {
        if child.name != parent.name || child.parameters != parent.parameters {
            return Ok(false);
        }
        self.covariant(&child.returns, &parent.returns, emit)
    }
    fn same_package(&self, left: usize, right: usize) -> bool {
        let prefix = |value: &[u16]| {
            value
                .iter()
                .rposition(|&unit| unit == u16::from(b'/'))
                .unwrap_or(0)
        };
        let left = &self.nodes[left].location.descriptor;
        let right = &self.nodes[right].location.descriptor;
        left[..prefix(left)] == right[..prefix(right)]
    }
}

pub(super) fn run(
    query: &Xref<'_>,
    emit: &mut dyn FnMut(Event) -> Result<(), QueryError>,
) -> Result<(), QueryError> {
    let mut graph = Graph::load(query, emit)?;
    if query.plan.method_relation != MethodRelation::References {
        for target in &query.plan.targets {
            if let Target::Method {
                owner: Some(super::TextMatch::Exact(owner)),
                ..
            } = target
            {
                graph.resolve(owner, emit)?;
            }
        }
        return method_relations(&mut graph, query, emit);
    }
    let mut names = Vec::new();
    for descriptor in graph.children.keys() {
        push(&mut names, copy(descriptor)?)?;
    }
    // Unknown intermediate declarations make global negative results incomplete.
    // Known reverse edges are still independently usable positive witnesses.
    if let Some(class) = query.plan.class.as_deref() {
        if let Some(node) = graph.resolve(class, emit)? {
            graph.ancestors(node, true, emit)?;
        }
    } else {
        for descriptor in &names {
            graph.resolve(descriptor, emit)?;
        }
    }
    for target in &query.plan.targets {
        let Target::Hierarchy { relation, target } = target else {
            continue;
        };
        for descriptor in &names {
            let Some(selected) = target.evaluate_units(descriptor)? else {
                continue;
            };
            for (node, path) in graph.descendants(descriptor, *relation, emit)? {
                if query
                    .plan
                    .class
                    .as_deref()
                    .is_some_and(|filter| filter != graph.nodes[node].location.descriptor)
                {
                    continue;
                }
                let label = match relation {
                    Relation::Extend => "extends",
                    Relation::Impl => {
                        if graph.nodes[node].flags & 0x200 != 0 {
                            "extends-interface"
                        } else {
                            "implements"
                        }
                    }
                };
                let use_ = if path.len() == 1 {
                    label.to_owned()
                } else {
                    format!("{label}-indirect via {}", graph.path_label(&path[1..]))
                };
                let depth = u32::try_from(path.len()).map_err(|_| InputError::PlatformCapacity)?;
                emit(graph.nodes[node].location.row(
                    Site::Class,
                    Matched::Type(copy(descriptor)?),
                    selected,
                    use_,
                    depth,
                )?)?;
            }
        }
    }
    Ok(())
}

fn method_relations(
    graph: &mut Graph,
    query: &Xref<'_>,
    emit: &mut dyn FnMut(Event) -> Result<(), QueryError>,
) -> Result<(), QueryError> {
    let methods = std::mem::take(&mut graph.methods);
    let mut by_name: HashMap<&[u16], Vec<usize>> = HashMap::new();
    for (index, method) in methods.iter().enumerate() {
        by_name.try_reserve(1).map_err(InputError::Allocation)?;
        push(by_name.entry(&method.signature.name).or_default(), index)?;
    }
    let implementing = query.plan.method_relation == MethodRelation::Implementations;
    for target in &methods {
        let interface = graph.nodes[target.owner].flags & 0x200 != 0;
        if implementing && !interface || !implementing && target.flags & 0x10 != 0 {
            continue;
        }
        let descriptor = copy(&graph.nodes[target.owner].location.descriptor)?;
        let value = Matched::Method {
            owner: copy(&descriptor)?,
            name: copy(&target.signature.name)?,
            prototype: copy(&target.signature.prototype)?,
        };
        let Some((_, selected)) = declarations::matched_value(value, &query.plan.targets)? else {
            continue;
        };
        if graph.resolve(&descriptor, emit)? != Some(target.owner) {
            continue;
        }
        let relation = if interface {
            Relation::Impl
        } else {
            Relation::Extend
        };
        for (class, witness) in graph.descendants(&descriptor, relation, emit)? {
            if query
                .plan
                .class
                .as_deref()
                .is_some_and(|filter| filter != graph.nodes[class].location.descriptor)
            {
                continue;
            }
            if !interface
                && target.flags & (0x1 | 0x4) == 0
                && !witness
                    .iter()
                    .all(|&node| graph.same_package(node, target.owner))
            {
                graph.gap(
                    graph.nodes[class].location.context(),
                    "package-private inheritance across package boundaries is not established"
                        .to_owned(),
                    emit,
                )?;
                continue;
            }
            let mut owners = HashMap::new();
            owners.try_reserve(1).map_err(InputError::Allocation)?;
            owners.insert(class, 0usize);
            if implementing {
                for (owner, path) in graph.ancestors(class, true, emit)? {
                    owners.try_reserve(1).map_err(InputError::Allocation)?;
                    owners.entry(owner).or_insert(path.len());
                }
            }
            let mut candidates = Vec::new();
            for &index in by_name
                .get(target.signature.name.as_slice())
                .into_iter()
                .flatten()
            {
                let candidate = &methods[index];
                let Some(&distance) = owners.get(&candidate.owner) else {
                    continue;
                };
                if candidate.owner == target.owner
                    && candidate.id == target.id
                    && (!implementing || target.flags & 0x400 != 0)
                {
                    continue;
                }
                if !graph.compatible(&candidate.signature, &target.signature, emit)? {
                    continue;
                }
                if interface && candidate.flags & 0x1 == 0 {
                    graph.gap(graph.nodes[candidate.owner].location.context(), "non-public declaration does not establish a public interface implementation".to_owned(), emit)?;
                    continue;
                }
                push(
                    &mut candidates,
                    (
                        graph.nodes[candidate.owner].flags & 0x200 != 0,
                        distance,
                        index,
                    ),
                )?;
            }
            let class_depth = candidates
                .iter()
                .filter(|(is_interface, _, _)| !is_interface)
                .map(|(_, distance, _)| *distance)
                .min();
            let mut chosen = Vec::new();
            if let Some(depth) = class_depth {
                for (is_interface, distance, index) in &candidates {
                    if !is_interface && *distance == depth {
                        push(&mut chosen, *index)?;
                    }
                }
            } else {
                for (_, _, index) in &candidates {
                    let candidate = &methods[*index];
                    let mut dominated = false;
                    for (_, _, other) in &candidates {
                        let other = &methods[*other];
                        if candidate.owner == other.owner {
                            continue;
                        }
                        if graph
                            .ancestors(other.owner, true, emit)?
                            .iter()
                            .any(|(ancestor, _)| *ancestor == candidate.owner)
                        {
                            dominated = true;
                            break;
                        }
                    }
                    if !dominated {
                        push(&mut chosen, *index)?;
                    }
                }
                if chosen.first().is_some_and(|first| {
                    chosen
                        .iter()
                        .any(|index| methods[*index].owner != methods[*first].owner)
                }) {
                    graph.gap(
                        graph.nodes[class].location.context(),
                        "ambiguous interface default declarations".to_owned(),
                        emit,
                    )?;
                    continue;
                }
            }
            for index in chosen {
                let candidate = &methods[index];
                if implementing && candidate.flags & 0x400 != 0 {
                    continue;
                }
                let site = Site::Method {
                    method: candidate.id,
                    name: copy(&candidate.signature.name)?,
                    prototype: copy(&candidate.signature.prototype)?,
                };
                let label = if implementing {
                    "implements-method"
                } else {
                    "overrides-method"
                };
                let default_candidate =
                    implementing && graph.nodes[candidate.owner].flags & 0x200 != 0;
                let use_ = format!(
                    "{label}{} for {} via {}",
                    if default_candidate {
                        " default-candidate"
                    } else {
                        ""
                    },
                    at::escape_units(&graph.nodes[class].location.descriptor),
                    graph.path_label(&witness)
                );
                let value = Matched::Method {
                    owner: copy(&descriptor)?,
                    name: copy(&target.signature.name)?,
                    prototype: copy(&target.signature.prototype)?,
                };
                emit(graph.nodes[candidate.owner].location.row(
                    site,
                    value,
                    selected,
                    use_,
                    u32::try_from(witness.len()).map_err(|_| InputError::PlatformCapacity)?,
                )?)?;
            }
        }
    }
    Ok(())
}
