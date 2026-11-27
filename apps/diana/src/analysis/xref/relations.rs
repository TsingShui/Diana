//! Input-local declaration relations. Witnesses are original header paths, not
//! runtime dispatch or a claim that the input is a valid Java program.
use super::declarations;
use super::{
    Event, FilterMatch, Issue, Matched, MethodRelation, QueryError, Relation, Site, Target, Xref,
    copy,
};
use crate::analysis::{
    at, dex,
    input::InputError,
    jvm::{JvmRoots, Location as JvmLocation},
    jvm_code::{self, Hit, MemberSite},
};
use diana_decomp::p0::Fact;
use std::collections::{HashMap, HashSet, VecDeque};

// Hierarchy and method relationships consume original declaration facts from
// either input format. The reader adapter owns physical locations and errors.
struct Location {
    address: Address,
    descriptor: Vec<u16>,
}

enum Address {
    Dex {
        image: dex::ImageIndex,
        logical: crate::analysis::images::LogicalDex,
        class_def: diana_decomp::ClassDefId,
    },
    Jvm(JvmLocation),
}

struct RelationHit {
    member: Option<MethodSiteFact>,
    matched: Matched,
    selected: FilterMatch,
    use_: String,
    depth: u32,
}

struct MethodSiteFact {
    ordinal: u32,
    name: Vec<u16>,
    prototype: Vec<u16>,
}

struct Plan<'a> {
    class: Option<&'a [u16]>,
    targets: &'a [Target],
    method_relation: MethodRelation,
}

trait Sink {
    fn issue(&mut self, issue: Issue) -> Result<(), QueryError>;
    fn hit(&mut self, location: &Location, hit: RelationHit) -> Result<(), QueryError>;
}

struct DexSink<'a>(&'a mut dyn FnMut(Event) -> Result<(), QueryError>);

impl Sink for DexSink<'_> {
    fn issue(&mut self, issue: Issue) -> Result<(), QueryError> {
        (self.0)(Event::Issue(issue))
    }

    fn hit(&mut self, location: &Location, hit: RelationHit) -> Result<(), QueryError> {
        let Address::Dex {
            image,
            logical,
            class_def,
        } = location.address
        else {
            return Err(QueryError::WorkerLost);
        };
        let site = hit.member.map_or(Site::Class, |member| Site::Method {
            method: diana_decomp::MethodId(member.ordinal),
            name: member.name,
            prototype: member.prototype,
        });
        (self.0)(Event::Row(super::XrefRow {
            image,
            logical,
            class_def,
            class_descriptor: copy(&location.descriptor)?,
            site,
            relevance: hit.selected.relevance,
            matched: hit.matched,
            use_: hit.use_,
            depth: hit.depth,
        }))
    }
}

struct JvmSink<'a>(&'a mut dyn FnMut(jvm_code::Event) -> Result<(), QueryError>);

impl Sink for JvmSink<'_> {
    fn issue(&mut self, issue: Issue) -> Result<(), QueryError> {
        (self.0)(jvm_code::Event::Issue(issue))
    }

    fn hit(&mut self, location: &Location, hit: RelationHit) -> Result<(), QueryError> {
        let Address::Jvm(location) = &location.address else {
            return Err(QueryError::WorkerLost);
        };
        let member = hit
            .member
            .map(|member| u16::try_from(member.ordinal).map(MemberSite::Method))
            .transpose()
            .map_err(|_| InputError::PlatformCapacity)?;
        let matched = match hit.matched {
            Matched::Type(units) => at::escape_units(&units),
            Matched::Method {
                owner,
                name,
                prototype,
            } => {
                format!(
                    "{}->{}{}",
                    at::escape_units(&owner),
                    at::escape_units(&name),
                    at::escape_units(&prototype)
                )
            }
            _ => return Err(QueryError::WorkerLost),
        };
        (self.0)(jvm_code::Event::Hit(Hit {
            location: location.clone(),
            member,
            offset_byte: None,
            matched,
            use_: hit.use_.into(),
            relevance: hit.selected.relevance,
            depth: hit.depth,
            caller: None,
        }))
    }
}

