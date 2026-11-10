use diana_decomp::input::Cursor;
use diana_decomp::input::annotations::{AnnotationMembers, AnnotationOwner, association};
use diana_decomp::input::values::{EncodedKind, EncodedValue, EncodedValues, ValueStep};
use diana_decomp::input::{
    ProgramIndex,
    value_facts::{ValueDeclaration, ValueMember, ValuePrototype},
};
use diana_decomp::p0::{ClassFacts, Fact, InitialValue};
use diana_decomp::{
    AnnotationLocation, AnnotationToken, AnnotationValueKind, ClassDefId, ClassKey, DexId,
};

use super::{
    Event, FilterMatch, Issue, Matched, QueryError, Site, Target, XrefRow, copy, prototype_units,
};
use crate::analysis::{dex::ImageIndex, images::LogicalDex};

pub(super) trait Descriptor {
    fn to_units(&self) -> Result<Vec<u16>, QueryError>;
}

impl Descriptor for Vec<u16> {
    fn to_units(&self) -> Result<Vec<u16>, QueryError> {
        copy(self)
    }
}

impl Descriptor for diana_decomp::input::DexString<'_> {
    fn to_units(&self) -> Result<Vec<u16>, QueryError> {
        Ok(self.units()?)
    }
}

/// Scans borrow encoded descriptors; rows materialize original units on a hit.
pub(super) struct Location<D = Vec<u16>> {
    pub image: ImageIndex,
    pub logical: LogicalDex,
    pub class_def: ClassDefId,
    pub descriptor: D,
}

pub(super) type InputLocation<'a> = Location<diana_decomp::input::DexString<'a>>;
impl<D: Descriptor> Location<D> {
    pub fn context(&self) -> String {
        format!(
            "image:{}:l{}:c{}",
            self.image.0, self.logical.0, self.class_def.0
        )
    }
    pub fn row(
        &self,
        site: Site,
        matched: Matched,
        selected: FilterMatch,
        use_: String,
        depth: u32,
    ) -> Result<Event, QueryError> {
        Ok(Event::Row(XrefRow {
            image: self.image,
            logical: self.logical,
            class_def: self.class_def,
            class_descriptor: self.descriptor.to_units()?,
            relevance: selected.relevance,
            site,
            depth,
            use_,
            matched,
        }))
    }
}

pub(super) fn collect(
    index: &ProgramIndex<'_>,
    class_def: ClassDefId,
) -> Result<ClassFacts, QueryError> {
    // Standalone ProgramIndex values have request-local DexId(0); the application
    // retains physical/logical namespaces separately in Location.
    Ok(diana_decomp::p0::collect(
        index,
        ClassKey {
            dex: DexId(0),
            class_def,
        },
    )?)
}

pub(super) fn method_site(
    facts: &ClassFacts,
    id: diana_decomp::MethodId,
) -> Result<Site, QueryError> {
    Ok(
        match facts.method_entry(id).map(|entry| entry.declaration()) {
            Some(Fact::Present(declaration)) => Site::Method {
                method: id,
                name: copy(declaration.name_units())?,
                prototype: copy(declaration.prototype_units())?,
            },
            _ => Site::UnknownMethod(id),
        },
    )
}
fn field_site(facts: &ClassFacts, id: diana_decomp::FieldId) -> Result<Site, QueryError> {
    Ok(
        match facts.field_entry(id).map(|entry| entry.declaration()) {
            Some(Fact::Present(declaration)) => Site::Field {
                field: id,
                name: copy(declaration.name.units())?,
                field_type: copy(declaration.descriptor.units())?,
            },
            _ => Site::UnknownField(id),
        },
    )
}

pub(super) fn prototype(value: &ValuePrototype) -> Result<Vec<u16>, QueryError> {
    let mut units = Vec::new();
    super::append(&mut units, &[u16::from(b'(')])?;
    for parameter in &value.parameters {
        super::append(&mut units, &parameter.descriptor)?;
    }
    super::append(&mut units, &[u16::from(b')')])?;
    super::append(&mut units, &value.returns.descriptor)?;
    Ok(units)
}

