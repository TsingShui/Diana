//! A call outline over original JVM definitions in the requested input.

use std::collections::{HashMap, HashSet, VecDeque};

use diana_decomp::FailureKind;
use diana_decomp::input::jvm::bytecode::Reference;
use diana_decomp::input::jvm::{ClassFile, Member};
use serde::Serialize;

use super::input::InputError;
use super::jvm::{JvmReader, JvmRoots, Location};
use super::jvm_code::{class_error, display, method_code, type_text};
use super::query::QueryError;
use super::runner;
use super::xref::Issue;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct MethodKey {
    class: Location,
    ordinal: u16,
}

impl MethodKey {
    fn text(&self) -> String {
        format!("{}:m{}", self.class, self.ordinal)
    }
}

#[derive(Serialize)]
pub(crate) struct Outline {
    pub root: String,
    pub members: Vec<Declaration>,
    pub calls: Vec<Call>,
    pub issues: Vec<Issue>,
}

#[derive(Serialize)]
pub(crate) struct Declaration {
    pub kind: &'static str,
    pub at: String,
    pub symbol: Option<String>,
    pub access_flags: Option<u16>,
}

#[derive(Serialize)]
pub(crate) struct Call {
    pub caller: String,
    pub offset_byte: u16,
    pub mnemonic: &'static str,
    pub symbol: String,
    pub target: Option<String>,
    pub resolution: &'static str,
    pub depth: usize,
}

struct Edge {
    offset: u16,
    mnemonic: &'static str,
    symbol: String,
    lookup: Option<MemberSymbol>,
    unresolved: &'static str,
}

