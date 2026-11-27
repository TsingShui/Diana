//! Reference queries over original JVM instruction and declaration sites.

use std::borrow::Cow;
use std::collections::HashSet;
use std::sync::Arc;

use diana_decomp::FailureKind;
use diana_decomp::input::jvm::bytecode::{
    Code, Reference, ReferenceError, bootstrap_argument_reference, method_handle,
};
use diana_decomp::input::jvm::{
    BootstrapMethod, ClassError, ClassFile, Constant, JvmString, Member, MemberReference,
    ReferenceKind,
};

use super::at;
use super::filter::{FilterMatch, Relevance, TextFilter};
use super::input::InputError;
use super::jvm::{JvmRoots, Location};
use super::query::QueryError;
use super::runner;
use super::xref::{self, Issue, MethodRelation, Relation, Target};

mod callers;
mod metadata;
use callers::{Caller, Callers, Frontier};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) enum MemberSite {
    Method(u16),
    Field(u16),
}

pub(crate) struct Hit {
    pub location: Location,
    pub member: Option<MemberSite>,
    pub offset_byte: Option<u16>,
    pub matched: String,
    pub use_: Cow<'static, str>,
    pub depth: u32,
    pub(super) caller: Option<Arc<Caller>>,
    pub relevance: Option<Relevance>,
}

impl Hit {
    pub(crate) fn context(&self) -> String {
        match self.member {
            Some(MemberSite::Method(index)) => format!("{}:m{index}", self.location),
            Some(MemberSite::Field(index)) => format!("{}:f{index}", self.location),
            None => self.location.to_string(),
        }
    }
}

pub(crate) enum Event {
    Hit(Hit),
    Issue(Issue),
}

pub(crate) struct Plan<'a> {
    pub target: &'a Target,
    pub class: Option<&'a [u16]>,
    pub jobs: usize,
    pub typed_uses: bool,
    pub parameter_types: bool,
    pub recursive: bool,
    pub method_relation: MethodRelation,
    pub transitive: u32,
}

#[derive(Clone, Copy)]
enum Search<'a> {
    Initial(&'a Target),
    Caller(&'a Frontier),
}

struct Level<'a> {
    search: Search<'a>,
    class: Option<&'a [u16]>,
    jobs: usize,
    typed_uses: bool,
    parameter_types: bool,
    depth: u32,
    track_callers: bool,
}

pub(crate) fn scan(
    roots: &JvmRoots<'_>,
    plan: &Plan<'_>,
    sink: &mut impl FnMut(Event) -> Result<(), QueryError>,
) -> Result<(), QueryError> {
    if plan.jobs == 0 {
        return Err(QueryError::InvalidJobs);
    }
    for issue in roots.issues() {
        sink(Event::Issue(Issue {
            context: issue.context.clone(),
            kind: issue.kind.clone(),
            reason: issue.reason.clone(),
            range_bytes: None,
        }))?;
    }
    if plan.recursive || plan.method_relation != MethodRelation::References {
        return xref::jvm_relations(
            roots,
            plan.target,
            plan.class,
            plan.method_relation,
            plan.jobs,
            sink,
        );
    }
    let limit = plan.transitive.max(1);
    let first = Level {
        search: Search::Initial(plan.target),
        class: plan.class,
        jobs: plan.jobs,
        typed_uses: plan.typed_uses,
        parameter_types: plan.parameter_types,
        depth: 1,
        track_callers: limit > 1,
    };
    if limit == 1 {
        return scan_level(roots, &first, sink);
    }

    let mut callers = Callers::default();
    let mut issues = HashSet::new();
    let mut run = |level: &Level<'_>, callers: &mut Callers| {
        scan_level(roots, level, &mut |mut event| {
            match &mut event {
                Event::Hit(hit) => {
                    if let Some(caller) = hit.caller.take() {
                        callers.reach(caller, hit.relevance)?;
                    }
                }
                Event::Issue(issue) => {
                    issues.try_reserve(1).map_err(InputError::Allocation)?;
                    if !issues.insert(issue.clone()) {
                        return Ok(());
                    }
                }
            }
            sink(event)
        })
    };
    run(&first, &mut callers)?;
    for depth in 2..=limit {
        let Some(frontier) = callers.advance()? else {
            break;
        };
        run(
            &Level {
                search: Search::Caller(&frontier),
                class: None,
                jobs: plan.jobs,
                typed_uses: false,
                parameter_types: false,
                depth,
                track_callers: depth < limit,
            },
            &mut callers,
        )?;
    }
    Ok(())
}

