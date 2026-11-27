//! Adapter from checked JVM declaration facts to the relation graph.

use diana_decomp::input::jvm::{ClassError, JvmString};

use super::{Address, Edge, Graph, Issue, JvmRoots, Location, Method, Node, QueryError, Signature};
use super::{copy, push};
use crate::analysis::{filter::TextFilter, input::InputError, jvm_code, runner};

enum Entry {
    Read(Node, Vec<Method>),
    Issue(Issue),
}

pub(super) fn load(
    roots: &JvmRoots<'_>,
    method_name: Option<&TextFilter>,
    jobs: usize,
    sink: &mut impl super::Sink,
) -> Result<Graph, QueryError> {
    let mut graph = Graph::default();
    // Archive issues are delivered by the owning xref scan.
    runner::run(
        roots.len(),
        jobs,
        || roots.reader(),
        |reader, index, emit| {
            let root = roots.get(index).ok_or(QueryError::WorkerLost)?;
            let location = root.location();
            let parsed = reader.read(root);
            let class = match parsed {
                Ok(class) => class,
                Err(error) => {
                    return emit(Entry::Issue(Issue::new(location.to_string(), &error, None)));
                }
            };
            let mut issue = |error: ClassError, ordinal: Option<usize>| {
                let context = ordinal.map_or_else(
                    || location.to_string(),
                    |ordinal| format!("{location}:m{ordinal}"),
                );
                emit(Entry::Issue(Issue::new(
                    context,
                    &jvm_code::class_error(error),
                    None,
                )))
            };
            let mut node = match class.name() {
                Ok(name) => Node {
                    location: Location {
                        address: Address::Jvm(location.clone()),
                        descriptor: jvm_code::type_descriptor(name)?,
                    },
                    flags: class.access_flags().into(),
                    parents: Vec::new(),
                },
                Err(error) => return issue(error, None),
            };
            // The class-file shape gives an interface a java/lang/Object slot.
            // Source inheritance is described by its original interface list.
            if node.flags & 0x0200 == 0 {
                match class.super_name() {
                    Ok(Some(parent)) => push(&mut node.parents, edge(parent, false)?)?,
                    Ok(None) => {}
                    Err(error) => issue(error, None)?,
                }
            }
            for interface in class.interfaces() {
                match interface {
                    Ok(interface) => push(&mut node.parents, edge(interface, true)?)?,
                    Err(error) => issue(error, None)?,
                }
            }
            let mut declarations = Vec::new();
            if let Some(filter) = method_name {
                match class.methods() {
                    Ok(entries) => {
                        for (ordinal, method) in entries.enumerate() {
                            let method = match method {
                                Ok(method) => method,
                                Err(error) => {
                                    issue(error, Some(ordinal))?;
                                    continue;
                                }
                            };
                            let flags = method.access_flags();
                            if flags & (0x0002 | 0x0008) != 0 {
                                continue;
                            }
                            let name = match method.name(&class) {
                                Ok(name) => name,
                                Err(error) => {
                                    issue(error, Some(ordinal))?;
                                    continue;
                                }
                            };
                            // A relationship can only match a candidate with
                            // the same original name as its selected declaration.
                            if name.bytes().first() == Some(&b'<')
                                || filter.evaluate_jvm(name)?.is_none()
                            {
                                continue;
                            }
                            match method.descriptor(&class) {
                                Ok(descriptor) => push(
                                    &mut declarations,
                                    method_facts(method, name, descriptor)?,
                                )?,
                                Err(error) => issue(error, Some(ordinal))?,
                            }
                        }
                    }
                    Err(error) => issue(error, None)?,
                }
            }
            emit(Entry::Read(node, declarations))
        },
        &mut |record| match record {
            Entry::Read(node, mut methods) => {
                let owner = graph.nodes.len();
                graph.push_node(node)?;
                graph
                    .methods
                    .try_reserve(methods.len())
                    .map_err(InputError::Allocation)?;
                for method in &mut methods {
                    method.owner = owner;
                }
                graph.methods.extend(methods);
                Ok(())
            }
            Entry::Issue(issue) => sink.issue(issue),
        },
    )?;
    Ok(graph)
}

fn edge(parent: JvmString<'_>, interface: bool) -> Result<Edge, QueryError> {
    Ok(Edge {
        descriptor: jvm_code::type_descriptor(parent)?,
        interface,
    })
}

fn method_facts(
    member: diana_decomp::input::jvm::Member<'_>,
    name: JvmString<'_>,
    descriptor: JvmString<'_>,
) -> Result<Method, QueryError> {
    let prototype = descriptor.to_utf16()?;
    // Member::descriptor checked the grammar of both halves. Separating
    // individual types preserves the UTF-16 identity of surrogate names.
    let mut parameters = Vec::new();
    let mut remaining = &prototype[1..];
    let returns = loop {
        if let Some(returns) = remaining.strip_prefix(&[u16::from(b')')]) {
            break copy(returns)?;
        }
        let length = jvm_code::descriptor_size(remaining).ok_or(QueryError::WorkerLost)?;
        push(&mut parameters, copy(&remaining[..length])?)?;
        remaining = &remaining[length..];
    };
    Ok(Method {
        owner: 0, // The sink assigns the index of the owning declaration.
        ordinal: u32::from(member.ordinal()),
        flags: u32::from(member.access_flags()),
        signature: Signature {
            name: name.to_utf16()?,
            returns,
            prototype,
            parameters,
        },
    })
}