struct MemberSymbol {
    owner: Vec<u8>,
    name: Vec<u8>,
    descriptor: Vec<u8>,
    invoke: Invoke,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Invoke {
    Static,
    Constructor,
    Special,
    Virtual,
    Interface,
}

impl Invoke {
    fn runtime_exact(self) -> bool {
        matches!(self, Self::Static | Self::Constructor)
    }
}

pub(crate) fn outline(
    roots: &JvmRoots<'_>,
    location: Location,
    ordinal: Option<u16>,
    depth: Option<usize>,
    method_jobs: usize,
    lookup_jobs: usize,
    mut initial_issues: Vec<Issue>,
) -> Result<Outline, QueryError> {
    // Named lookup has already reported archive and class identity issues.
    // Physical expansion has not scanned the other roots yet.
    if initial_issues.is_empty() {
        for issue in roots.issues() {
            initial_issues
                .try_reserve(1)
                .map_err(InputError::Allocation)?;
            initial_issues.push(Issue {
                context: issue.context.clone(),
                kind: issue.kind.clone(),
                reason: issue.reason.clone(),
                range_bytes: None,
            });
        }
    }
    let mut doc = Outline {
        root: ordinal.map_or_else(
            || location.to_string(),
            |ordinal| format!("{location}:m{ordinal}"),
        ),
        members: Vec::new(),
        calls: Vec::new(),
        issues: initial_issues,
    };
    let mut work = VecDeque::new();
    let mut scheduled = HashSet::new();
    let mut reader = roots.reader();
    let mut resolver = Resolver::default();
    let entry = roots.by_location(&location).ok_or_else(|| {
        local(
            FailureKind::Unsupported,
            format_args!("no JVM definition at {location}"),
        )
    })?;
    let class = reader.read(entry)?;
    if let Some(ordinal) = ordinal {
        let method = member(&class, ordinal)?;
        PreparedMethod::read(
            &class,
            &location,
            usize::from(ordinal),
            Ok(method),
            depth != Some(0),
        )?
        .register(&mut doc, &mut work, &mut scheduled)?;
    } else {
        let owner = match class.name() {
            Ok(name) => Some(type_text(name)?),
            Err(error) => {
                push_issue(&mut doc.issues, &doc.root, &class_error(error), None)?;
                None
            }
        };
        push_declaration(
            &mut doc.members,
            "class",
            doc.root.clone(),
            owner,
            Some(class.access_flags()),
        )?;
        match class.fields() {
            Ok(fields) => {
                for (index, field) in fields.enumerate() {
                    let at = format!("{location}:f{index}");
                    match field {
                        Ok(field) => {
                            let symbol =
                                declaration_symbol(&class, field, true, &at, &mut doc.issues)?;
                            push_declaration(
                                &mut doc.members,
                                "field",
                                at,
                                symbol,
                                Some(field.access_flags()),
                            )?;
                        }
                        Err(error) => {
                            push_issue(&mut doc.issues, &at, &class_error(error), None)?;
                            push_declaration(&mut doc.members, "field", at, None, None)?;
                        }
                    }
                }
            }
            Err(error) => push_issue(&mut doc.issues, &doc.root, &class_error(error), None)?,
        }
        match class.methods() {
            Ok(methods) => {
                let mut entries = Vec::new();
                entries
                    .try_reserve_exact(methods.size_hint().0)
                    .map_err(InputError::Allocation)?;
                for entry in methods {
                    entries.try_reserve(1).map_err(InputError::Allocation)?;
                    entries.push(entry);
                }
                // Worker results are folded back in original declaration
                // order before breadth-first expansion from all class members.
                let mut prepared = Vec::new();
                prepared
                    .try_reserve_exact(entries.len())
                    .map_err(InputError::Allocation)?;
                runner::run(
                    entries.len(),
                    method_jobs,
                    || (),
                    |(), ordinal, emit| {
                        emit((
                            ordinal,
                            PreparedMethod::read(
                                &class,
                                &location,
                                ordinal,
                                entries[ordinal],
                                depth != Some(0),
                            )?,
                        ))
                    },
                    &mut |value| {
                        prepared.push(value);
                        Ok::<_, QueryError>(())
                    },
                )?;
                prepared.sort_unstable_by_key(|(ordinal, _)| *ordinal);
                for (_, method) in prepared {
                    method.register(&mut doc, &mut work, &mut scheduled)?;
                }
            }
            Err(error) => push_issue(&mut doc.issues, &doc.root, &class_error(error), None)?,
        }
    }
    while let Some((key, level, initial_edges)) = work.pop_front() {
        let caller = key.text();
        let edges = if let Some(edges) = initial_edges {
            edges
        } else {
            let result = (|| {
                let entry = roots
                    .by_location(&key.class)
                    .ok_or(QueryError::WorkerLost)?;
                let class = reader.read(entry)?;
                let method = member(&class, key.ordinal)?;
                call_edges(
                    &class,
                    method,
                    &caller,
                    depth.is_none_or(|limit| level < limit),
                    &mut doc.issues,
                )
            })();
            match result {
                Ok(edges) => edges,
                Err(error) => {
                    push_issue(&mut doc.issues, &caller, &error, None)?;
                    continue;
                }
            }
        };
        let next_level = level.checked_add(1).ok_or(InputError::PlatformCapacity)?;
        for edge in edges {
            let mut target = None;
            let resolution = match edge.lookup {
                None => edge.unresolved,
                Some(symbol) => {
                    match resolver.resolve(
                        roots,
                        &mut reader,
                        lookup_jobs,
                        &symbol,
                        &mut doc.issues,
                    )? {
                        Resolved::Method(method, uncertain) => {
                            target = Some(method.text());
                            scheduled.try_reserve(1).map_err(InputError::Allocation)?;
                            let first = scheduled.insert(method.clone());
                            if first {
                                work.try_reserve(1).map_err(InputError::Allocation)?;
                                work.push_back((method, next_level, None));
                            }
                            match (symbol.invoke.runtime_exact(), first, uncertain) {
                                (true, true, false) => "local",
                                (true, true, true) => "local-candidate",
                                (true, false, false) => "link",
                                (true, false, true) => "candidate-link",
                                (false, true, false) => "declaration",
                                (false, true, true) => "declaration-candidate",
                                (false, false, false) => "declaration-link",
                                (false, false, true) => "declaration-candidate-link",
                            }
                        }
                        Resolved::External => "external-or-unavailable",
                        Resolved::Ambiguous => "ambiguous",
                        Resolved::Inherited => "declaration-unresolved",
                        Resolved::Incomplete => "input-incomplete",
                    }
                }
            };
            doc.calls.try_reserve(1).map_err(InputError::Allocation)?;
            doc.calls.push(Call {
                caller: caller.clone(),
                offset_byte: edge.offset,
                mnemonic: edge.mnemonic,
                symbol: edge.symbol,
                target,
                resolution,
                depth: next_level,
            });
        }
    }
    if doc.issues.len() > 1 {
        let mut seen = HashSet::new();
        seen.try_reserve(doc.issues.len())
            .map_err(InputError::Allocation)?;
        doc.issues.retain(|issue| seen.insert(issue.clone()));
    }
    Ok(doc)
}

type Pending = VecDeque<(MethodKey, usize, Option<Vec<Edge>>)>;

struct PreparedMethod {
    declaration: Declaration,
    pending: Option<(MethodKey, Vec<Edge>)>,
    issues: Vec<Issue>,
}

impl PreparedMethod {
    fn read(
        class: &ClassFile<'_>,
        location: &Location,
        ordinal: usize,
        method: Result<Member<'_>, diana_decomp::input::jvm::ClassError>,
        follow: bool,
    ) -> Result<Self, QueryError> {
        let at = format!("{location}:m{ordinal}");
        let mut issues = Vec::new();
        let (symbol, access_flags, pending) = match method {
            Ok(method) => {
                let key = MethodKey {
                    class: location.clone(),
                    ordinal: method.ordinal(),
                };
                let symbol = declaration_symbol(class, method, false, &at, &mut issues)?;
                let edges = call_edges(class, method, &at, follow, &mut issues)?;
                (symbol, Some(method.access_flags()), Some((key, edges)))
            }
            Err(error) => {
                push_issue(&mut issues, &at, &class_error(error), None)?;
                (None, None, None)
            }
        };
        Ok(Self {
            declaration: Declaration {
                kind: "method",
                at,
                symbol,
                access_flags,
            },
            pending,
            issues,
        })
    }