pub(super) fn matched_value(
    value: Matched,
    targets: &[Target],
) -> Result<Option<(Matched, FilterMatch)>, QueryError> {
    for target in targets {
        let selected = match (&value, target) {
            (Matched::String(value), Target::String(predicate))
            | (Matched::Type(value), Target::Type(predicate)) => predicate.evaluate_units(value)?,
            (
                Matched::Method {
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
                if signature
                    .as_ref()
                    .is_some_and(|signature| signature != prototype)
                {
                    continue;
                }
                let Some(left) = (match owner_match {
                    Some(predicate) => predicate.evaluate_units(owner)?,
                    None => Some(FilterMatch::default()),
                }) else {
                    continue;
                };
                let Some(right) = name_match.evaluate_units(name)? else {
                    continue;
                };
                let mut selected = left.combine(right)?;
                if relevance.is_some() {
                    selected.relevance = *relevance;
                }
                Some(selected)
            }
            (
                Matched::Field {
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
                if signature
                    .as_ref()
                    .is_some_and(|signature| signature != field_type)
                {
                    continue;
                }
                let Some(left) = (match owner_match {
                    Some(predicate) => predicate.evaluate_units(owner)?,
                    None => Some(FilterMatch::default()),
                }) else {
                    continue;
                };
                let Some(right) = name_match.evaluate_units(name)? else {
                    continue;
                };
                Some(left.combine(right)?)
            }
            _ => None,
        };
        if let Some(selected) = selected {
            return Ok(Some((value, selected)));
        }
    }
    Ok(None)
}

struct Scan<'a, 'e> {
    location: &'a InputLocation<'a>,
    facts: &'a ClassFacts,
    targets: &'a [Target],
    emit: &'e mut dyn FnMut(Event) -> Result<(), QueryError>,
    depth: u32,
    stopped: bool,
}
impl Scan<'_, '_> {
    fn send(&mut self, event: Event) -> Result<(), QueryError> {
        let result = (self.emit)(event);
        if result.is_err() {
            self.stopped = true;
        }
        result
    }

    fn wants_types(&self) -> bool {
        self.targets
            .iter()
            .any(|target| matches!(target, Target::Type(_)))
    }
    fn typed(&mut self, site: &Site, units: &[u16], use_: &str) -> Result<(), QueryError> {
        let result = (|| -> Result<(), QueryError> {
            for target in self.targets {
                if let Target::Type(filter) = target
                    && let Some(selected) = filter.evaluate_units(units)?
                {
                    return self.row(site, Matched::Type(copy(units)?), selected, use_);
                }
            }
            Ok(())
        })();
        if let Err(error) = result {
            if self.stopped {
                return Err(error);
            }
            self.issue(&format!("{site:?} {use_}"), &error)?;
        }
        Ok(())
    }
    fn value(&mut self, site: &Site, value: Matched, use_: &str) -> Result<(), QueryError> {
        match matched_value(value, self.targets) {
            Ok(Some((value, selected))) => self.row(site, value, selected, use_),
            Ok(None) => Ok(()),
            Err(error) => self.issue(use_, &error),
        }
    }
    fn row(
        &mut self,
        site: &Site,
        value: Matched,
        selected: crate::analysis::filter::FilterMatch,
        use_: &str,
    ) -> Result<(), QueryError> {
        let unavailable = match site {
            Site::UnknownField(id) => self
                .facts
                .field_entry(*id)
                .map(|entry| entry.declaration().unavailable()),
            Site::UnknownMethod(id) => self
                .facts
                .method_entry(*id)
                .map(|entry| entry.declaration().unavailable()),
            _ => None,
        };
        if let Some(Some(error)) = unavailable {
            self.send(Event::Issue(Issue::from_request(
                format!("{} {site:?}", self.location.context()),
                error,
            )))?;
        } else if matches!(site, Site::UnknownField(_) | Site::UnknownMethod(_)) {
            self.send(Event::Issue(Issue::unknown(
                format!("{} {site:?}", self.location.context()),
                "member declaration identity unavailable in published class directory",
            )))?;
        }
        self.send(self.location.row(
            site.try_copy()?,
            value,
            selected,
            use_.to_owned(),
            self.depth,
        )?)
    }
    fn prefix(&mut self, site: &Site, use_: &str, range: [usize; 2]) -> Result<(), QueryError> {
        let mut issue = Issue::unknown(
            format!("{} {site:?} {use_}", self.location.context()),
            "encoding is a retained prefix, not a complete value; range identifies the retained container prefix",
        );
        issue.range_bytes = Some(range);
        self.send(Event::Issue(issue))
    }
    fn issue(&mut self, scope: &str, error: &QueryError) -> Result<(), QueryError> {
        self.send(Event::Issue(Issue::new(
            format!("{} {scope}", self.location.context()),
            error,
            None,
        )))
    }
    fn member(&mut self, site: &Site, member: &ValueMember, use_: &str) -> Result<(), QueryError> {
        match member {
            ValueMember::Field {
                owner,
                name,
                field_type,
            } => {
                if self.wants_types() {
                    self.typed(site, &owner.descriptor, &format!("{use_} owner-type"))?;
                    self.typed(site, &field_type.descriptor, &format!("{use_} field-type"))?;
                }
                if !self
                    .targets
                    .iter()
                    .any(|target| matches!(target, Target::Field { .. }))
                {
                    return Ok(());
                }
                self.value(
                    site,
                    Matched::Field {
                        owner: copy(&owner.descriptor)?,
                        name: copy(name)?,
                        field_type: copy(&field_type.descriptor)?,
                    },
                    use_,
                )
            }
            ValueMember::Method {
                owner,
                name,
                prototype: signature,
            } => {
                if self.wants_types() {
                    self.typed(site, &owner.descriptor, &format!("{use_} owner-type"))?;
                    self.prototype(site, signature, use_)?;
                }
                if !self
                    .targets
                    .iter()
                    .any(|target| matches!(target, Target::Method { .. }))
                {
                    return Ok(());
                }
                self.value(
                    site,
                    Matched::Method {
                        owner: copy(&owner.descriptor)?,
                        name: copy(name)?,
                        prototype: prototype(signature)?,
                    },
                    use_,
                )
            }
        }
    }
    fn prototype(
        &mut self,
        site: &Site,
        signature: &ValuePrototype,
        use_: &str,
    ) -> Result<(), QueryError> {
        if !self.wants_types() {
            return Ok(());
        }
        self.typed(
            site,
            &signature.returns.descriptor,
            &format!("{use_} return-type"),
        )?;
        for (ordinal, parameter) in signature.parameters.iter().enumerate() {
            self.typed(
                site,
                &parameter.descriptor,
                &format!("{use_} parameter-type[{ordinal}]"),
            )?;
        }
        Ok(())
    }
    fn tokens(
        &mut self,
        site: &Site,
        tokens: &[AnnotationToken],
        references: &[diana_decomp::input::value_facts::ValueReference],
        use_: &str,
    ) -> Result<(), QueryError> {
        for token in tokens {
            let AnnotationToken::Value(value) = token else {
                continue;
            };
            let result = (|| -> Result<(), QueryError> {
                match value.kind() {
                    AnnotationValueKind::Signed(value) => {
                        if let Ok(value) = i32::try_from(*value)
                            && let Some((matched, selected)) = super::resource_match(
                                u32::from_le_bytes(value.to_le_bytes()),
                                self.targets,
                            )?
                        {
                            self.row(
                                site,
                                matched,
                                selected,
                                &format!("{use_} resource-id-literal candidate"),
                            )?;
                        }
                    }
                    AnnotationValueKind::Unsigned(value) => {
                        if let Ok(value) = u32::try_from(*value)
                            && let Some((matched, selected)) =
                                super::resource_match(value, self.targets)?
                        {
                            self.row(
                                site,
                                matched,
                                selected,
                                &format!("{use_} resource-id-literal candidate"),
                            )?;
                        }
                    }
                    AnnotationValueKind::String { units, .. }
                        if self
                            .targets
                            .iter()
                            .any(|target| matches!(target, Target::String(_))) =>
                    {
                        self.value(site, Matched::String(copy(units)?), use_)?
                    }
                    AnnotationValueKind::Annotation { descriptor, .. } if self.wants_types() => {
                        self.typed(site, descriptor, &format!("{use_} annotation-type"))?
                    }
                    AnnotationValueKind::Reference { declaration, .. } => {
                        let index = usize::try_from(declaration.0)
                            .map_err(|_| crate::analysis::input::InputError::PlatformCapacity)?;
                        let reference = references.get(index).ok_or({
                            QueryError::Engine(diana_decomp::input::InputError::InternalInvariant(
                                "P0 value lost its reference",
                            ))
                        })?;
                        if !super::needs_reference(reference.original, self.targets) {
                            return Ok(());
                        }
                        match &reference.declaration {
                            Err(error) => self.issue(use_, &(*error).into())?,
                            Ok(ValueDeclaration::String(units)) => {
                                self.value(site, Matched::String(copy(units)?), use_)?
                            }
                            Ok(ValueDeclaration::Type(value)) => {
                                self.typed(site, &value.descriptor, use_)?
                            }
                            Ok(ValueDeclaration::Prototype(signature)) => {
                                self.prototype(site, signature, use_)?
                            }
                            Ok(ValueDeclaration::Member(member)) => {
                                self.member(site, member, use_)?
                            }
                            Ok(ValueDeclaration::Handle { declaration, .. }) => {
                                self.member(site, declaration, &format!("{use_} method-handle"))?
                            }
                        }
                    }
                    _ => (),
                }
                Ok(())
            })();
            if let Err(error) = result {
                if self.stopped {
                    return Err(error);
                }
                let range = value.range();
                self.send(Event::Issue(Issue::new(
                    format!("{} {site:?} {use_} encoded value", self.location.context()),
                    &error,
                    Some([range.start.0, range.end.0]),
                )))?;
            }
        }
        Ok(())
    }
}

pub(super) fn scan(
    facts: &ClassFacts,
    location: &InputLocation<'_>,
    targets: &[Target],
    depth: u32,
    emit: &mut dyn FnMut(Event) -> Result<(), QueryError>,
) -> Result<(), QueryError> {
    let mut scan = Scan {
        location,
        facts,
        targets,
        emit,
        depth,
        stopped: false,
    };
    for (scope, issue) in [
        ("fields", facts.fields_issue()),
        ("methods", facts.methods_issue()),
        ("annotations", facts.annotations_issue()),
    ] {
        if let Some(issue) = issue {
            scan.issue(scope, &borrowed_error(issue))?;
        }
    }
    // Each local fact is checked independently: one unavailable field or
    // annotation does not suppress signatures, other records, or method bodies.
    if scan.wants_types() {
        match facts.superclass() {
            Fact::Present(value) => scan.value(
                &Site::Class,
                Matched::Type(copy(value.units())?),
                "superclass-type",
            )?,
            Fact::Unavailable(error) => scan.issue("superclass", &borrowed_error(error))?,
            _ => (),
        }
        match facts.interfaces() {
            Fact::Present(values) => {
                for value in values {
                    match &value.descriptor {
                        Fact::Present(value) => scan.value(
                            &Site::Class,
                            Matched::Type(copy(value.units())?),
                            "interface-type",
                        )?,
                        Fact::Unavailable(error) => {
                            scan.issue("interface", &borrowed_error(error))?
                        }
                        _ => (),
                    }
                }
            }
            Fact::Unavailable(error) => scan.issue("interfaces", &borrowed_error(error))?,
            _ => (),
        }
    }
    for field in facts.fields() {
        let result = (|| -> Result<(), QueryError> {
            let site = field_site(facts, field.field())?;
            if scan.wants_types() {
                match field.declaration() {
                    Fact::Present(value) => {
                        scan.typed(&site, value.descriptor.units(), "field-type")?
                    }
                    Fact::Unavailable(error) => scan.issue(
                        &format!("field:{}", field.field().0),
                        &borrowed_error(error),
                    )?,
                    _ => (),
                }
            }
            match field.initial_value() {
                Fact::Present(InitialValue::Encoded {
                    tokens,
                    references,
                    extent,
                    range_bytes,
                    ..
                }) => {
                    scan.tokens(&site, tokens, references, "static-value")?;
                    if *extent == diana_decomp::AnnotationExtent::Prefix {
                        scan.prefix(&site, "static-value", *range_bytes)?;
                    }
                }
                Fact::Unavailable(error) => scan.issue(
                    &format!("field:{} preparation", field.field().0),
                    &borrowed_error(error),
                )?,
                _ => (),
            }

            Ok(())
        })();
        if let Err(error) = result {
            if scan.stopped {
                return Err(error);
            }
            scan.issue(&format!("field:{}", field.field().0), &error)?;
        }
    }
    for method in facts.methods() {
        if !scan.wants_types() {
            continue;
        }
        let result = (|| -> Result<(), QueryError> {
            let site = method_site(facts, method.method())?;
            match method.declaration() {
                Fact::Present(value) => {
                    scan.typed(&site, value.return_type.units(), "return-type")?;
                    for (ordinal, parameter) in value.parameter_types.iter().enumerate() {
                        scan.typed(
                            &site,
                            parameter.units(),
                            &format!("parameter-type[{ordinal}]"),
                        )?;
                    }
                }
                Fact::Unavailable(error) => scan.issue(
                    &format!("method:{} declaration", method.method().0),
                    &borrowed_error(error),
                )?,
                _ => (),
            }

            Ok(())
        })();
        if let Err(error) = result {
            if scan.stopped {
                return Err(error);
            }
            scan.issue(&format!("method:{}", method.method().0), &error)?;
        }
    }
    for record in facts.annotations().records() {
        let result = (|| -> Result<(), QueryError> {
            let site = match record.location() {
                AnnotationLocation::Class => Site::Class,
                AnnotationLocation::Field(id) => field_site(facts, id)?,
                AnnotationLocation::Method(id)
                | AnnotationLocation::Parameter { method: id, .. } => method_site(facts, id)?,
            };
            let use_ = match record.location() {
                AnnotationLocation::Parameter { parameter, .. } => {
                    format!("annotation parameter[{}]", parameter.0)
                }
                _ => "annotation".to_owned(),
            };
            if scan.wants_types() {
                scan.typed(
                    &site,
                    record.descriptor(),
                    &format!("{use_} annotation-type"),
                )?;
            }
            scan.tokens(
                &site,
                record.tokens(),
                facts.annotations().references(),
                &use_,
            )?;
            if record.extent() == diana_decomp::AnnotationExtent::Prefix {
                let range = record.range();
                scan.prefix(&site, &use_, [range.start.0, range.end.0])?;
            }

            Ok(())
        })();
        if let Err(error) = result {
            if scan.stopped {
                return Err(error);
            }
            scan.issue("annotation", &error)?;
        }
    }
    Ok(())
}

// RequestError is not Clone. Preserve its kind and concrete diagnostic in an
// application query error rather than replacing a local failure with absence.
fn borrowed_error(error: &diana_decomp::RequestError) -> QueryError {
    QueryError::Local {
        kind: error.kind(),
        reason: error.to_string(),
    }
}

/// Declaration signature rows for a pure type query.
///
/// Reads field and method signatures without constructing a class snapshot.
pub(super) fn scan_type_declarations<'a>(
    class: &diana_decomp::input::ClassView<'_, 'a>,
    location: &InputLocation<'a>,
    targets: &[Target],
    depth: u32,
    emit: &mut dyn FnMut(Event) -> Result<(), QueryError>,
) -> Result<(), QueryError> {
    let mut failure = None;
    let visited = class.visit_fields(|field| {
        if failure.is_some() {
            return Ok(());
        }
        let result = (|| -> Result<(), QueryError> {
            let declaration = match field.declaration {
                Ok(declaration) => declaration,
                Err(_) => return Ok(()),
            };
            emit_type_row(
                location,
                targets,
                depth,
                emit,
                || {
                    Ok(Site::Field {
                        field: field.field_idx,
                        name: declaration.name.units()?,
                        field_type: declaration.field_type.units()?,
                    })
                },
                declaration.field_type,
                || "field-type".to_owned(),
            )
        })();
        if let Err(error) = result {
            failure = Some(error);
        }
        Ok(())
    });
    if let Err(error) = visited {
        emit(Event::Issue(Issue::new(
            format!("{} fields", location.context()),
            &QueryError::from(error),
            None,
        )))?;
    }
    if let Some(error) = failure {
        return Err(error);
    }
    let mut failure = None;
    let visited = class.visit_method_headers(None, |method_idx, _, declaration| {
        if failure.is_some() {
            return Ok(());
        }
        let result = (|| -> Result<(), QueryError> {
            let Ok(declaration) = declaration else {
                return Ok(());
            };
            let site = || {
                Ok(Site::Method {
                    method: method_idx,
                    name: declaration.name.units()?,
                    prototype: prototype_units(&declaration.prototype)?,
                })
            };
            emit_type_row(
                location,
                targets,
                depth,
                emit,
                site,
                declaration.prototype.return_type,
                || "return-type".to_owned(),
            )?;
            for (ordinal, parameter) in declaration
                .prototype
                .parameter_descriptors()
                .iter()
                .enumerate()
            {
                emit_type_row(location, targets, depth, emit, site, *parameter, || {
                    format!("parameter-type[{ordinal}]")
                })?;
            }
            Ok(())
        })();
        if let Err(error) = result {
            failure = Some(error);
        }
        Ok(())
    });
    if let Err(error) = visited {
        emit(Event::Issue(Issue::new(
            format!("{} methods", location.context()),
            &QueryError::from(error),
            None,
        )))?;
    }
    if let Some(error) = failure {
        return Err(error);
    }
    Ok(())
}

/// One declaration type row when the target filter selects it.
fn emit_type_row(
    location: &InputLocation<'_>,
    targets: &[Target],
    depth: u32,
    emit: &mut dyn FnMut(Event) -> Result<(), QueryError>,
    site: impl FnOnce() -> Result<Site, QueryError>,
    value: diana_decomp::input::DexString<'_>,
    use_: impl FnOnce() -> String,
) -> Result<(), QueryError> {
    for target in targets {
        let Target::Type(filter) = target else {
            continue;
        };
        let Some(selected) = filter.evaluate(value)? else {
            continue;
        };
        return emit(location.row(
            site()?,
            Matched::Type(value.units()?),
            selected,
            use_(),
            depth,
        )?);
    }
    Ok(())
}

/// Declaration rows that live outside code.
///
/// An annotation value or a static initial value can carry an encoded
/// reference to a string, a field, a method or a method handle. Reading those
/// values directly keeps a reference query from building a full class
/// snapshot, which parses every declaration, annotation and body view in the
/// class.
pub(super) fn scan_declared_values<'a>(
    index: &ProgramIndex<'a>,
    class: &diana_decomp::input::ClassView<'_, 'a>,
    location: &InputLocation<'a>,
    targets: &[Target],
    depth: u32,
    include_values: bool,
    emit: &mut dyn FnMut(Event) -> Result<(), QueryError>,
) -> Result<(), QueryError> {
    let context = index.class_context(location.class_def)?;
    let annotated = context.annotations_offset.0 != 0;
    let values = include_values && matches!(context.values_offset, Ok(Some(_)));
    if !annotated && !values {
        return Ok(());
    }
    let mut scan = MemberScan {
        index,
        location,
        targets,
        depth,
        emit,
        stopped: false,
        type_query: targets
            .iter()
            .any(|target| matches!(target, Target::Type(_))),
    };
    if annotated {
        let directory = index.annotation_directory(context.annotations_offset)?;
        scan.annotation_set(directory.class, None, "annotation")?;
        scan.member_annotations(class, directory.fields, false)?;
        scan.member_annotations(class, directory.methods, true)?;
        scan.parameter_annotations(class, directory.parameters)?;
    }
    if let Ok(Some(offset)) = context.values_offset {
        scan.static_values(class, offset)?;
    }
    Ok(())
}