fn scan_level(
    roots: &JvmRoots<'_>,
    plan: &Level<'_>,
    sink: &mut impl FnMut(Event) -> Result<(), QueryError>,
) -> Result<(), QueryError> {
    runner::run(
        roots.len(),
        plan.jobs,
        || roots.reader(),
        |reader, ordinal, emit| {
            let root = roots.get(ordinal).ok_or(QueryError::WorkerLost)?;
            let location = root.location();
            let class = match reader.read(root) {
                Ok(class) => class,
                Err(error) => {
                    return emit(Event::Issue(Issue::new(location.to_string(), &error, None)));
                }
            };
            scan_class(&class, location, plan, emit)
        },
        sink,
    )
}

pub(crate) fn class_error(error: ClassError) -> QueryError {
    error.into()
}

fn local(kind: FailureKind, problem: impl std::fmt::Display) -> QueryError {
    QueryError::Local {
        kind,
        reason: problem.to_string(),
    }
}

fn report(
    emit: &mut dyn FnMut(Event) -> Result<(), QueryError>,
    context: impl FnOnce() -> String,
    error: QueryError,
    range: Option<[usize; 2]>,
) -> Result<(), QueryError> {
    emit(Event::Issue(Issue::new(context(), &error, range)))
}

pub(crate) fn method_code<'a>(
    class: &ClassFile<'a>,
    member: Member<'a>,
) -> Result<Option<Code<'a>>, QueryError> {
    let attribute = class.code_attribute(member).map_err(class_error)?;
    match attribute {
        Some(attribute) => Code::parse_at(attribute.payload, attribute.offset)
            .map(Some)
            .map_err(|error| local(FailureKind::Malformed, error)),
        None if member.access_flags() & (0x0100 | 0x0400) != 0 => Ok(None),
        None => Err(local(
            FailureKind::Malformed,
            "method has no Code attribute",
        )),
    }
}