    fn register(
        self,
        doc: &mut Outline,
        work: &mut Pending,
        scheduled: &mut HashSet<MethodKey>,
    ) -> Result<(), QueryError> {
        doc.members.try_reserve(1).map_err(InputError::Allocation)?;
        doc.members.push(self.declaration);
        doc.issues
            .try_reserve(self.issues.len())
            .map_err(InputError::Allocation)?;
        doc.issues.extend(self.issues);
        if let Some((key, edges)) = self.pending {
            scheduled.try_reserve(1).map_err(InputError::Allocation)?;
            scheduled.insert(key.clone());
            work.try_reserve(1).map_err(InputError::Allocation)?;
            work.push_back((key, 0, Some(edges)));
        }
        Ok(())
    }
}

fn push_declaration(
    members: &mut Vec<Declaration>,
    kind: &'static str,
    at: String,
    symbol: Option<String>,
    access_flags: Option<u16>,
) -> Result<(), QueryError> {
    members.try_reserve(1).map_err(InputError::Allocation)?;
    members.push(Declaration {
        kind,
        at,
        symbol,
        access_flags,
    });
    Ok(())
}

fn declaration_symbol(
    class: &ClassFile<'_>,
    member: Member<'_>,
    field: bool,
    at: &str,
    issues: &mut Vec<Issue>,
) -> Result<Option<String>, QueryError> {
    let symbol = (|| {
        let name = display(member.name(class).map_err(class_error)?)?;
        let descriptor = display(member.descriptor(class).map_err(class_error)?)?;
        let separator = if field { ":" } else { "" };
        Ok(format!("{name}{separator}{descriptor}"))
    })();
    match symbol {
        Ok(symbol) => Ok(Some(symbol)),
        Err(error) => {
            push_issue(issues, at, &error, None)?;
            Ok(None)
        }
    }
}

fn member<'a>(class: &ClassFile<'a>, ordinal: u16) -> Result<Member<'a>, QueryError> {
    for member in class.methods().map_err(class_error)? {
        let member = member.map_err(class_error)?;
        if member.ordinal() == ordinal {
            return Ok(member);
        }
    }
    Err(local(
        FailureKind::Unsupported,
        format_args!("no JVM method at ordinal {ordinal}"),
    ))
}

