use diana_decomp::input::{ClassView, ProgramIndex, Prototype, ReferenceFacts};
use diana_decomp::raw::LayoutKind;
use diana_decomp::{ClassDefId, MethodId};

use super::apk::DexRoot;
use super::dex::{self, ImageIndex};
pub use super::filter::TextFilter as TextMatch;
use super::filter::{self, FilterMatch, Relevance};
use super::images::LogicalDex;
use super::input::InputError;
use super::query::{ImageInfo, QueryError};
use super::request::RequestScope;

pub mod code;
mod coordinate;
mod declarations;
mod delivery;
mod prefilter;
mod relations;
pub use coordinate::resolve_coordinate;
pub use delivery::{Event, Issue};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Relation {
    /// The original superclass reference.
    Extend,
    /// A reference in the original direct-interface list, including subinterfaces.
    Impl,
}

/// A reference predicate. Non-exact searches may omit owner/signature
/// constraints; a supplied prototype or field type remains exact.
#[derive(Clone, Debug)]
pub enum Target {
    String(TextMatch),
    Type(TextMatch),
    Resource(crate::analysis::resources::ResourceTarget),
    Method {
        owner: Option<TextMatch>,
        name: TextMatch,
        prototype: Option<Vec<u16>>,
        /// Presentation relevance carried by exact upstream caller expansion.
        relevance: Option<Relevance>,
    },
    Field {
        owner: Option<TextMatch>,
        name: TextMatch,
        field_type: Option<Vec<u16>>,
    },
    Hierarchy {
        relation: Relation,
        target: TextMatch,
    },
}

impl Target {
    pub fn is_ranked(&self) -> bool {
        match self {
            Self::Resource(target) => target
                .entries
                .values()
                .any(|(_, selected)| selected.relevance.is_some()),
            Self::String(predicate)
            | Self::Type(predicate)
            | Self::Hierarchy {
                target: predicate, ..
            } => predicate.is_fuzzy(),
            Self::Method {
                owner,
                name,
                relevance,
                ..
            } => {
                relevance.is_some()
                    || name.is_fuzzy()
                    || owner.as_ref().is_some_and(TextMatch::is_fuzzy)
            }
            Self::Field { owner, name, .. } => {
                name.is_fuzzy() || owner.as_ref().is_some_and(TextMatch::is_fuzzy)
            }
        }
    }
}

/// The original referenced value actually found at one site, never the pattern.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Matched {
    Resource {
        id: crate::analysis::resources::ResourceId,
        name: String,
    },
    String(Vec<u16>),
    Type(Vec<u16>),
    Method {
        owner: Vec<u16>,
        name: Vec<u16>,
        prototype: Vec<u16>,
    },
    Field {
        owner: Vec<u16>,
        name: Vec<u16>,
        field_type: Vec<u16>,
    },
}

/// Declaration relations have a class locator, not an invented method or offset.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Site {
    Class,
    UnknownField(diana_decomp::FieldId),
    UnknownMethod(MethodId),
    Field {
        field: diana_decomp::FieldId,
        name: Vec<u16>,
        field_type: Vec<u16>,
    },
    Method {
        method: MethodId,
        name: Vec<u16>,
        prototype: Vec<u16>,
    },
    Code {
        method: MethodId,
        name: Vec<u16>,
        prototype: Vec<u16>,
        code_unit: u32,
    },
}