struct MemberScan<'i, 'a, 'e> {
    index: &'i ProgramIndex<'a>,
    location: &'e InputLocation<'a>,
    targets: &'e [Target],
    depth: u32,
    emit: &'e mut dyn FnMut(Event) -> Result<(), QueryError>,
    stopped: bool,
    type_query: bool,
}

impl<'i, 'a> MemberScan<'i, 'a, '_> {
    fn row(
        &mut self,
        owner: Option<&AnnotationOwner>,
        matched: Matched,
        selected: FilterMatch,
        use_: &str,
    ) -> Result<(), QueryError> {
        let site = match owner {
            None => Site::Class,
            Some(AnnotationOwner::Field(id)) => field_site_at(self.index, *id)?,
            Some(AnnotationOwner::Method(id)) => method_site_at(self.index, *id)?,
        };
        let event = self
            .location
            .row(site, matched, selected, use_.to_owned(), self.depth)?;
        let result = (self.emit)(event);
        if result.is_err() {
            self.stopped = true;
        }
        result
    }

    fn issue(&mut self, scope: &str, error: &QueryError) -> Result<(), QueryError> {
        let event = Event::Issue(Issue::new(
            format!("{} {scope}", self.location.context()),
            error,
            None,
        ));
        let result = (self.emit)(event);
        if result.is_err() {
            self.stopped = true;
        }
        result
    }

    fn annotation_set(
        &mut self,
        offset: diana_decomp::ByteOffset,
        owner: Option<&AnnotationOwner>,
        use_: &str,
    ) -> Result<(), QueryError> {
        if offset.0 == 0 {
            return Ok(());
        }
        let mut offsets = self.index.annotation_offsets(0x1003, offset)?;
        let mut last = None;
        while offsets.remaining_byte().0 != 0 {
            let (_at, _visibility, type_id, mut values) =
                self.index.next_annotation(&mut offsets, &mut last)?;
            if self.type_query {
                let descriptor = self.index.type_name(type_id)?;
                self.typed(owner, descriptor, || format!("{use_} annotation-type"))?;
            }
            self.values(&mut values, owner, use_)?;
        }
        Ok(())
    }

    /// One declaration type row when the target filter selects it.
    fn typed(
        &mut self,
        owner: Option<&AnnotationOwner>,
        value: diana_decomp::input::DexString<'_>,
        use_: impl FnOnce() -> String,
    ) -> Result<(), QueryError> {
        for target in self.targets {
            let Target::Type(filter) = target else {
                continue;
            };
            let Some(selected) = filter.evaluate(value)? else {
                continue;
            };
            return self.row(owner, Matched::Type(value.units()?), selected, &use_());
        }
        Ok(())
    }

    fn values(
        &mut self,
        values: &mut EncodedValues<'a>,
        owner: Option<&AnnotationOwner>,
        use_: &str,
    ) -> Result<(), QueryError> {
        while let Some(step) = values.next(self.index)? {
            let ValueStep::Value(value) = step else {
                continue;
            };
            self.value(&value, owner, use_)?;
        }
        Ok(())
    }

    /// One encoded value: only original member references can answer a member
    /// query, and every leaf keeps its original path.
    fn value(
        &mut self,
        value: &EncodedValue<'a>,
        owner: Option<&AnnotationOwner>,
        use_: &str,
    ) -> Result<(), QueryError> {
        if let EncodedKind::Annotation { type_id, .. } = value.kind {
            if self.type_query {
                let descriptor = self.index.type_name(type_id)?;
                self.typed(owner, descriptor, || format!("{use_} annotation-type"))?;
            }
            return Ok(());
        }
        let EncodedKind::Reference(reference) = value.kind else {
            return Ok(());
        };
        if !super::needs_reference(reference, self.targets) {
            return Ok(());
        }
        let index = self.index;
        index.visit_reference_leaves::<QueryError>(reference, |path, facts| {
            let facts = match facts {
                Ok(facts) => facts,
                Err(error) => {
                    let context = match owner {
                        None => "class".to_owned(),
                        Some(AnnotationOwner::Field(id)) => format!("field:{}", id.0),
                        Some(AnnotationOwner::Method(id)) => format!("method:{}", id.0),
                    };
                    return self.issue(
                        &format!("{context} {use_} reference"),
                        &QueryError::from(error),
                    );
                }
            };
            let Some((matched, selected)) = super::matched(facts, self.targets)? else {
                return Ok(());
            };
            let label = match reference {
                diana_decomp::raw::OperandReference::MethodHandle(_) => {
                    format!("{use_} method-handle")
                }
                _ if path.is_empty() => use_.to_owned(),
                _ => format!("{use_} → {path}"),
            };
            self.row(owner, matched, selected, &label)
        })
    }

    fn member_annotations(
        &mut self,
        class: &diana_decomp::input::ClassView<'_, 'a>,
        mut records: Cursor<'a>,
        method: bool,
    ) -> Result<(), QueryError> {
        if records.remaining_byte().0 == 0 {
            return Ok(());
        }
        let index = self.index;
        let mut members = AnnotationMembers::open(index, class, method)?;
        let mut last = None;
        while records.remaining_byte().0 != 0 {
            let (owner, offset, _parameters) =
                association(index, class, &mut members, &mut records, &mut last, method)?;
            self.annotation_set(offset, Some(&owner), "annotation")?;
        }
        Ok(())
    }

    fn parameter_annotations(
        &mut self,
        class: &diana_decomp::input::ClassView<'_, 'a>,
        mut records: Cursor<'a>,
    ) -> Result<(), QueryError> {
        if records.remaining_byte().0 == 0 {
            return Ok(());
        }
        let index = self.index;
        let mut members = AnnotationMembers::open(index, class, true)?;
        let mut last = None;
        while records.remaining_byte().0 != 0 {
            let (owner, offset, parameters) =
                association(index, class, &mut members, &mut records, &mut last, true)?;
            let AnnotationOwner::Method(id) = owner else {
                return Err(QueryError::Engine(
                    diana_decomp::input::InputError::InternalInvariant(
                        "parameter annotation mapped to a field",
                    ),
                ));
            };
            let owner = AnnotationOwner::Method(id);
            let mut references = index.annotation_parameters(offset, parameters)?;
            for parameter in 0..parameters {
                let value = references.read_u32()?;
                let offset = diana_decomp::ByteOffset(usize::try_from(value).map_err(|_| {
                    diana_decomp::input::InputError::PlatformCapacity {
                        offset_byte: diana_decomp::ByteOffset(0),
                    }
                })?);
                self.annotation_set(
                    offset,
                    Some(&owner),
                    &format!("annotation parameter[{parameter}]"),
                )?;
            }
        }
        Ok(())
    }

    /// Static initial values are encoded in class-data order, one root value
    /// per static field; nested containers keep their root field.
    fn static_values(
        &mut self,
        class: &diana_decomp::input::ClassView<'_, 'a>,
        offset: diana_decomp::ByteOffset,
    ) -> Result<(), QueryError> {
        let mut fields = Vec::new();
        class.visit_fields(|field| {
            if let diana_decomp::input::FieldStorage::Static { ordinal } = field.storage {
                fields
                    .try_reserve(1)
                    .map_err(|_| diana_decomp::input::InputError::Allocation {
                        offset_byte: diana_decomp::ByteOffset(0),
                    })?;
                fields.push((ordinal, field.field_idx));
            }
            Ok(())
        })?;
        if fields.is_empty() {
            return Ok(());
        }
        let count = u32::try_from(fields.len()).map_err(|_| {
            diana_decomp::input::InputError::PlatformCapacity {
                offset_byte: diana_decomp::ByteOffset(0),
            }
        })?;
        let mut values = EncodedValues::open(self.index, offset, Some(count))?;
        let mut owner = None;
        let mut depth = 0usize;
        while let Some(step) = values.next(self.index)? {
            match step {
                ValueStep::EndContainer => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        owner = None;
                    }
                }
                ValueStep::Value(value) => {
                    if depth == 0 {
                        owner = fields
                            .get(value.ordinal.0 as usize)
                            .map(|(_, field)| AnnotationOwner::Field(*field));
                    }
                    let Some(current) = owner.as_ref() else {
                        continue;
                    };
                    self.value(&value, Some(current), "static-value")?;
                    if matches!(
                        value.kind,
                        EncodedKind::Array { .. } | EncodedKind::Annotation { .. }
                    ) {
                        depth = depth.saturating_add(1);
                    }
                }
            }
        }
        Ok(())
    }
}

fn field_site_at(index: &ProgramIndex<'_>, id: diana_decomp::FieldId) -> Result<Site, QueryError> {
    Ok(match index.field_reference(id) {
        Ok(reference) => Site::Field {
            field: id,
            name: copy(&reference.declaration.name.units()?)?,
            field_type: copy(&reference.declaration.field_type.units()?)?,
        },
        Err(_) => Site::UnknownField(id),
    })
}

fn method_site_at(
    index: &ProgramIndex<'_>,
    id: diana_decomp::MethodId,
) -> Result<Site, QueryError> {
    Ok(match index.method_reference(id) {
        Ok(reference) => Site::Method {
            method: id,
            name: copy(&reference.declaration.name.units()?)?,
            prototype: prototype_units(&reference.declaration.prototype)?,
        },
        Err(_) => Site::UnknownMethod(id),
    })
}