fn call_edges<'a>(
    class: &ClassFile<'a>,
    method: Member<'a>,
    context: &str,
    follow: bool,
    issues: &mut Vec<Issue>,
) -> Result<Vec<Edge>, QueryError> {
    let Some(code) = (match method_code(class, method) {
        Ok(code) => code,
        Err(error) => {
            push_issue(issues, context, &error, None)?;
            None
        }
    }) else {
        return Ok(Vec::new());
    };
    if let Some(missing) = code.missing_bytes() {
        push_issue(
            issues,
            context,
            &local(FailureKind::Malformed, "truncated JVM Code bytes"),
            Some([usize::from(missing.start), usize::from(missing.end)]),
        )?;
    }
    let mut edges = Vec::new();
    for item in code.instructions() {
        let instruction = match item {
            Ok(instruction) => instruction,
            Err(error) => {
                let range = [usize::from(error.range.start), usize::from(error.range.end)];
                push_issue(issues, context, &local(error.kind(), error), Some(range))?;
                continue;
            }
        };
        if !matches!(instruction.opcode(), 0xb6..=0xba) {
            continue;
        }
        let offset = instruction.offset();
        let start = usize::from(offset);
        let range = Some([start, start + instruction.bytes().len()]);
        let operand_problem = instruction.operand_problem();
        if let Some(problem) = operand_problem {
            push_issue(
                issues,
                context,
                &local(FailureKind::Malformed, format_args!("{problem:?}")),
                range,
            )?;
        }
        let (symbol, lookup, unresolved) = match instruction.reference(class) {
            Ok(Some(Reference::Member(member))) => {
                let symbol = format!(
                    "{}->{}{}",
                    type_text(member.owner)?,
                    display(member.name)?,
                    display(member.descriptor)?,
                );
                let invoke = match instruction.opcode() {
                    0xb8 => Invoke::Static,
                    0xb7 if member.name.bytes() == b"<init>" => Invoke::Constructor,
                    0xb7 => Invoke::Special,
                    0xb9 => Invoke::Interface,
                    _ => Invoke::Virtual,
                };
                let lookup = if follow && operand_problem.is_none() {
                    Some(MemberSymbol {
                        owner: copy(member.owner.bytes())?,
                        name: copy(member.name.bytes())?,
                        descriptor: copy(member.descriptor.bytes())?,
                        invoke,
                    })
                } else {
                    None
                };
                let unresolved = match invoke {
                    Invoke::Static | Invoke::Constructor => "depth-limit",
                    Invoke::Special => "special-dispatch",
                    Invoke::Interface => "interface",
                    Invoke::Virtual => "virtual",
                };
                (symbol, lookup, unresolved)
            }
            Ok(Some(Reference::Dynamic {
                name,
                descriptor,
                bootstrap_index,
                ..
            })) => (
                format!(
                    "{}{} [bootstrap {bootstrap_index}]",
                    display(name)?,
                    display(descriptor)?
                ),
                None,
                "dynamic",
            ),
            Ok(_) => {
                push_issue(
                    issues,
                    context,
                    &local(
                        FailureKind::InternalInvariant,
                        "invoke opcode has no JVM symbolic operand",
                    ),
                    range,
                )?;
                continue;
            }
            Err(error) => {
                push_issue(issues, context, &local(error.kind(), error), range)?;
                continue;
            }
        };
        edges.try_reserve(1).map_err(InputError::Allocation)?;
        edges.push(Edge {
            offset,
            mnemonic: instruction.mnemonic(),
            symbol,
            lookup,
            unresolved: if operand_problem.is_some() {
                "malformed-operands"
            } else {
                unresolved
            },
        });
    }
    Ok(edges)
}

enum Resolved {
    Method(MethodKey, bool),
    External,
    Ambiguous,
    Inherited,
    Incomplete,
}

#[derive(Default)]
struct Resolver {
    owners: Option<HashMap<Vec<u8>, Option<Location>>>,
    incomplete_owners: bool,
    methods: HashMap<Location, MethodCatalog>,
}

type MemberNames = HashMap<Vec<u8>, HashMap<Vec<u8>, Option<MethodEntry>>>;

#[derive(Clone, Copy)]
struct MethodEntry {
    ordinal: u16,
    flags: u16,
}

impl MethodEntry {
    fn matches_invocation(self, invoke: Invoke, inherited: bool) -> bool {
        (self.flags & 0x0008 != 0) == (invoke == Invoke::Static)
            && (!inherited || self.flags & 0x0002 == 0)
    }
}

#[derive(Default)]
struct MethodCatalog {
    ordinals: MemberNames,
    incomplete: bool,
    interface: bool,
    superclass: Option<Vec<u8>>,
    interfaces: Vec<Vec<u8>>,
    parent_gaps: Vec<diana_decomp::input::jvm::ClassError>,
    parents_incomplete: bool,
}