impl Location {
    fn context(&self) -> String {
        match &self.address {
            Address::Dex {
                image,
                logical,
                class_def,
            } => format!("image:{}:l{}:c{}", image.0, logical.0, class_def.0),
            Address::Jvm(location) => location.to_string(),
        }
    }
}

#[derive(Default)]
struct Paths {
    visits: Vec<Visit>,
}

struct Visit {
    node: usize,
    through: Option<usize>,
    depth: usize,
}

impl Paths {
    fn reach(&mut self, node: usize, through: Option<usize>) -> Result<usize, QueryError> {
        let depth = through
            .map_or(0, |parent| self.visits[parent].depth)
            .checked_add(1)
            .ok_or(InputError::PlatformCapacity)?;
        let index = self.visits.len();
        push(
            &mut self.visits,
            Visit {
                node,
                through,
                depth,
            },
        )?;
        Ok(index)
    }

    fn nodes(&self, from: Option<usize>) -> impl Iterator<Item = usize> + '_ {
        std::iter::successors(from, |&index| self.visits[index].through)
            .map(|index| self.visits[index].node)
    }
}

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
    ordinal: u32,
    flags: u32,
    signature: Signature,
}
#[derive(Default)]
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
        sink: &mut impl Sink,
    ) -> Result<(), QueryError> {
        let key = format!("{context}: {reason}");
        if self.gaps.contains(&key) {
            return Ok(());
        }
        self.gaps.try_reserve(1).map_err(InputError::Allocation)?;
        self.gaps.insert(key);
        sink.issue(Issue {
            context,
            kind: "Unknown".to_owned(),
            reason,
            range_bytes: None,
        })
    }
    fn resolve(
        &mut self,
        descriptor: &[u16],
        sink: &mut impl Sink,
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
                    sink,
                )?;
                Ok(None)
            }
        }
    }
    fn load_dex(query: &Xref<'_>, sink: &mut impl Sink) -> Result<Self, QueryError> {
        let mut graph = Self::default();
        // One pass over declarations: the payload lives in this scratch instead
        // of every root retaining its own.
        let mut scratch = Vec::new();
        for (image, root) in query.roots.iter().enumerate() {
            let stopped = std::cell::Cell::new(false);
            let mut forward = |issue| {
                let result = sink.issue(issue);
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
                                    forward(Issue::new(
                                        format!("image:{image}:l{}:c{ordinal}", logical.0),
                                        &error,
                                        None,
                                    ))?;
                                    continue;
                                }
                            };
                            let location = Location {
                                address: Address::Dex {
                                    image: dex::ImageIndex(image),
                                    logical,
                                    class_def,
                                },
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
                                Fact::Unavailable(error) => {
                                    forward(Issue::from_request(location.context(), error))?
                                }
                                Fact::NotRead => forward(Issue::unknown(
                                    location.context(),
                                    "superclass not read",
                                ))?,
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
                                            Fact::Unavailable(error) => forward(
                                                Issue::from_request(location.context(), error),
                                            )?,
                                            _ => forward(Issue::unknown(
                                                location.context(),
                                                "interface descriptor unavailable",
                                            ))?,
                                        }
                                    }
                                }
                                Fact::Unavailable(error) => {
                                    forward(Issue::from_request(location.context(), error))?
                                }
                                Fact::NotRead => forward(Issue::unknown(
                                    location.context(),
                                    "interface list not read",
                                ))?,
                                Fact::Absent => (),
                            }
                            let owner = graph.nodes.len();
                            if query.plan.method_relation != MethodRelation::References {
                                if let Some(error) = facts.methods_issue() {
                                    forward(Issue::from_request(location.context(), error))?;
                                }
                                for method in facts.methods() {
                                    let declaration = match method.declaration() {
                                        Fact::Present(value) => value,
                                        Fact::Unavailable(error) => {
                                            forward(Issue::from_request(
                                                format!(
                                                    "{}:m{}",
                                                    location.context(),
                                                    method.method().0
                                                ),
                                                error,
                                            ))?;
                                            continue;
                                        }
                                        _ => {
                                            forward(Issue::unknown(
                                                format!(
                                                    "{}:m{}",
                                                    location.context(),
                                                    method.method().0
                                                ),
                                                "method declaration unavailable",
                                            ))?;
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
                                            ordinal: method.method().0,
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
                            graph.push_node(Node {
                                location,
                                flags: facts.access_flags().raw(),
                                parents,
                            })?;
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
                forward(Issue::new(format!("image:{image}"), &error, None))?;
            }
        }
        Ok(graph)
    }
    fn push_node(&mut self, node: Node) -> Result<(), QueryError> {
        let owner = self.nodes.len();
        self.names.try_reserve(1).map_err(InputError::Allocation)?;
        push(
            self.names
                .entry(copy(&node.location.descriptor)?)
                .or_default(),
            owner,
        )?;
        for edge in &node.parents {
            self.children
                .try_reserve(1)
                .map_err(InputError::Allocation)?;
            push(
                self.children.entry(copy(&edge.descriptor)?).or_default(),
                (owner, edge.interface),
            )?;
        }
        push(&mut self.nodes, node)
    }
    fn path_label(&self, path: impl Iterator<Item = usize>) -> Result<String, QueryError> {
        let mut label = String::new();
        for index in path {
            let name = at::escape_units(&self.nodes[index].location.descriptor);
            let separator = if label.is_empty() { "" } else { " → " };
            let remaining = separator
                .len()
                .checked_add(name.len())
                .ok_or(InputError::PlatformCapacity)?;
            label
                .try_reserve(remaining)
                .map_err(InputError::Allocation)?;
            label.push_str(separator);
            label.push_str(&name);
        }
        Ok(label)
    }

    fn descendants(
        &mut self,
        descriptor: &[u16],
        relation: Relation,
        sink: &mut impl Sink,
    ) -> Result<Paths, QueryError> {
        let mut queue = VecDeque::<Option<usize>>::new();
        queue.try_reserve(1).map_err(InputError::Allocation)?;
        queue.push_back(None);
        let mut visited = HashSet::new();
        let mut found = Paths::default();
        while let Some(path) = queue.pop_front() {
            let parent = path.map_or(descriptor, |index| {
                self.nodes[found.visits[index].node]
                    .location
                    .descriptor
                    .as_slice()
            });
            let mut edges = Vec::new();
            if let Some(values) = self.children.get(parent) {
                edges
                    .try_reserve_exact(values.len())
                    .map_err(InputError::Allocation)?;
                edges.extend_from_slice(values);
            }
            for (node, interface_edge) in edges {
                if relation == Relation::Extend && interface_edge
                    || relation == Relation::Impl && path.is_none() && !interface_edge
                {
                    continue;
                }
                if visited.contains(&node) {
                    if found.nodes(path).any(|ancestor| ancestor == node) {
                        self.gap(
                            self.nodes[node].location.context(),
                            "cyclic hierarchy path".to_owned(),
                            sink,
                        )?;
                    }
                    continue;
                }
                visited.try_reserve(1).map_err(InputError::Allocation)?;
                visited.insert(node);
                let entry = found.reach(node, path)?;
                let name = copy(&self.nodes[node].location.descriptor)?;
                if name == descriptor {
                    self.gap(
                        self.nodes[node].location.context(),
                        "cyclic hierarchy path".to_owned(),
                        sink,
                    )?;
                    continue;
                }
                if self.resolve(&name, sink)?.is_some() {
                    queue.try_reserve(1).map_err(InputError::Allocation)?;
                    queue.push_back(Some(entry));
                }
            }
        }
        Ok(found)
    }

    fn ancestors(
        &mut self,
        start: usize,
        interfaces: bool,
        sink: &mut impl Sink,
    ) -> Result<Paths, QueryError> {
        let mut queue = VecDeque::new();
        queue.try_reserve(1).map_err(InputError::Allocation)?;
        queue.push_back((start, None));
        let mut visited = HashSet::new();
        let mut found = Paths::default();
        while let Some((node, path)) = queue.pop_front() {
            let mut names = Vec::new();
            for edge in &self.nodes[node].parents {
                if interfaces || !edge.interface {
                    push(&mut names, copy(&edge.descriptor)?)?;
                }
            }
            for name in names {
                let Some(parent) = self.resolve(&name, sink)? else {
                    continue;
                };
                if parent == start || visited.contains(&parent) {
                    if parent == start || found.nodes(path).any(|ancestor| ancestor == parent) {
                        self.gap(
                            self.nodes[parent].location.context(),
                            "cyclic hierarchy path".to_owned(),
                            sink,
                        )?;
                    }
                    continue;
                }
                visited.try_reserve(1).map_err(InputError::Allocation)?;
                visited.insert(parent);
                let entry = found.reach(parent, path)?;
                queue.try_reserve(1).map_err(InputError::Allocation)?;
                queue.push_back((parent, Some(entry)));
            }
        }
        Ok(found)
    }
    fn covariant(
        &mut self,
        mut child: &[u16],
        mut parent: &[u16],
        sink: &mut impl Sink,
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
            let Some(start) = self.resolve(child, sink)? else {
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
                    if let Some(next) = self.resolve(&name, sink)? {
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
        sink: &mut impl Sink,
    ) -> Result<bool, QueryError> {
        if child.name != parent.name || child.parameters != parent.parameters {
            return Ok(false);
        }
        self.covariant(&child.returns, &parent.returns, sink)
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
    let mut sink = DexSink(emit);
    let graph = Graph::load_dex(query, &mut sink)?;
    run_relations(
        graph,
        Plan {
            class: query.plan.class.as_deref(),
            targets: &query.plan.targets,
            method_relation: query.plan.method_relation,
        },
        &mut sink,
    )
}

pub(crate) fn run_jvm(
    roots: &JvmRoots<'_>,
    target: &Target,
    class: Option<&[u16]>,
    method_relation: MethodRelation,
    jobs: usize,
    emit: &mut dyn FnMut(jvm_code::Event) -> Result<(), QueryError>,
) -> Result<(), QueryError> {
    let mut sink = JvmSink(emit);
    let method_name = match target {
        Target::Method { name, .. } if method_relation != MethodRelation::References => Some(name),
        _ => None,
    };
    let graph = jvm::load(roots, method_name, jobs, &mut sink)?;
    run_relations(
        graph,
        Plan {
            class,
            targets: std::slice::from_ref(target),
            method_relation,
        },
        &mut sink,
    )
}

mod jvm;

fn run_relations(mut graph: Graph, plan: Plan<'_>, sink: &mut impl Sink) -> Result<(), QueryError> {
    if plan.method_relation != MethodRelation::References {
        for target in plan.targets {
            if let Target::Method {
                owner: Some(super::TextMatch::Exact(owner)),
                ..
            } = target
            {
                graph.resolve(owner, sink)?;
            }
        }
        return method_relations(&mut graph, &plan, sink);
    }
    let mut names = Vec::new();
    for descriptor in graph.children.keys() {
        push(&mut names, copy(descriptor)?)?;
    }
    // Unknown intermediate declarations make global negative results incomplete.
    // Known reverse edges are still independently usable positive witnesses.
    if let Some(class) = plan.class {
        if let Some(node) = graph.resolve(class, sink)? {
            graph.ancestors(node, true, sink)?;
        }
    } else {
        for descriptor in &names {
            graph.resolve(descriptor, sink)?;
        }
    }
    for target in plan.targets {
        let Target::Hierarchy { relation, target } = target else {
            continue;
        };
        for descriptor in &names {
            let Some(selected) = target.evaluate_units(descriptor)? else {
                continue;
            };
            let paths = graph.descendants(descriptor, *relation, sink)?;
            for path in &paths.visits {
                let node = path.node;
                if plan
                    .class
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
                let use_ = if let Some(parent) = path.through {
                    format!(
                        "{label}-indirect via {}",
                        graph.path_label(paths.nodes(Some(parent)))?
                    )
                } else {
                    label.to_owned()
                };
                let depth = u32::try_from(path.depth).map_err(|_| InputError::PlatformCapacity)?;
                sink.hit(
                    &graph.nodes[node].location,
                    RelationHit {
                        member: None,
                        matched: Matched::Type(copy(descriptor)?),
                        selected,
                        use_,
                        depth,
                    },
                )?;
            }
        }
    }
    Ok(())
}

fn method_relations(
    graph: &mut Graph,
    plan: &Plan<'_>,
    sink: &mut impl Sink,
) -> Result<(), QueryError> {
    let methods = std::mem::take(&mut graph.methods);
    let mut by_name: HashMap<&[u16], Vec<usize>> = HashMap::new();
    for (index, method) in methods.iter().enumerate() {
        by_name.try_reserve(1).map_err(InputError::Allocation)?;
        push(by_name.entry(&method.signature.name).or_default(), index)?;
    }
    let implementing = plan.method_relation == MethodRelation::Implementations;
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
        let Some((_, selected)) = declarations::matched_value(value, plan.targets)? else {
            continue;
        };
        if graph.resolve(&descriptor, sink)? != Some(target.owner) {
            continue;
        }
        let relation = if interface {
            Relation::Impl
        } else {
            Relation::Extend
        };
        let paths = graph.descendants(&descriptor, relation, sink)?;
        for (path_index, witness) in paths.visits.iter().enumerate() {
            let class = witness.node;
            if plan
                .class
                .is_some_and(|filter| filter != graph.nodes[class].location.descriptor)
            {
                continue;
            }
            if !interface
                && target.flags & (0x1 | 0x4) == 0
                && !paths
                    .nodes(Some(path_index))
                    .all(|node| graph.same_package(node, target.owner))
            {
                graph.gap(
                    graph.nodes[class].location.context(),
                    "package-private inheritance across package boundaries is not established"
                        .to_owned(),
                    sink,
                )?;
                continue;
            }
            let mut owners = HashMap::new();
            owners.try_reserve(1).map_err(InputError::Allocation)?;
            owners.insert(class, 0usize);
            if implementing {
                for path in graph.ancestors(class, true, sink)?.visits {
                    owners.try_reserve(1).map_err(InputError::Allocation)?;
                    owners.entry(path.node).or_insert(path.depth);
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
                    && candidate.ordinal == target.ordinal
                    && (!implementing || target.flags & 0x400 != 0)
                {
                    continue;
                }
                if !graph.compatible(&candidate.signature, &target.signature, sink)? {
                    continue;
                }
                if interface && candidate.flags & 0x1 == 0 {
                    graph.gap(graph.nodes[candidate.owner].location.context(), "non-public declaration does not establish a public interface implementation".to_owned(), sink)?;
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
                            .ancestors(other.owner, true, sink)?
                            .visits
                            .iter()
                            .any(|path| path.node == candidate.owner)
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
                        sink,
                    )?;
                    continue;
                }
            }
            for index in chosen {
                let candidate = &methods[index];
                if implementing && candidate.flags & 0x400 != 0 {
                    continue;
                }
                let site = MethodSiteFact {
                    ordinal: candidate.ordinal,
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
                    graph.path_label(paths.nodes(Some(path_index)))?
                );
                let value = Matched::Method {
                    owner: copy(&descriptor)?,
                    name: copy(&target.signature.name)?,
                    prototype: copy(&target.signature.prototype)?,
                };
                sink.hit(
                    &graph.nodes[candidate.owner].location,
                    RelationHit {
                        member: Some(site),
                        matched: value,
                        selected,
                        use_,
                        depth: u32::try_from(witness.depth)
                            .map_err(|_| InputError::PlatformCapacity)?,
                    },
                )?;
            }
        }
    }
    Ok(())
}