fn scan_class<'a>(
    class: &ClassFile<'a>,
    location: &Location,
    plan: &Level<'_>,
    emit: &mut dyn FnMut(Event) -> Result<(), QueryError>,
) -> Result<(), QueryError> {
    if let Some(name) = plan.class {
        match class.name() {
            Ok(actual) if type_units(actual).eq(name.iter().copied()) => {}
            Ok(_) => return Ok(()),
            Err(error) => return report(emit, || location.to_string(), class_error(error), None),
        }
    }
    let base_site = Site {
        location,
        member: None,
        offset_byte: None,
        depth: plan.depth,
    };
    match plan.search {
        Search::Initial(Target::Hierarchy { relation, target }) => {
            return match relation {
                Relation::Extend => match class.super_name() {
                    Ok(Some(name)) if class.access_flags() & 0x0200 == 0 => {
                        type_hit(target, name, "extends", base_site, emit)
                    }
                    Ok(Some(_)) => Ok(()),
                    Ok(None) => Ok(()),
                    Err(error) => report(emit, || location.to_string(), class_error(error), None),
                },
                Relation::Impl => {
                    let label = if class.access_flags() & 0x0200 == 0 {
                        "implements"
                    } else {
                        "extends-interface"
                    };
                    for name in class.interfaces() {
                        match name {
                            Ok(name) => type_hit(target, name, label, base_site, emit)?,
                            Err(error) => {
                                report(emit, || location.to_string(), class_error(error), None)?
                            }
                        }
                    }
                    Ok(())
                }
            };
        }
        Search::Initial(Target::Type(filter)) => {
            // Declarations remain readable even if the same class has one
            // malformed method body or an unavailable superclass.
            match class.super_name() {
                Ok(Some(name)) if class.access_flags() & 0x0200 == 0 => {
                    type_hit(filter, name, "extends", base_site, emit)?;
                }
                Ok(_) => {}
                Err(error) => report(emit, || location.to_string(), class_error(error), None)?,
            }
            for name in class.interfaces() {
                match name {
                    Ok(name) => type_hit(filter, name, "implements", base_site, emit)?,
                    Err(error) => report(emit, || location.to_string(), class_error(error), None)?,
                }
            }
        }
        _ => {}
    }
    let pool_match = pool_can_match(class, plan.search)?;
    let metadata = metadata::Metadata::new(class, plan.search, pool_match);
    let type_filter = match plan.search {
        Search::Initial(Target::Type(filter)) => Some(filter),
        _ => None,
    };
    if let Some(metadata) = &metadata {
        metadata.read(class, None, base_site, emit)?;
    }
    if metadata.is_some() || type_filter.is_some() {
        match class.fields() {
            Ok(fields) => {
                for field in fields {
                    let field = match field {
                        Ok(field) => field,
                        Err(error) => {
                            report(emit, || base_site.context(), class_error(error), None)?;
                            continue;
                        }
                    };
                    let site = base_site.with_member(MemberSite::Field(field.ordinal()));
                    if let Some(filter) = type_filter {
                        match field.descriptor(class) {
                            Ok(value) => {
                                signature_hits(filter, value, false, "field-type", site, emit)?
                            }
                            Err(error) => {
                                report(emit, || site.context(), class_error(error), None)?
                            }
                        }
                    }
                    if let Some(metadata) = &metadata {
                        metadata.field(class, field, site, emit)?;
                    }
                }
            }
            Err(error) => report(emit, || base_site.context(), class_error(error), None)?,
        }
    }
    if !pool_match
        && !metadata
            .as_ref()
            .is_some_and(|metadata| metadata.annotations)
    {
        return Ok(());
    }
    let methods = match class.methods() {
        Ok(methods) => methods,
        Err(error) => return report(emit, || location.to_string(), class_error(error), None),
    };
    let mut bootstraps = None;
    for member in methods {
        let member = match member {
            Ok(member) => member,
            Err(error) => {
                report(emit, || location.to_string(), class_error(error), None)?;
                continue;
            }
        };
        let site = base_site.with_member(MemberSite::Method(member.ordinal()));
        if let Search::Initial(Target::Type(filter)) = plan.search {
            match member.descriptor(class) {
                Ok(value) => signature_hits(
                    filter,
                    value,
                    plan.parameter_types,
                    "method-type",
                    site,
                    emit,
                )?,
                Err(error) => report(emit, || site.context(), class_error(error), None)?,
            }
        }
        if let Some(metadata) = &metadata {
            metadata.read(class, Some(member), site, emit)?;
        }
        if !pool_match {
            continue;
        }
        match method_code(class, member) {
            Ok(Some(code)) if plan.track_callers => {
                // Materialize a caller identity only when this method contributes
                // a recovered reference. Repeated hits share that identity.
                let mut caller = None::<Option<Arc<Caller>>>;
                scan_code(
                    class,
                    code,
                    site,
                    plan,
                    &mut bootstraps,
                    &mut |mut event| {
                        if let Event::Hit(hit) = &mut event {
                            if caller.is_none() {
                                caller = Some(match Caller::read(class, member) {
                                    Ok(value) => Some(Arc::new(value)),
                                    Err(error) => {
                                        report(emit, || site.context(), error, None)?;
                                        None
                                    }
                                });
                            }
                            hit.caller = caller.as_ref().and_then(|caller| caller.clone());
                        }
                        emit(event)
                    },
                )?;
            }
            Ok(Some(code)) => scan_code(class, code, site, plan, &mut bootstraps, emit)?,
            Ok(None) => {}
            Err(error) => report(emit, || site.context(), error, None)?,
        }
    }
    Ok(())
}

/// JVM reference instructions and bootstrap arguments can only obtain
/// these values from their own class's constant pool. Even an unavailable
/// instruction suffix cannot reference a matching entry absent from the pool.
fn pool_can_match(class: &ClassFile<'_>, search: Search<'_>) -> Result<bool, QueryError> {
    match search {
        Search::Initial(target @ (Target::String(_) | Target::JvmResource(_))) => {
            for item in class.strings() {
                let Ok((_, text)) = item else {
                    // Keep scanning original uses when a possible match is unavailable.
                    return Ok(true);
                };
                if match_literal(target, text)?.is_some() {
                    return Ok(true);
                }
            }
        }
        Search::Initial(Target::Method { .. } | Target::Field { .. }) | Search::Caller(_) => {
            let field = matches!(search, Search::Initial(Target::Field { .. }));
            for (index, tag) in class.constant_tags() {
                if (field && tag != 9) || (!field && !matches!(tag, 10 | 11)) {
                    continue;
                }
                let Ok(reference) = class.member_reference(index) else {
                    return Ok(true);
                };
                if match_search(search, reference)?.is_some() {
                    return Ok(true);
                }
            }
        }
        _ => return Ok(true),
    }
    Ok(false)
}