impl MethodCatalog {
    fn declaration(&self, symbol: &MemberSymbol) -> Option<Option<MethodEntry>> {
        self.ordinals
            .get(&symbol.name)?
            .get(&symbol.descriptor)
            .copied()
    }

    fn report_parent_gaps(
        &mut self,
        location: &Location,
        issues: &mut Vec<Issue>,
    ) -> Result<(), QueryError> {
        for error in self.parent_gaps.drain(..) {
            push_issue(issues, &location.to_string(), &class_error(error), None)?;
        }
        Ok(())
    }
}

impl Resolver {
    fn resolve(
        &mut self,
        roots: &JvmRoots<'_>,
        reader: &mut JvmReader,
        jobs: usize,
        symbol: &MemberSymbol,
        issues: &mut Vec<Issue>,
    ) -> Result<Resolved, QueryError> {
        if self.owners.is_none() {
            self.index_owners(roots, jobs, issues)?;
        }
        let mut location = match self.owner(&symbol.owner) {
            Ok(location) => location,
            Err(resolution) => return Ok(resolution),
        };
        let mut visited = HashSet::new();
        let mut interfaces = VecDeque::new();
        let mut incomplete = self.incomplete_owners;
        loop {
            let inherited = !visited.is_empty();
            let catalog = self.catalog(roots, reader, &location, issues)?;
            incomplete |= catalog.incomplete;
            match catalog.declaration(symbol) {
                Some(Some(method)) if method.matches_invocation(symbol.invoke, inherited) => {
                    return Ok(Resolved::Method(
                        MethodKey {
                            class: location,
                            ordinal: method.ordinal,
                        },
                        incomplete,
                    ));
                }
                Some(None) => return Ok(Resolved::Ambiguous),
                Some(Some(_)) => return Ok(Resolved::Inherited),
                None => {}
            }
            if symbol.invoke == Invoke::Constructor
                || symbol.invoke == Invoke::Static && catalog.interface
            {
                return Ok(if incomplete {
                    Resolved::Incomplete
                } else {
                    Resolved::Inherited
                });
            }
            visited.try_reserve(1).map_err(InputError::Allocation)?;
            if !visited.insert(location.clone()) {
                push_issue(
                    issues,
                    &location.to_string(),
                    &local(FailureKind::Malformed, "cyclic JVM superclass declarations"),
                    None,
                )?;
                return Ok(Resolved::Incomplete);
            }
            catalog.report_parent_gaps(&location, issues)?;
            incomplete |= catalog.parents_incomplete;
            if symbol.invoke != Invoke::Static {
                interfaces
                    .try_reserve(catalog.interfaces.len())
                    .map_err(InputError::Allocation)?;
                for parent in &catalog.interfaces {
                    interfaces.push_back(copy(parent)?);
                }
            }
            let Some(parent) = catalog.superclass.as_deref().map(copy).transpose()? else {
                break;
            };
            match self.owner(&parent) {
                Ok(parent) => location = parent,
                Err(Resolved::Ambiguous) => return Ok(Resolved::Ambiguous),
                Err(_) => {
                    incomplete = true;
                    break;
                }
            }
        }
        self.resolve_interfaces(roots, reader, symbol, interfaces, incomplete, issues)
    }

    fn owner(&self, name: &[u8]) -> Result<Location, Resolved> {
        match self.owners.as_ref().and_then(|owners| owners.get(name)) {
            Some(Some(location)) => Ok(location.clone()),
            Some(None) => Err(Resolved::Ambiguous),
            None if self.incomplete_owners => Err(Resolved::Incomplete),
            None => Err(Resolved::External),
        }
    }