impl Site {
    fn try_copy(&self) -> Result<Self, QueryError> {
        Ok(match self {
            Self::Class => Self::Class,
            Self::UnknownField(id) => Self::UnknownField(*id),
            Self::UnknownMethod(id) => Self::UnknownMethod(*id),
            Self::Field {
                field,
                name,
                field_type,
            } => Self::Field {
                field: *field,
                name: copy(name)?,
                field_type: copy(field_type)?,
            },
            Self::Method {
                method,
                name,
                prototype,
            } => Self::Method {
                method: *method,
                name: copy(name)?,
                prototype: copy(prototype)?,
            },
            Self::Code {
                method,
                name,
                prototype,
                code_unit,
            } => Self::Code {
                method: *method,
                name: copy(name)?,
                prototype: copy(prototype)?,
                code_unit: *code_unit,
            },
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct XrefRow {
    pub relevance: Option<Relevance>,
    pub image: ImageIndex,
    pub logical: LogicalDex,
    pub class_def: ClassDefId,
    pub class_descriptor: Vec<u16>,
    pub site: Site,
    /// One for the queried target, two for a caller of a caller.
    pub depth: u32,
    /// Original mnemonic, or the direct declaration relation.
    pub use_: String,
    pub matched: Matched,
}

impl XrefRow {
    pub fn view(&self) -> RowView<'_> {
        RowView {
            image: self.image,
            logical: self.logical,
            class_def: self.class_def,
            class_descriptor: &self.class_descriptor,
            site: &self.site,
            code_unit: match self.site {
                Site::Code { code_unit, .. } => Some(code_unit),
                _ => None,
            },
            depth: self.depth,
            relevance: self.relevance,
            use_: &self.use_,
            matched: &self.matched,
        }
    }
}

/// A borrowed presentation of either a declaration or an integer code hit.
#[derive(Clone, Copy)]
pub struct RowView<'a> {
    pub image: ImageIndex,
    pub logical: LogicalDex,
    pub class_def: ClassDefId,
    pub class_descriptor: &'a [u16],
    pub site: &'a Site,
    pub code_unit: Option<u32>,
    pub depth: u32,
    pub relevance: Option<Relevance>,
    pub use_: &'a str,
    pub matched: &'a Matched,
}

impl<'a> RowView<'a> {
    /// Original caller key for deduplicating the next scan's targets, not rows.
    pub fn caller_key(&self) -> Option<(&'a [u16], &'a [u16], &'a [u16])> {
        match self.site {
            Site::Code {
                name, prototype, ..
            }
            | Site::Method {
                name, prototype, ..
            } if self.code_unit.is_some() => Some((self.class_descriptor, name, prototype)),
            _ => None,
        }
    }

    /// Caller expansion always uses the complete original identity.
    pub fn caller(&self) -> Result<Option<Target>, QueryError> {
        let Some((owner, name, prototype)) = self.caller_key() else {
            return Ok(None);
        };
        Ok(Some(Target::Method {
            owner: Some(TextMatch::Exact(copy(owner)?)),
            name: TextMatch::Exact(copy(name)?),
            prototype: Some(copy(prototype)?),
            relevance: self.relevance,
        }))
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum MethodRelation {
    #[default]
    References,
    Implementations,
    Overrides,
}

#[derive(Clone, Debug)]
pub struct XrefPlan {
    pub recursive: bool,
    pub method_relation: MethodRelation,
    /// One initial query, or all exact callers reached at the previous depth.
    pub targets: Vec<Target>,
    /// Exact class descriptor restricting the referencing definition.
    pub class: Option<Vec<u16>>,
    pub depth: u32,
    pub jobs: usize,
    /// Expand type matches through the signatures of referenced members. Off by
    /// default: a call site references its callee, not the types in its
    /// signature, and expanding them costs one row per call site.
    pub typed_uses: bool,
    /// Keep inflated payloads for a later pass (transitive caller expansion).
    pub retain_payloads: bool,
}

/// Prepared reference scan over the root images of one leased input.
pub struct Xref<'a> {
    roots: Vec<DexRoot<'a>>,
    images: Vec<ImageInfo>,
    plan: XrefPlan,
}

impl<'a> Xref<'a> {
    #[cfg_attr(feature = "perf", fastrace::trace(name = "xref.prepare"))]
    pub fn new(scope: &'a RequestScope, plan: XrefPlan) -> Result<Self, QueryError> {
        if plan.jobs == 0 {
            return Err(QueryError::InvalidJobs);
        }
        let roots = scope.roots()?;
        let mut images = Vec::new();
        images
            .try_reserve_exact(roots.len())
            .map_err(InputError::Allocation)?;
        for (ordinal, root) in roots.iter().enumerate() {
            images.push(ImageInfo {
                index: ImageIndex(ordinal),
                label: root.label().to_owned(),
                entry: root.entry().map(|entry| entry.index()),
            });
        }
        Ok(Self {
            roots,
            images,
            plan,
        })
    }

    pub fn images(&self) -> &[ImageInfo] {
        &self.images
    }

    /// Streams every matching site in completion order. Instruction sites are
    /// never merged; hierarchy queries read declarations without opening bodies.
    /// Reader, allocation and sink failures retain their classified errors.
    pub fn run(
        &self,
        sink: &mut impl FnMut(Event) -> Result<(), QueryError>,
    ) -> Result<(), QueryError> {
        if self.plan.recursive || self.plan.method_relation != MethodRelation::References {
            return relations::run(self, sink);
        }
        #[cfg(feature = "perf")]
        let scan_span = fastrace::Span::enter_with_local_parent("xref.scan");
        #[cfg(feature = "perf")]
        let _scan_parent = scan_span.set_local_parent();
        super::runner::run(
            self.roots.len(),
            self.plan.jobs,
            Vec::new,
            |scratch: &mut Vec<u8>, ordinal, emit| {
                #[cfg(feature = "perf")]
                let image_span = fastrace::Span::enter_with_parent("xref.image", &scan_span);
                #[cfg(feature = "perf")]
                let _image_parent = image_span.set_local_parent();
                let root = &self.roots[ordinal];
                let stopped = std::cell::Cell::new(false);
                let mut forward = |event| {
                    let result = emit(event);
                    if result.is_err() {
                        stopped.set(true);
                    }
                    result
                };
                // One pass reads every root once, so the payload lives in this
                // worker's scratch instead of every root retaining its own.
                let bytes = {
                    #[cfg(feature = "perf")]
                    let _span = fastrace::local::LocalSpan::enter_with_local_parent("xref.inflate");
                    if self.plan.retain_payloads {
                        root.bytes()
                    } else {
                        root.bytes_into(scratch)
                    }
                };
                let result = bytes.map_err(QueryError::from).and_then(|bytes| {
                    dex::visit_logical(bytes, |logical, index| {
                        scan_logical(
                            &index,
                            ImageIndex(ordinal),
                            logical,
                            &self.plan,
                            &mut forward,
                        )
                    })
                });
                match result {
                    Err(error) if !stopped.get() => forward(Event::Issue(Issue::new(
                        format!("image:{ordinal}"),
                        &error,
                        None,
                    ))),
                    result => result,
                }
            },
            sink,
        )
    }
}

#[cfg_attr(feature = "perf", fastrace::trace(name = "xref.logical-scan"))]
fn scan_logical(
    index: &ProgramIndex<'_>,
    image: ImageIndex,
    logical: LogicalDex,
    plan: &XrefPlan,
    emit: &mut dyn FnMut(Event) -> Result<(), QueryError>,
) -> Result<(), QueryError> {
    let stopped = std::cell::Cell::new(false);
    let mut forward = |event| {
        let result = emit(event);
        if result.is_err() {
            stopped.set(true);
        }
        result
    };
    let prefilter = prefilter::Prefilter::new(index, &plan.targets, plan.typed_uses)?;
    let mut failure = None;
    let result = index.visit_classes(|class_def, class| {
        let context = format!("image:{}:l{}:c{}", image.0, logical.0, class_def.0);
        let result = class.map_err(QueryError::from).and_then(|class| {
            scan_class(
                index,
                &class,
                image,
                logical,
                class_def,
                plan,
                &prefilter,
                &mut forward,
            )
        });
        if let Err(error) = result {
            if !stopped.get() {
                if let Err(error) = forward(Event::Issue(Issue::new(context, &error, None))) {
                    failure = Some(error);
                }
            } else {
                failure = Some(error);
            }
        }
        if failure.is_some() {
            return Err(diana_decomp::input::InputError::InternalInvariant(
                "xref sink stopped",
            ));
        }
        Ok(())
    });
    if let Some(error) = failure {
        return Err(error);
    }
    if let Err(error) = result {
        forward(Event::Issue(Issue::new(
            format!("image:{}:l{}", image.0, logical.0),
            &error.into(),
            None,
        )))?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn scan_class<'a>(
    index: &ProgramIndex<'a>,
    class: &ClassView<'_, 'a>,
    image: ImageIndex,
    logical: LogicalDex,
    class_def: ClassDefId,
    plan: &XrefPlan,
    prefilter: &prefilter::Prefilter,
    emit: &mut dyn FnMut(Event) -> Result<(), QueryError>,
) -> Result<(), QueryError> {
    if let Some(target) = plan.class.as_deref()
        && !class.descriptor.matches(target)?
    {
        return Ok(());
    }
    // Hierarchy questions read class_def only: no class data, no member
    // declarations and no code.
    if plan
        .targets
        .iter()
        .all(|target| matches!(target, Target::Hierarchy { .. }))
    {
        let location = declarations::Location {
            image,
            logical,
            class_def,
            descriptor: copy(&class.descriptor.units()?)?,
        };
        let hierarchy = match class.hierarchy() {
            Ok(hierarchy) => hierarchy,
            Err(error) => {
                emit(Event::Issue(Issue::new(
                    location.context(),
                    &QueryError::from(error),
                    None,
                )))?;
                return Ok(());
            }
        };
        return emit_hierarchy(
            class.access_flags,
            hierarchy.superclass.as_ref(),
            &hierarchy.interfaces,
            &location,
            plan,
            emit,
        );
    }
    // A pure string query reads code plus the two declaration records that can
    // carry a string: static initial values and annotations.
    let string_only = plan
        .targets
        .iter()
        .all(|target| matches!(target, Target::String(_)));
    if string_only {
        let location = declarations::Location {
            image,
            logical,
            class_def,
            descriptor: copy(&class.descriptor.units()?)?,
        };
        if !prefilter.impossible() {
            scan_code(index, class, &location, plan, prefilter, emit)?;
        }
        if let Err(error) = declarations::scan_declared_values(
            index,
            class,
            &location,
            &plan.targets,
            plan.depth,
            true,
            emit,
        ) {
            emit(Event::Issue(Issue::new(
                format!("{} declaration metadata", location.context()),
                &error,
                None,
            )))?;
        }
        return Ok(());
    }
    // Method and field references live in code. Annotations and static
    // initial values can also name a member through an encoded reference, so
    // read those values directly; a full class snapshot would parse every
    // declaration in the class for a handful of references.
    let member_only = plan
        .targets
        .iter()
        .all(|target| matches!(target, Target::Method { .. } | Target::Field { .. }));
    if member_only {
        let location = declarations::Location {
            image,
            logical,
            class_def,
            descriptor: copy(&class.descriptor.units()?)?,
        };
        {
            let stopped = std::cell::Cell::new(false);
            let mut forward = |event| {
                let result = emit(event);
                if result.is_err() {
                    stopped.set(true);
                }
                result
            };
            if let Err(error) = declarations::scan_declared_values(
                index,
                class,
                &location,
                &plan.targets,
                plan.depth,
                true,
                &mut forward,
            ) {
                if stopped.get() {
                    return Err(error);
                }
                forward(Event::Issue(Issue::new(
                    format!("{} declaration metadata", location.context()),
                    &error,
                    None,
                )))?;
            }
        }
        return scan_code(index, class, &location, plan, prefilter, emit);
    }
    // A pure type query needs signatures, not a class-data snapshot.
    if !plan.typed_uses
        && plan
            .targets
            .iter()
            .all(|target| matches!(target, Target::Type(_)))
    {
        let location = declarations::Location {
            image,
            logical,
            class_def,
            descriptor: copy(&class.descriptor.units()?)?,
        };
        declarations::scan_type_declarations(
            index,
            class,
            &location,
            &plan.targets,
            plan.depth,
            emit,
        )?;
        return scan_code(index, class, &location, plan, prefilter, emit);
    }
    let facts = declarations::collect(index, class_def)?;
    let class_descriptor = copy(facts.descriptor().units())?;
    let location = declarations::Location {
        image,
        logical,
        class_def,
        descriptor: copy(&class_descriptor)?,
    };
    if plan
        .targets
        .iter()
        .any(|target| matches!(target, Target::Hierarchy { .. }))
    {
        use diana_decomp::p0::Fact;
        for target in &plan.targets {
            let Target::Hierarchy { relation, target } = target else {
                continue;
            };
            let mut references = Vec::new();
            match relation {
                Relation::Extend => match facts.superclass() {
                    Fact::Present(value) => {
                        references.try_reserve(1).map_err(InputError::Allocation)?;
                        references.push(value.units());
                    }
                    Fact::Unavailable(error) => {
                        emit(Event::Issue(Issue::from_request(location.context(), error)))?
                    }
                    Fact::NotRead => emit(Event::Issue(Issue::unknown(
                        location.context(),
                        "superclass unavailable",
                    )))?,
                    Fact::Absent => (),
                },
                Relation::Impl => match facts.interfaces() {
                    Fact::Present(values) => {
                        for value in values {
                            match &value.descriptor {
                                Fact::Present(value) => {
                                    references.try_reserve(1).map_err(InputError::Allocation)?;
                                    references.push(value.units());
                                }
                                Fact::Unavailable(error) => emit(Event::Issue(
                                    Issue::from_request(location.context(), error),
                                ))?,
                                _ => emit(Event::Issue(Issue::unknown(
                                    location.context(),
                                    "interface descriptor unavailable",
                                )))?,
                            }
                        }
                    }
                    Fact::Unavailable(error) => {
                        emit(Event::Issue(Issue::from_request(location.context(), error)))?
                    }
                    Fact::NotRead => emit(Event::Issue(Issue::unknown(
                        location.context(),
                        "interface list unavailable",
                    )))?,
                    Fact::Absent => (),
                },
            }
            for reference in references {
                if let Some(selected) = target.evaluate_units(reference)? {
                    let label = match relation {
                        Relation::Extend => "extends",
                        Relation::Impl => {
                            if class.access_flags & 0x200 != 0 {
                                "extends-interface"
                            } else {
                                "implements"
                            }
                        }
                    };
                    emit(location.row(
                        Site::Class,
                        Matched::Type(copy(reference)?),
                        selected,
                        label.to_owned(),
                        plan.depth,
                    )?)?;
                }
            }
        }
    }
    if plan
        .targets
        .iter()
        .all(|target| matches!(target, Target::Hierarchy { .. }))
    {
        return Ok(());
    }
    let stopped = std::cell::Cell::new(false);
    let mut forward = |event| {
        let result = emit(event);
        if result.is_err() {
            stopped.set(true);
        }
        result
    };
    if let Err(error) =
        declarations::scan(&facts, &location, &plan.targets, plan.depth, &mut forward)
    {
        if stopped.get() {
            return Err(error);
        }
        forward(Event::Issue(Issue::new(
            format!("{} declaration metadata", location.context()),
            &error,
            None,
        )))?;
    }
    // Bodies are scanned from the class-data method list, the same walk the
    // declaration-only paths use; only resource literals still need the
    // fact-side method loop.
    if !plan
        .targets
        .iter()
        .any(|target| matches!(target, Target::Resource(_)))
    {
        return scan_code(index, class, &location, plan, prefilter, emit);
    }
    let mut failure = None;
    for method in facts.methods() {
        let scope = format!(
            "image:{}:l{}:c{}:m{}",
            image.0,
            logical.0,
            class_def.0,
            method.method().0
        );
        let context = match method.declaration() {
            diana_decomp::p0::Fact::Present(declaration) => format!(
                "{scope}::{}->{}{}",
                crate::analysis::at::escape_units(&class_descriptor),
                crate::analysis::at::escape_units(declaration.name_units()),
                crate::analysis::at::escape_units(declaration.prototype_units())
            ),
            _ => scope,
        };
        let result = (|| -> Result<(), QueryError> {
            let source = index.method_source(&facts, method.method())?;
            let declaration = source.declaration();
            let view = match source.raw() {
                diana_decomp::input::RawPrintable::Method(view) => view,
                diana_decomp::input::RawPrintable::Declaration => return Ok(()),
            };
            let range = view.instruction_byte_range();
            // A body without any target operand byte cannot reference the
            // target; skip its decode entirely.
            if let Some(code) = view.source_bytes().get(range.start.0..range.end.0)
                && !prefilter.might_reference(code)
            {
                return Ok(());
            }
            let mut sites = Vec::new();
            let mut recovered_end = range.start.0;
            let mut literals = Vec::new();
            let resources = plan
                .targets
                .iter()
                .any(|target| matches!(target, Target::Resource(_)));
            let decoded = view.visit_instructions(|item| {
                let instruction = match item {
                    diana_decomp::raw::RawCodeItem::Instruction(instruction) => instruction,
                    diana_decomp::raw::RawCodeItem::Payload(payload) => {
                        if resources && let Some(elements) = payload.array_elements() {
                            for (ordinal, element) in elements.enumerate() {
                                if let Ok(value) = u32::try_from(element.bits) {
                                    literals
                                        .try_reserve(1)
                                        .map_err(diana_decomp::input::MethodError::Allocation)?;
                                    literals.push((
                                        payload.pc(),
                                        format!("array-payload[{ordinal}]"),
                                        value,
                                    ));
                                }
                            }
                        }
                        recovered_end = recovered_end
                            .checked_add(payload.words().len())
                            .ok_or(diana_decomp::input::MethodError::PlatformCapacity)?;
                        return Ok(());
                    }
                };
                let code_unit = instruction.pc();
                let layout = instruction.layout();
                let LayoutKind::Instruction(spec) = layout.kind() else {
                    return Ok(());
                };
                if resources {
                    let literal = match instruction.operands().literal() {
                        Some(diana_decomp::raw::RawLiteral::Bits32(value)) => Some(value),
                        Some(diana_decomp::raw::RawLiteral::Bits64(value)) => {
                            u32::try_from(value).ok()
                        }
                        _ => None,
                    };
                    if let Some(value) = literal {
                        literals
                            .try_reserve(1)
                            .map_err(diana_decomp::input::MethodError::Allocation)?;
                        literals.push((code_unit, spec.mnemonic.to_owned(), value));
                    }
                }
                for reference in layout.references().into_iter().flatten() {
                    let needed = if plan.typed_uses {
                        needs_reference(reference.target, &plan.targets)
                    } else {
                        needs_direct(reference.target, &plan.targets)
                    };
                    if !needed {
                        continue;
                    }
                    sites
                        .try_reserve(1)
                        .map_err(diana_decomp::input::MethodError::Allocation)?;
                    sites.push((code_unit, spec.mnemonic, reference.target));
                }
                recovered_end = recovered_end
                    .checked_add(instruction.words().len())
                    .ok_or(diana_decomp::input::MethodError::PlatformCapacity)?;
                Ok(())
            });
            let name = copy(declaration.name_units())?;
            let prototype = copy(declaration.prototype_units())?;
            for (code_unit, mnemonic, reference) in sites {
                // A prototype operand names its types directly; every other
                // derived leaf belongs to typed-use expansion.
                let direct = plan.typed_uses
                    || matches!(reference, diana_decomp::raw::OperandReference::Proto(_));
                index.visit_reference_leaves::<QueryError>(reference, |path, facts| {
                    if !direct && !path.is_empty() {
                        return Ok(());
                    }
                    let found = facts
                        .map_err(QueryError::from)
                        .and_then(|facts| matched(facts, &plan.targets));
                    let produced = (|| -> Result<Option<Event>, QueryError> {
                        match found {
                            Ok(Some((matched, selected))) => Ok(Some(Event::Row(XrefRow {
                                image,
                                logical,
                                class_def,
                                class_descriptor: copy(&class_descriptor)?,
                                relevance: selected.relevance,
                                site: Site::Code {
                                    method: method.method(),
                                    name: copy(&name)?,
                                    prototype: copy(&prototype)?,
                                    code_unit: code_unit.0,
                                },
                                depth: plan.depth,
                                use_: if path.is_empty() {
                                    mnemonic.to_owned()
                                } else {
                                    format!("{mnemonic} → {path}")
                                },
                                matched,
                            }))),
                            Ok(None) => Ok(None),
                            Err(error) => Err(error),
                        }
                    })();
                    match produced {
                        Ok(Some(event)) => forward(event)?,
                        Ok(None) => (),
                        Err(error) => forward(Event::Issue(Issue::new(
                            format!(
                                "{context}@cu:{:#x} {path} reference:{reference:?}",
                                code_unit.0
                            ),
                            &error,
                            None,
                        )))?,
                    }
                    Ok(())
                })?;
            }
            for (code_unit, mnemonic, id) in literals {
                let produced = (|| -> Result<Option<Event>, QueryError> {
                    let Some((matched, selected)) = resource_match(id, &plan.targets)? else {
                        return Ok(None);
                    };
                    Ok(Some(Event::Row(XrefRow {
                        image,
                        logical,
                        class_def,
                        class_descriptor: copy(&class_descriptor)?,
                        relevance: selected.relevance,
                        site: Site::Code {
                            method: method.method(),
                            name: copy(&name)?,
                            prototype: copy(&prototype)?,
                            code_unit: code_unit.0,
                        },
                        depth: plan.depth,
                        use_: format!("resource-id-literal candidate {mnemonic}"),
                        matched,
                    })))
                })();
                match produced {
                    Ok(Some(event)) => forward(event)?,
                    Ok(None) => (),
                    Err(error) => forward(Event::Issue(Issue::new(
                        format!("{context}@cu:{:#x} resource:{id:#010x}", code_unit.0),
                        &error,
                        None,
                    )))?,
                }
            }
            if let Err(error) = decoded {
                forward(Event::Issue(Issue::new(
                    context.clone(),
                    &error.into(),
                    Some([recovered_end, range.end.0]),
                )))?;
            }
            Ok(())
        })();
        if let Err(error) = result {
            if !stopped.get() {
                if let Err(error) = forward(Event::Issue(Issue::new(context, &error, None))) {
                    failure = Some(error);
                }
            } else {
                failure = Some(error);
            }
        }
        if failure.is_some() {
            break;
        }
    }
    if let Some(error) = failure {
        return Err(error);
    }
    Ok(())
}

/// Emits direct hierarchy relations from `class_def` alone.
fn emit_hierarchy(
    access_flags: u32,
    superclass: Option<&diana_decomp::input::DexString<'_>>,
    interfaces: &[diana_decomp::input::DexString<'_>],
    location: &declarations::Location,
    plan: &XrefPlan,
    emit: &mut dyn FnMut(Event) -> Result<(), QueryError>,
) -> Result<(), QueryError> {
    for target in &plan.targets {
        let Target::Hierarchy { relation, target } = target else {
            continue;
        };
        let label = match relation {
            Relation::Extend => "extends",
            Relation::Impl if access_flags & 0x200 != 0 => "extends-interface",
            Relation::Impl => "implements",
        };
        let mut emit_one = |value: &diana_decomp::input::DexString<'_>| -> Result<(), QueryError> {
            let units = value.units()?;
            let Some(selected) = target.evaluate_units(&units)? else {
                return Ok(());
            };
            emit(location.row(
                Site::Class,
                Matched::Type(copy(&units)?),
                selected,
                label.to_owned(),
                plan.depth,
            )?)?;
            Ok(())
        };
        match relation {
            Relation::Extend => {
                if let Some(value) = superclass {
                    emit_one(value)?;
                }
            }
            Relation::Impl => {
                for value in interfaces {
                    emit_one(value)?;
                }
            }
        }
    }
    Ok(())
}

/// Direct references in method bodies, without a class-data snapshot.
///
/// Method identities and code offsets come from one class-data walk. A body is
/// read only when its bytes contain a target operand, and its instructions are
/// walked with a packed width/kind table; encodings that need operand expansion
/// fall back to the checked decoder.
fn scan_code(
    index: &ProgramIndex<'_>,
    class: &ClassView<'_, '_>,
    location: &declarations::Location,
    plan: &XrefPlan,
    prefilter: &prefilter::Prefilter,
    emit: &mut dyn FnMut(Event) -> Result<(), QueryError>,
) -> Result<(), QueryError> {
    if prefilter.impossible() {
        return Ok(());
    }
    let mut failure = None;
    let mut sites = Vec::new();
    let mut batch = code::Builder::new(location, plan.depth)?;
    let visited = class.visit_method_codes(|method_idx, access_flags, code_offset| {
        if failure.is_some() {
            return Ok(());
        }
        sites.clear();
        let mut unavailable = None;
        let result = (|| -> Result<(), QueryError> {
            let Some(code_offset) = code_offset else {
                return Ok(());
            };
            let Some(code) = index.method_instructions(Some(code_offset), access_flags)? else {
                return Ok(());
            };
            if !prefilter.might_reference(code) {
                return Ok(());
            }
            if flat_sites(code, prefilter, &mut sites)? {
                return batch.method(index, method_idx, &sites, plan);
            }
            let diana_decomp::input::MethodBody::Code(view) =
                index.method_body(Some(code_offset), method_idx, access_flags)?
            else {
                return Ok(());
            };
            let range = view.instruction_byte_range();
            // A packed walk that encounters an unsupported encoding publishes
            // nothing. The checked decoder owns the entire instruction stream.
            sites.clear();
            let mut recovered_end = range.start.0;
            let decoded = view.visit_instructions(|item| {
                let words = match &item {
                    diana_decomp::raw::RawCodeItem::Instruction(value) => value.words().len(),
                    diana_decomp::raw::RawCodeItem::Payload(value) => value.words().len(),
                };
                recovered_end = words
                    .checked_mul(2)
                    .and_then(|bytes| recovered_end.checked_add(bytes))
                    .ok_or(diana_decomp::input::MethodError::PlatformCapacity)?;
                let diana_decomp::raw::RawCodeItem::Instruction(instruction) = item else {
                    return Ok(());
                };
                let layout = instruction.layout();
                let LayoutKind::Instruction(spec) = layout.kind() else {
                    return Ok(());
                };
                for reference in layout.references().into_iter().flatten() {
                    let needed = if plan.typed_uses {
                        needs_reference(reference.target, &plan.targets)
                    } else {
                        needs_direct(reference.target, &plan.targets)
                    };
                    if needed {
                        sites
                            .try_reserve(1)
                            .map_err(diana_decomp::input::MethodError::Allocation)?;
                        sites.push(FlatSite {
                            code_unit: instruction.pc().0,
                            mnemonic: spec.mnemonic,
                            reference: reference.target,
                        });
                    }
                }
                Ok(())
            });
            if decoded.is_err() {
                unavailable = Some([recovered_end, range.end.0]);
            }
            batch.method(index, method_idx, &sites, plan)?;
            decoded.map_err(QueryError::from)
        })();
        if let Err(error) = result
            && let Err(error) = emit(Event::Issue(Issue::new(
                format!("{}:m{}", location.context(), method_idx.0),
                &error,
                unavailable,
            )))
        {
            failure = Some(error);
        }
        Ok(())
    });
    let batch = batch.finish();
    if failure.is_none() && !batch.is_empty() {
        emit(Event::Code(batch))?;
    }
    if let Err(error) = visited {
        emit(Event::Issue(Issue::new(
            format!("{} methods", location.context()),
            &QueryError::from(error),
            None,
        )))?;
    }
    match failure {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

struct FlatSite {
    code_unit: u32,
    mnemonic: &'static str,
    reference: diana_decomp::raw::OperandReference,
}

/// Walks one instruction stream with the packed opcode table and appends the
/// operand references whose index the filter accepts. False means the body
/// needs the checked decoder (unsupported layouts, wide operands or unknown bytes).
///
/// The caller clears and reuses the accumulator across method bodies. A false
/// result leaves provisional hits that must be discarded before checked decoding.
fn flat_sites(
    code: &[u8],
    prefilter: &prefilter::Prefilter,
    sites: &mut Vec<FlatSite>,
) -> Result<bool, QueryError> {
    use diana_decomp::raw::OperandReference;
    use diana_decomp::{FieldId, MethodId, StringId, TypeId};
    let table = diana_decomp::input::formats::opcode_scan_table();
    let mut pc = 0usize;
    while pc + 2 <= code.len() {
        let opcode = code[pc];
        let first = u16::from_le_bytes([code[pc], code[pc + 1]]);
        if opcode == 0 {
            if first == 0 {
                pc += 2;
                continue;
            }
            let Some(width) = payload_bytes(code, pc, first) else {
                return Ok(false);
            };
            let Some(end) = pc.checked_add(width).filter(|end| *end <= code.len()) else {
                return Ok(false);
            };
            pc = end;
            continue;
        }
        let info = table[usize::from(opcode)];
        let units = usize::from(info & 0xff);
        let mask = info >> 8;
        // Two references, dynamic handles or unknown encodings need operands
        // the packed table cannot describe.
        if units == 0 || mask.count_ones() > 1 || mask & (32 | 64) != 0 {
            return Ok(false);
        }
        let Some(end) = pc.checked_add(units * 2).filter(|end| *end <= code.len()) else {
            return Ok(false);
        };
        if mask != 0 && prefilter.accepts_kind(mask) {
            let index = if opcode == 0x1b {
                if pc + 6 > code.len() {
                    return Ok(false);
                }
                u32::from_le_bytes([code[pc + 2], code[pc + 3], code[pc + 4], code[pc + 5]])
            } else {
                if pc + 4 > code.len() {
                    return Ok(false);
                }
                u32::from(u16::from_le_bytes([code[pc + 2], code[pc + 3]]))
            };
            if prefilter.contains(index) {
                let reference = match mask {
                    1 => OperandReference::String(StringId(index)),
                    2 => OperandReference::Type(TypeId(index)),
                    4 => OperandReference::Field(FieldId(index)),
                    8 => OperandReference::Method(MethodId(index)),
                    _ => return Ok(false),
                };
                let Some(mnemonic) = diana_decomp::input::formats::opcode_mnemonic(opcode) else {
                    return Ok(false);
                };
                sites.try_reserve(1).map_err(InputError::Allocation)?;
                sites.push(FlatSite {
                    code_unit: match u32::try_from(pc / 2) {
                        Ok(code_unit) => code_unit,
                        Err(_) => return Ok(false),
                    },
                    mnemonic,
                    reference,
                });
            }
        }
        pc = end;
    }
    Ok(pc == code.len())
}

/// Data payloads contain no pool references. Establish their complete byte
/// extent before skipping; unsupported or truncated layouts use the checked walk.
fn payload_bytes(code: &[u8], pc: usize, ident: u16) -> Option<usize> {
    if !pc.is_multiple_of(4) {
        return None;
    }
    let header = code.get(pc..pc.checked_add(4)?)?;
    let size = usize::from(u16::from_le_bytes([header[2], header[3]]));
    match ident {
        0x0100 => size.checked_mul(4)?.checked_add(8),
        0x0200 => size.checked_mul(8)?.checked_add(4),
        0x0300 => {
            if !matches!(size, 1 | 2 | 4 | 8) {
                return None;
            }
            let count = code.get(pc.checked_add(4)?..pc.checked_add(8)?)?;
            let count =
                usize::try_from(u32::from_le_bytes([count[0], count[1], count[2], count[3]]))
                    .ok()?;
            let bytes = size.checked_mul(count)?;
            bytes.checked_add(bytes % 2)?.checked_add(8)
        }
        _ => None,
    }
}

fn copy(units: &[u16]) -> Result<Vec<u16>, QueryError> {
    let mut owned = Vec::new();
    append(&mut owned, units)?;
    Ok(owned)
}

fn append(out: &mut Vec<u16>, units: &[u16]) -> Result<(), QueryError> {
    out.try_reserve(units.len())
        .map_err(InputError::Allocation)?;
    out.extend_from_slice(units);
    Ok(())
}

fn prototype_units(prototype: &Prototype<'_, '_>) -> Result<Vec<u16>, QueryError> {
    let mut units = Vec::new();
    append(&mut units, &[u16::from(b'(')])?;
    for parameter in prototype.parameter_descriptors() {
        append(&mut units, &parameter.units()?)?;
    }
    append(&mut units, &[u16::from(b')')])?;
    append(&mut units, &prototype.return_type.units()?)?;
    Ok(units)
}

/// Whether this operand kind is the target kind itself. Derived type uses
/// (member owner/parameter/return/field types) stay behind `--typed-uses`.
pub(super) fn needs_direct(
    reference: diana_decomp::raw::OperandReference,
    targets: &[Target],
) -> bool {
    use diana_decomp::raw::OperandReference as Ref;
    targets.iter().any(|target| match (reference, target) {
        (_, Target::Resource(_) | Target::Hierarchy { .. }) => false,
        (Ref::CallSite(_), _) => true,
        (Ref::String(_), Target::String(_)) => true,
        (Ref::Type(_) | Ref::Proto(_), Target::Type(_)) => true,
        (Ref::Method(_), Target::Method { .. }) => true,
        (Ref::Field(_), Target::Field { .. }) => true,
        (Ref::MethodHandle(_), Target::Method { .. } | Target::Field { .. }) => true,
        _ => false,
    })
}

pub(super) fn needs_reference(
    reference: diana_decomp::raw::OperandReference,
    targets: &[Target],
) -> bool {
    use diana_decomp::raw::OperandReference as Ref;
    targets.iter().any(|target| match (reference, target) {
        (_, Target::Resource(_) | Target::Hierarchy { .. }) => false,
        (Ref::CallSite(_), _) => true,
        (Ref::String(_), Target::String(_)) => true,
        (Ref::Type(_) | Ref::Proto(_), Target::Type(_)) => true,
        (Ref::Method(_), Target::Method { .. } | Target::Type(_)) => true,
        (Ref::Field(_), Target::Field { .. } | Target::Type(_)) => true,
        (Ref::MethodHandle(_), Target::Method { .. } | Target::Field { .. } | Target::Type(_)) => {
            true
        }
        _ => false,
    })
}

pub(super) fn resource_match(
    id: u32,
    targets: &[Target],
) -> Result<Option<(Matched, FilterMatch)>, QueryError> {
    let id = crate::analysis::resources::ResourceId(id);
    for target in targets {
        if let Target::Resource(target) = target
            && let Some((name, selected)) = target.entries.get(&id)
        {
            let mut copied = String::new();
            copied
                .try_reserve_exact(name.len())
                .map_err(InputError::Allocation)?;
            copied.push_str(name);
            return Ok(Some((Matched::Resource { id, name: copied }, *selected)));
        }
    }
    Ok(None)
}

fn matched(
    facts: ReferenceFacts<'_, '_>,
    targets: &[Target],
) -> Result<Option<(Matched, FilterMatch)>, QueryError> {
    for target in targets {
        let found = match (&facts, target) {
            (ReferenceFacts::String(value), Target::String(predicate)) => {
                let Some(selected) = predicate.evaluate(*value)? else {
                    continue;
                };
                Some((Matched::String(value.units()?), selected))
            }
            (ReferenceFacts::Type(value), Target::Type(predicate)) => {
                let Some(selected) = predicate.evaluate(*value)? else {
                    continue;
                };
                Some((Matched::Type(value.units()?), selected))
            }
            (
                ReferenceFacts::Method {
                    owner,
                    name,
                    prototype,
                },
                Target::Method {
                    owner: owner_match,
                    name: name_match,
                    prototype: signature,
                    relevance,
                },
            ) => {
                let Some(owner_quality) = filter::select(owner_match.as_ref(), *owner)? else {
                    continue;
                };
                let Some(name_quality) = name_match.evaluate(*name)? else {
                    continue;
                };
                if let Some(signature) = signature
                    && !prototype.matches(signature)?
                {
                    continue;
                }
                let mut selected = owner_quality.combine(name_quality)?;
                if let Some(relevance) = relevance {
                    selected.relevance = Some(*relevance);
                }
                Some((
                    Matched::Method {
                        owner: owner.units()?,
                        name: name.units()?,
                        prototype: prototype_units(prototype)?,
                    },
                    selected,
                ))
            }
            (
                ReferenceFacts::Field {
                    owner,
                    name,
                    field_type,
                },
                Target::Field {
                    owner: owner_match,
                    name: name_match,
                    field_type: signature,
                },
            ) => {
                let Some(owner_quality) = filter::select(owner_match.as_ref(), *owner)? else {
                    continue;
                };
                let Some(name_quality) = name_match.evaluate(*name)? else {
                    continue;
                };
                if let Some(signature) = signature
                    && !field_type.matches(signature)?
                {
                    continue;
                }
                Some((
                    Matched::Field {
                        owner: owner.units()?,
                        name: name.units()?,
                        field_type: field_type.units()?,
                    },
                    owner_quality.combine(name_quality)?,
                ))
            }
            _ => None,
        };
        if found.is_some() {
            return Ok(found);
        }
    }
    Ok(None)
}

/// Convenience for an exact `type` target expressed as a descriptor or dotted name.
pub fn type_target(text: &str) -> Target {
    Target::Type(TextMatch::Exact(super::dex::class_descriptor(text)))
}