#[derive(Clone, Copy)]
struct Site<'a> {
    location: &'a Location,
    member: Option<MemberSite>,
    offset_byte: Option<u16>,
    depth: u32,
}

impl Site<'_> {
    fn with_member(self, member: MemberSite) -> Self {
        Self {
            member: Some(member),
            ..self
        }
    }

    fn at_byte(self, offset: u16) -> Self {
        Self {
            offset_byte: Some(offset),
            ..self
        }
    }

    fn context(self) -> String {
        match self.member {
            Some(MemberSite::Method(index)) => format!("{}:m{index}", self.location),
            Some(MemberSite::Field(index)) => format!("{}:f{index}", self.location),
            None => self.location.to_string(),
        }
    }

    fn hit(
        self,
        matched: String,
        use_: impl Into<Cow<'static, str>>,
        relevance: Option<Relevance>,
        emit: &mut dyn FnMut(Event) -> Result<(), QueryError>,
    ) -> Result<(), QueryError> {
        emit(Event::Hit(Hit {
            location: self.location.clone(),
            member: self.member,
            offset_byte: self.offset_byte,
            matched,
            use_: use_.into(),
            relevance,
            depth: self.depth,
            caller: None,
        }))
    }
}

fn scan_code<'a>(
    class: &ClassFile<'a>,
    code: Code<'a>,
    site: Site<'_>,
    plan: &Level<'_>,
    bootstraps: &mut Option<Bootstraps<'a>>,
    emit: &mut dyn FnMut(Event) -> Result<(), QueryError>,
) -> Result<(), QueryError> {
    if let Some(missing) = code.missing_bytes() {
        report(
            emit,
            || site.context(),
            local(FailureKind::Malformed, "truncated JVM Code bytes"),
            Some([usize::from(missing.start), usize::from(missing.end)]),
        )?;
    } else if let Search::Initial(Target::Type(filter)) = plan.search {
        match code.exception_handlers() {
            Ok(handlers) => {
                if let Some(error) = handlers.issue() {
                    report(
                        emit,
                        || site.context(),
                        local(FailureKind::Malformed, error),
                        None,
                    )?;
                }
                for handler in handlers {
                    if let Some(problem) = handler.range_problem(code) {
                        report(
                            emit,
                            || site.context(),
                            local(FailureKind::Malformed, format_args!("{problem:?}")),
                            None,
                        )?;
                    }
                    if let Some(index) = handler.catch_type {
                        match class.class_name(index) {
                            Ok(name) => type_hit(
                                filter,
                                name,
                                "catch-type",
                                site.at_byte(handler.target),
                                emit,
                            )?,
                            Err(error) => {
                                report(emit, || site.context(), class_error(error), None)?
                            }
                        }
                    }
                }
            }
            Err(error) => report(
                emit,
                || site.context(),
                local(FailureKind::Malformed, error),
                None,
            )?,
        }
    }
    for item in code.instructions() {
        let insn = match item {
            Ok(insn) => insn,
            Err(error) => {
                let range = [usize::from(error.range.start), usize::from(error.range.end)];
                report(
                    emit,
                    || site.context(),
                    local(error.kind(), error),
                    Some(range),
                )?;
                continue;
            }
        };
        if !may_reference(plan, insn.opcode()) {
            continue;
        }
        let position = site.at_byte(insn.offset());
        let interval = || {
            let start = usize::from(insn.offset());
            Some([start, start + insn.bytes().len()])
        };
        if let Some(problem) = insn.operand_problem() {
            report(
                emit,
                || site.context(),
                local(FailureKind::Malformed, format_args!("{problem:?}")),
                interval(),
            )?;
        }
        match insn.reference(class) {
            Ok(Some(Reference::Dynamic {
                bootstrap_index,
                descriptor,
                ..
            })) => {
                if let Search::Initial(Target::Type(filter)) = plan.search
                    && plan.typed_uses
                {
                    signature_hits(filter, descriptor, true, "dynamic-type", position, emit)?;
                }
                bootstraps
                    .get_or_insert_with(|| Bootstraps::read(class))
                    .scan(class, bootstrap_index, position, interval(), plan, emit)?;
            }
            Ok(Some(reference)) => {
                reference_hit(reference, insn.mnemonic(), position, plan, emit)?;
            }
            Ok(None) => {}
            Err(error) => report(
                emit,
                || site.context(),
                local(error.kind(), error),
                interval(),
            )?,
        }
    }
    Ok(())
}