    fn resolve_interfaces(
        &mut self,
        roots: &JvmRoots<'_>,
        reader: &mut JvmReader,
        symbol: &MemberSymbol,
        mut interfaces: VecDeque<Vec<u8>>,
        mut incomplete: bool,
        issues: &mut Vec<Issue>,
    ) -> Result<Resolved, QueryError> {
        let mut seen = HashSet::new();
        let mut candidates = Vec::new();
        let mut parents = HashMap::new();
        while let Some(name) = interfaces.pop_front() {
            seen.try_reserve(1).map_err(InputError::Allocation)?;
            if !seen.insert(name.clone()) {
                continue;
            }
            let location = match self.owner(&name) {
                Ok(location) => location,
                Err(Resolved::Ambiguous) => return Ok(Resolved::Ambiguous),
                Err(_) => {
                    incomplete = true;
                    continue;
                }
            };
            let catalog = self.catalog(roots, reader, &location, issues)?;
            if !catalog.interface {
                push_issue(
                    issues,
                    &location.to_string(),
                    &local(
                        FailureKind::Malformed,
                        "JVM superinterface resolves to a class",
                    ),
                    None,
                )?;
                incomplete = true;
                continue;
            }
            incomplete |= catalog.incomplete;
            match catalog.declaration(symbol) {
                Some(Some(method)) if method.matches_invocation(symbol.invoke, true) => {
                    candidates.try_reserve(1).map_err(InputError::Allocation)?;
                    candidates.push((
                        copy(&name)?,
                        MethodKey {
                            class: location.clone(),
                            ordinal: method.ordinal,
                        },
                    ));
                }
                Some(None) => return Ok(Resolved::Ambiguous),
                _ => {}
            }
            catalog.report_parent_gaps(&location, issues)?;
            incomplete |= catalog.parents_incomplete;
            let mut supers = Vec::new();
            supers
                .try_reserve_exact(catalog.interfaces.len())
                .map_err(InputError::Allocation)?;
            interfaces
                .try_reserve(catalog.interfaces.len())
                .map_err(InputError::Allocation)?;
            for parent in &catalog.interfaces {
                supers.push(copy(parent)?);
                interfaces.push_back(copy(parent)?);
            }
            parents.try_reserve(1).map_err(InputError::Allocation)?;
            parents.insert(name, supers);
        }
        if candidates.len() > 1 {
            // An interface declaration dominates another only along an
            // established original superinterface path.
            let mut dominated = HashSet::new();
            for (name, _) in &candidates {
                let mut seen = HashSet::new();
                let mut work = Vec::new();
                if let Some(supers) = parents.get(name) {
                    work.try_reserve(supers.len())
                        .map_err(InputError::Allocation)?;
                    work.extend(supers);
                }
                while let Some(parent) = work.pop() {
                    seen.try_reserve(1).map_err(InputError::Allocation)?;
                    if !seen.insert(parent) {
                        continue;
                    }
                    if parent == name {
                        let location = self.owner(name).map_err(|_| QueryError::WorkerLost)?;
                        push_issue(
                            issues,
                            &location.to_string(),
                            &local(
                                FailureKind::Malformed,
                                "cyclic JVM superinterface declarations",
                            ),
                            None,
                        )?;
                        return Ok(Resolved::Incomplete);
                    }
                    dominated.try_reserve(1).map_err(InputError::Allocation)?;
                    dominated.insert(parent);
                    if let Some(supers) = parents.get(parent) {
                        work.try_reserve(supers.len())
                            .map_err(InputError::Allocation)?;
                        work.extend(supers);
                    }
                }
            }
            candidates.retain(|(name, _)| !dominated.contains(name));
        }
        Ok(match candidates.as_mut_slice() {
            [(_, _)] => Resolved::Method(
                candidates.pop().ok_or(QueryError::WorkerLost)?.1,
                incomplete,
            ),
            [] if incomplete => Resolved::Incomplete,
            [] => Resolved::Inherited,
            _ => Resolved::Ambiguous,
        })
    }