fn may_reference(plan: &Level<'_>, opcode: u8) -> bool {
    match plan.search {
        Search::Initial(Target::String(_) | Target::JvmResource(_)) => {
            matches!(opcode, 0x12..=0x14 | 0xba)
        }
        Search::Initial(Target::Method { .. }) | Search::Caller(_) => {
            matches!(opcode, 0x12..=0x14 | 0xb6..=0xba)
        }
        Search::Initial(Target::Field { .. }) => matches!(opcode, 0x12..=0x14 | 0xb2..=0xb5 | 0xba),
        Search::Initial(Target::Type(_)) => {
            matches!(
                opcode,
                0x12..=0x14 | 0xba | 0xbb | 0xbd | 0xc0 | 0xc1 | 0xc5
            ) || plan.typed_uses && matches!(opcode, 0xb2..=0xb9)
        }
        _ => false,
    }
}

#[derive(Default)]
struct Bootstraps<'a> {
    rows: Vec<BootstrapMethod<'a>>,
    issue: Option<QueryError>,
    pending: Vec<u16>,
    seen: std::collections::HashSet<u16>,
}

impl<'a> Bootstraps<'a> {
    fn read(class: &ClassFile<'a>) -> Self {
        let mut output = Self::default();
        match class.bootstrap_methods() {
            Ok(Some(methods)) => {
                for method in methods {
                    match method {
                        Ok(method) => {
                            if let Err(error) = output.rows.try_reserve(1) {
                                output.issue = Some(InputError::Allocation(error).into());
                                break;
                            }
                            output.rows.push(method);
                        }
                        Err(error) => {
                            output.issue = Some(class_error(error));
                            break;
                        }
                    }
                }
            }
            Ok(None) => {}
            Err(error) => output.issue = Some(class_error(error)),
        }
        output
    }

    fn scan(
        &mut self,
        class: &ClassFile<'a>,
        initial: u16,
        site: Site<'_>,
        range: Option<[usize; 2]>,
        plan: &Level<'_>,
        emit: &mut dyn FnMut(Event) -> Result<(), QueryError>,
    ) -> Result<(), QueryError> {
        self.pending.clear();
        self.seen.clear();
        let mut next = Some(initial);
        while let Some(index) = next.take().or_else(|| self.pending.pop()) {
            let Some(row) = self.rows.get(usize::from(index)).copied() else {
                if let Some(error) = &self.issue {
                    emit(Event::Issue(Issue::new(site.context(), error, range)))?;
                } else {
                    report(
                        emit,
                        || site.context(),
                        local(
                            FailureKind::Malformed,
                            format_args!("invalid JVM bootstrap index {index}"),
                        ),
                        range,
                    )?;
                }
                continue;
            };
            match bootstrap_method_reference(class, row.method_ref) {
                Ok(reference) => reference_hit(reference, "bootstrap-method", site, plan, emit)?,
                Err(error) => report(emit, || site.context(), error, range)?,
            }
            for index in row.arguments() {
                match bootstrap_argument_reference(class, index) {
                    Ok(Some(Reference::Dynamic {
                        bootstrap_index,
                        descriptor,
                        ..
                    })) => {
                        if let Search::Initial(Target::Type(filter)) = plan.search
                            && plan.typed_uses
                        {
                            signature_hits(filter, descriptor, true, "dynamic-type", site, emit)?;
                        }
                        if bootstrap_index != initial
                            && let Err(error) = self.schedule(bootstrap_index)
                        {
                            report(emit, || site.context(), error, range)?;
                        }
                    }
                    Ok(Some(reference)) => {
                        reference_hit(reference, "bootstrap-argument", site, plan, emit)?;
                    }
                    Ok(None) => {}
                    Err(error) => report(emit, || site.context(), reference_error(error), range)?,
                }
            }
        }
        Ok(())
    }