    fn catalog(
        &mut self,
        roots: &JvmRoots<'_>,
        reader: &mut JvmReader,
        location: &Location,
        issues: &mut Vec<Issue>,
    ) -> Result<&mut MethodCatalog, QueryError> {
        if !self.methods.contains_key(location) {
            self.methods
                .try_reserve(1)
                .map_err(InputError::Allocation)?;
            let root = roots.by_location(location).ok_or(QueryError::WorkerLost)?;
            let mut catalog = MethodCatalog::default();
            match reader.read(root) {
                Ok(class) => {
                    catalog.interface = class.access_flags() & 0x0200 != 0;
                    let mut parent_issue = |error| {
                        catalog
                            .parent_gaps
                            .try_reserve(1)
                            .map_err(InputError::Allocation)?;
                        catalog.parent_gaps.push(error);
                        catalog.parents_incomplete = true;
                        Ok::<_, QueryError>(())
                    };
                    if !catalog.interface {
                        match class.super_name() {
                            Ok(Some(parent)) => catalog.superclass = Some(copy(parent.bytes())?),
                            Ok(None) => {}
                            Err(error) => parent_issue(error)?,
                        }
                    }
                    for parent in class.interfaces() {
                        match parent {
                            Ok(parent) => {
                                catalog
                                    .interfaces
                                    .try_reserve(1)
                                    .map_err(InputError::Allocation)?;
                                catalog.interfaces.push(copy(parent.bytes())?);
                            }
                            Err(error) => parent_issue(error)?,
                        }
                    }
                    match class.methods() {
                        Ok(members) => {
                            for item in members {
                                let row = item.and_then(|member| {
                                    Ok((
                                        member.name(&class)?,
                                        member.descriptor(&class)?,
                                        MethodEntry {
                                            ordinal: member.ordinal(),
                                            flags: member.access_flags(),
                                        },
                                    ))
                                });
                                match row {
                                    Ok((name, descriptor, method)) => {
                                        catalog
                                            .ordinals
                                            .try_reserve(1)
                                            .map_err(InputError::Allocation)?;
                                        let overloads = catalog
                                            .ordinals
                                            .entry(copy(name.bytes())?)
                                            .or_default();
                                        overloads.try_reserve(1).map_err(InputError::Allocation)?;
                                        overloads
                                            .entry(copy(descriptor.bytes())?)
                                            .and_modify(|entry| *entry = None)
                                            .or_insert(Some(method));
                                    }
                                    Err(error) => {
                                        catalog.incomplete = true;
                                        push_issue(
                                            issues,
                                            &location.to_string(),
                                            &class_error(error),
                                            None,
                                        )?;
                                    }
                                }
                            }
                        }
                        Err(error) => {
                            catalog.incomplete = true;
                            push_issue(issues, &location.to_string(), &class_error(error), None)?;
                        }
                    }
                }
                Err(error) => {
                    catalog.incomplete = true;
                    push_issue(issues, &location.to_string(), &error, None)?;
                }
            }
            self.methods.insert(location.clone(), catalog);
        }
        self.methods.get_mut(location).ok_or(QueryError::WorkerLost)
    }

    fn index_owners(
        &mut self,
        roots: &JvmRoots<'_>,
        jobs: usize,
        issues: &mut Vec<Issue>,
    ) -> Result<(), QueryError> {
        if jobs == 0 {
            return Err(QueryError::InvalidJobs);
        }
        let mut owners = HashMap::<Vec<u8>, Option<Location>>::new();
        self.incomplete_owners = !roots.issues().is_empty();
        runner::run::<_, _, _, QueryError>(
            roots.len(),
            jobs,
            || roots.reader(),
            |reader, index, emit| {
                let root = roots.get(index).ok_or(QueryError::WorkerLost)?;
                let name = reader
                    .read(root)
                    .and_then(|class| copy(class.name().map_err(class_error)?.bytes()));
                emit((root.location().clone(), name))
            },
            &mut |(location, name)| {
                match name {
                    Ok(name) => {
                        owners.try_reserve(1).map_err(InputError::Allocation)?;
                        owners
                            .entry(name)
                            .and_modify(|entry| *entry = None)
                            .or_insert(Some(location));
                    }
                    Err(error) => {
                        self.incomplete_owners = true;
                        push_issue(issues, &location.to_string(), &error, None)?;
                    }
                }
                Ok(())
            },
        )?;
        self.owners = Some(owners);
        Ok(())
    }
}

fn copy(bytes: &[u8]) -> Result<Vec<u8>, QueryError> {
    let mut output = Vec::new();
    output
        .try_reserve_exact(bytes.len())
        .map_err(InputError::Allocation)?;
    output.extend_from_slice(bytes);
    Ok(output)
}

fn local(kind: FailureKind, error: impl std::fmt::Display) -> QueryError {
    QueryError::Local {
        kind,
        reason: error.to_string(),
    }
}

fn push_issue(
    issues: &mut Vec<Issue>,
    context: &str,
    error: &QueryError,
    bytes: Option<[usize; 2]>,
) -> Result<(), QueryError> {
    issues.try_reserve(1).map_err(InputError::Allocation)?;
    issues.push(Issue::new(context.to_owned(), error, bytes));
    Ok(())
}