    // Each bootstrap row is reached at most once per original instruction.
    // The common case without nested dynamic constants needs no traversal buffers.
    fn schedule(&mut self, index: u16) -> Result<(), QueryError> {
        if !self.seen.contains(&index) {
            self.pending
                .try_reserve(1)
                .map_err(InputError::Allocation)?;
            self.seen.try_reserve(1).map_err(InputError::Allocation)?;
            self.seen.insert(index);
            self.pending.push(index);
        }
        Ok(())
    }
}

fn bootstrap_method_reference<'a>(
    class: &ClassFile<'a>,
    index: u16,
) -> Result<Reference<'a>, QueryError> {
    let Constant::MethodHandle {
        kind,
        reference_index,
    } = class.constant(index).map_err(class_error)?
    else {
        return Err(local(
            FailureKind::Malformed,
            format_args!("invalid JVM bootstrap method handle #{index}"),
        ));
    };
    Ok(Reference::MethodHandle {
        kind,
        reference_index,
        member: method_handle(class, kind, reference_index).map_err(reference_error)?,
    })
}

fn reference_error(error: ReferenceError) -> QueryError {
    local(error.kind(), error)
}

fn reference_hit(
    reference: Reference<'_>,
    use_: &'static str,
    site: Site<'_>,
    plan: &Level<'_>,
    emit: &mut dyn FnMut(Event) -> Result<(), QueryError>,
) -> Result<(), QueryError> {
    match (plan.search, reference) {
        (
            Search::Initial(target @ (Target::String(_) | Target::JvmResource(_))),
            Reference::String(text),
        ) => {
            if let Some(hit) = match_literal(target, text)? {
                let role = if matches!(target, Target::JvmResource(_)) {
                    "resource-path"
                } else {
                    use_
                };
                site.hit(display(text)?, role, hit.relevance, emit)?;
            }
        }
        (Search::Initial(Target::Type(filter)), Reference::Class(name)) => {
            type_hit(filter, name, use_, site, emit)?;
        }
        (Search::Initial(Target::Type(filter)), Reference::MethodType(descriptor))
            if plan.typed_uses =>
        {
            signature_hits(filter, descriptor, true, use_, site, emit)?;
        }
        (
            Search::Initial(Target::Type(filter)),
            Reference::Member(member) | Reference::MethodHandle { member, .. },
        ) if plan.typed_uses => {
            type_hit(filter, member.owner, use_, site, emit)?;
            signature_hits(filter, member.descriptor, true, use_, site, emit)?;
        }
        (target, Reference::Member(member) | Reference::MethodHandle { member, .. }) => {
            if let Some(hit) = match_search(target, member)? {
                let owner = type_text(member.owner)?;
                let name = display(member.name)?;
                let descriptor = display(member.descriptor)?;
                let separator = if member.kind == ReferenceKind::Field {
                    ":"
                } else {
                    ""
                };
                site.hit(
                    format!("{owner}->{name}{separator}{descriptor}"),
                    use_,
                    hit.relevance,
                    emit,
                )?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn match_literal(target: &Target, text: JvmString<'_>) -> Result<Option<FilterMatch>, QueryError> {
    match target {
        Target::String(filter) => filter.evaluate_jvm(text),
        Target::JvmResource(paths) => Ok(paths.matches(text)),
        _ => Ok(None),
    }
}

fn match_search(
    search: Search<'_>,
    member: MemberReference<'_>,
) -> Result<Option<FilterMatch>, QueryError> {
    match search {
        Search::Initial(target) => match_member(target, member),
        Search::Caller(frontier) if member.kind != ReferenceKind::Field => Ok(frontier
            .matches(member)
            .map(|relevance| FilterMatch { relevance })),
        Search::Caller(_) => Ok(None),
    }
}

fn match_member(
    target: &Target,
    member: MemberReference<'_>,
) -> Result<Option<FilterMatch>, QueryError> {
    let (owner, name, descriptor, upstream) = match target {
        Target::Method {
            owner,
            name,
            prototype,
            relevance,
        } if member.kind != ReferenceKind::Field => (owner, name, prototype, *relevance),
        Target::Field {
            owner,
            name,
            field_type,
        } if member.kind == ReferenceKind::Field => (owner, name, field_type, None),
        _ => return Ok(None),
    };
    if let Some(descriptor) = descriptor
        && !member.descriptor.units().eq(descriptor.iter().copied())
    {
        return Ok(None);
    }
    let Some(name_hit) = name.evaluate_jvm(member.name)? else {
        return Ok(None);
    };
    let owner_hit = match owner {
        Some(owner) => match type_match(owner, member.owner)? {
            Some(hit) => hit,
            None => return Ok(None),
        },
        None => FilterMatch::default(),
    };
    Ok(Some(name_hit.combine(owner_hit)?.combine(FilterMatch {
        relevance: upstream,
    })?))
}

fn type_units<'a>(name: JvmString<'a>) -> impl Iterator<Item = u16> + 'a {
    let is_binary = name.bytes().first() != Some(&b'[');
    is_binary
        .then_some(u16::from(b'L'))
        .into_iter()
        .chain(name.units())
        .chain(is_binary.then_some(u16::from(b';')))
}

pub(crate) fn type_descriptor(name: JvmString<'_>) -> Result<Vec<u16>, QueryError> {
    let mut units = Vec::new();
    units
        .try_reserve_exact(name.bytes().len() + 2)
        .map_err(InputError::Allocation)?;
    units.extend(type_units(name));
    Ok(units)
}

pub(crate) fn type_text(name: JvmString<'_>) -> Result<String, QueryError> {
    Ok(at::escape_units(&type_descriptor(name)?))
}

fn type_match(filter: &TextFilter, name: JvmString<'_>) -> Result<Option<FilterMatch>, QueryError> {
    if let TextFilter::Exact(expected) = filter {
        return Ok(type_units(name)
            .eq(expected.iter().copied())
            .then_some(FilterMatch::default()));
    }
    filter.evaluate_units(&type_descriptor(name)?)
}

fn type_hit(
    filter: &TextFilter,
    name: JvmString<'_>,
    use_: &'static str,
    site: Site<'_>,
    emit: &mut dyn FnMut(Event) -> Result<(), QueryError>,
) -> Result<(), QueryError> {
    if let Some(hit) = type_match(filter, name)? {
        site.hit(type_text(name)?, use_, hit.relevance, emit)?;
    }
    Ok(())
}

fn signature_hits(
    filter: &TextFilter,
    descriptor: JvmString<'_>,
    include_parameters: bool,
    use_: &'static str,
    site: Site<'_>,
    emit: &mut dyn FnMut(Event) -> Result<(), QueryError>,
) -> Result<(), QueryError> {
    let units = descriptor.to_utf16().map_err(class_error)?;
    let mut remaining = units.as_slice();
    let mut parameters = remaining.first() == Some(&u16::from(b'('));
    if parameters {
        remaining = &remaining[1..];
    }
    while let Some(&first) = remaining.first() {
        if parameters && first == u16::from(b')') {
            remaining = &remaining[1..];
            parameters = false;
            continue;
        }
        let Some(length) = descriptor_size(remaining) else {
            break;
        };
        let ty = &remaining[..length];
        if (!parameters || include_parameters)
            && let Some(hit) = filter.evaluate_units(ty)?
        {
            site.hit(at::escape_units(ty), use_, hit.relevance, emit)?;
        }
        remaining = &remaining[length..];
    }
    Ok(())
}

pub(crate) fn descriptor_size(units: &[u16]) -> Option<usize> {
    let shape = units
        .iter()
        .take_while(|&&unit| unit == u16::from(b'['))
        .count();
    match units.get(shape)? {
        &value if value == u16::from(b'L') => {
            let tail = units.get(shape + 1..)?;
            let end = tail.iter().position(|&unit| unit == u16::from(b';'))?;
            Some(shape + 2 + end)
        }
        _ => Some(shape + 1),
    }
}

pub(crate) fn display(text: JvmString<'_>) -> Result<String, QueryError> {
    if let Some(text) = text.utf8()
        && !text
            .bytes()
            .any(|byte| matches!(byte, b'\\' | b'\t' | b'\r' | b'\n' | 0))
    {
        return Ok(text.to_owned());
    }
    Ok(at::escape_units(&text.to_utf16().map_err(class_error)?))
}

pub(crate) fn method_at(selector: &str) -> Result<(Location, u16), super::selector::SelectorError> {
    let (location, ordinal) =
        selector
            .rsplit_once(":m")
            .ok_or(super::selector::SelectorError::Syntax(
                "JVM method locator requires j<entry>:m<method>",
            ))?;
    if ordinal.is_empty() || !ordinal.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(super::selector::SelectorError::InvalidIndex);
    }
    let location = location
        .parse()
        .map_err(|_| super::selector::SelectorError::Syntax("invalid JVM entry locator"))?;
    let ordinal = ordinal
        .parse()
        .map_err(|_| super::selector::SelectorError::InvalidIndex)?;
    Ok((location, ordinal))
}

/// Resolve a JVM query subject by its original class, member, or string locus.
pub(crate) fn target_at(
    roots: &JvmRoots<'_>,
    kind: &str,
    selector: &str,
) -> Result<Target, QueryError> {
    let (location, ordinal) = if let Some((location, member)) = selector.rsplit_once(':') {
        let prefix = match kind {
            "method" => 'm',
            "field" => 'f',
            "string" => 's',
            _ => return Err(local(FailureKind::Unsupported, "wrong JVM locator kind")),
        };
        let ordinal = member
            .strip_prefix(prefix)
            .filter(|number| !number.is_empty() && number.bytes().all(|byte| byte.is_ascii_digit()))
            .and_then(|number| number.parse::<u16>().ok())
            .ok_or_else(|| {
                local(
                    FailureKind::Unsupported,
                    "invalid JVM member or constant index",
                )
            })?;
        (location, Some(ordinal))
    } else {
        (selector, None)
    };
    let location: Location = location
        .parse()
        .map_err(|error| local(FailureKind::Unsupported, error))?;
    let root = roots.by_location(&location).ok_or_else(|| {
        local(
            FailureKind::Unsupported,
            format_args!("no JVM definition at {location}"),
        )
    })?;
    let mut reader = roots.reader();
    let class = reader.read(root)?;
    match (kind, ordinal) {
        ("string", Some(index)) => {
            let string = class.string_constant(index).map_err(class_error)?;
            Ok(Target::String(TextFilter::Exact(
                string.to_utf16().map_err(class_error)?,
            )))
        }
        (kind @ ("method" | "field"), Some(ordinal)) => {
            let methods = kind == "method";
            let members = if methods {
                class.methods()
            } else {
                class.fields()
            }
            .map_err(class_error)?;
            let mut selected = None;
            for raw in members {
                let member = raw.map_err(class_error)?;
                if member.ordinal() == ordinal {
                    selected = Some(member);
                    break;
                }
            }
            let member = selected.ok_or_else(|| {
                local(
                    FailureKind::Unsupported,
                    "JVM member ordinal is out of range",
                )
            })?;
            let owner = Some(TextFilter::Exact(type_descriptor(
                class.name().map_err(class_error)?,
            )?));
            let name = TextFilter::Exact(
                member
                    .name(&class)
                    .and_then(JvmString::to_utf16)
                    .map_err(class_error)?,
            );
            let descriptor = Some(
                member
                    .descriptor(&class)
                    .and_then(JvmString::to_utf16)
                    .map_err(class_error)?,
            );
            if methods {
                Ok(Target::Method {
                    owner,
                    name,
                    prototype: descriptor,
                    relevance: None,
                })
            } else {
                Ok(Target::Field {
                    owner,
                    name,
                    field_type: descriptor,
                })
            }
        }
        (kind @ ("type" | "extend" | "impl"), None) => {
            let target = TextFilter::Exact(type_descriptor(class.name().map_err(class_error)?)?);
            Ok(match kind {
                "type" => Target::Type(target),
                "extend" => Target::Hierarchy {
                    relation: Relation::Extend,
                    target,
                },
                _ => Target::Hierarchy {
                    relation: Relation::Impl,
                    target,
                },
            })
        }
        _ => Err(local(FailureKind::Unsupported, "wrong JVM locator kind")),
    }
}
