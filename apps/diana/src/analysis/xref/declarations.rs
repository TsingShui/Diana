use diana_decomp::input::{
    ProgramIndex,
    value_facts::{ValueDeclaration, ValueMember, ValuePrototype},
};
use diana_decomp::p0::{ClassFacts, Fact, InitialValue};
use diana_decomp::{
    AnnotationLocation, AnnotationToken, AnnotationValueKind, ClassDefId, ClassKey, DexId,
};

use super::{Event, FilterMatch, Issue, Matched, QueryError, Site, Target, XrefRow, copy};
use crate::analysis::{dex::ImageIndex, images::LogicalDex};

pub(super) struct Location {
    pub image: ImageIndex,
    pub logical: LogicalDex,
    pub class_def: ClassDefId,
    pub descriptor: Vec<u16>,
}
impl Location {
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
            class_descriptor: copy(&self.descriptor)?,
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
    location: &'a Location,
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
    location: &Location,
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
/// Reads `class_def`, the field and method headers, and annotation types only:
/// no class-data snapshot, no initial values, no code. The rows are the same
/// ones [`scan`] produces for a type target.
pub(super) fn scan_type_declarations(
    index: &ProgramIndex<'_>,
    class: &diana_decomp::input::ClassView<'_, '_>,
    location: &Location,
    targets: &[Target],
    depth: u32,
    emit: &mut dyn FnMut(Event) -> Result<(), QueryError>,
) -> Result<(), QueryError> {
    match class.hierarchy() {
        Ok(hierarchy) => {
            if let Some(superclass) = hierarchy.superclass {
                emit_type_row(
                    location,
                    targets,
                    depth,
                    emit,
                    Site::Class,
                    &superclass.units()?,
                    "superclass-type",
                )?;
            }
            for interface in &hierarchy.interfaces {
                emit_type_row(
                    location,
                    targets,
                    depth,
                    emit,
                    Site::Class,
                    &interface.units()?,
                    "interface-type",
                )?;
            }
        }
        Err(error) => emit(Event::Issue(Issue::new(
            location.context(),
            &QueryError::from(error),
            None,
        )))?,
    }
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
            let field_type = declaration.field_type.units()?;
            let site = Site::Field {
                field: field.field_idx,
                name: copy(&declaration.name.units()?)?,
                field_type: copy(&field_type)?,
            };
            emit_type_row(
                location,
                targets,
                depth,
                emit,
                site,
                &field_type,
                "field-type",
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
            let name = copy(&declaration.name.units()?)?;
            let prototype = prototype_units(&declaration.prototype)?;
            let site = Site::Method {
                method: method_idx,
                name,
                prototype,
            };
            emit_type_row(
                location,
                targets,
                depth,
                emit,
                site.clone(),
                &declaration.prototype.return_type.units()?,
                "return-type",
            )?;
            for (ordinal, parameter) in declaration
                .prototype
                .parameter_descriptors()
                .iter()
                .enumerate()
            {
                emit_type_row(
                    location,
                    targets,
                    depth,
                    emit,
                    site.clone(),
                    &parameter.units()?,
                    &format!("parameter-type[{ordinal}]"),
                )?;
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
    let Ok(context) = index.class_context(location.class_def) else {
        return Ok(());
    };
    if context.annotations_offset.0 == 0 {
        return Ok(());
    }
    let Ok(read) = diana_decomp::input::annotation_facts::collect(
        index,
        ClassKey {
            dex: DexId(0),
            class_def: location.class_def,
        },
    ) else {
        return Ok(());
    };
    if let Some(issue) = read.issue {
        emit(Event::Issue(Issue::from_request(
            location.context(),
            &issue,
        )))?;
    }
    for record in read.facts.records() {
        let site = match record.location() {
            diana_decomp::AnnotationLocation::Class => Site::Class,
            diana_decomp::AnnotationLocation::Field(id) => {
                let reference = index.field_reference(diana_decomp::FieldId(id.0))?;
                Site::Field {
                    field: id,
                    name: copy(&reference.declaration.name.units()?)?,
                    field_type: copy(&reference.declaration.field_type.units()?)?,
                }
            }
            diana_decomp::AnnotationLocation::Method(id)
            | diana_decomp::AnnotationLocation::Parameter { method: id, .. } => {
                let reference = index.method_reference(diana_decomp::MethodId(id.0))?;
                Site::Method {
                    method: id,
                    name: copy(&reference.declaration.name.units()?)?,
                    prototype: prototype_units(&reference.declaration.prototype)?,
                }
            }
        };
        let use_ = match record.location() {
            diana_decomp::AnnotationLocation::Parameter { parameter, .. } => {
                format!("annotation parameter[{}] annotation-type", parameter.0)
            }
            _ => "annotation annotation-type".to_owned(),
        };
        emit_type_row(
            location,
            targets,
            depth,
            emit,
            site,
            record.descriptor(),
            &use_,
        )?;
    }
    Ok(())
}

/// One declaration type row when the target filter selects it.
fn emit_type_row(
    location: &Location,
    targets: &[Target],
    depth: u32,
    emit: &mut dyn FnMut(Event) -> Result<(), QueryError>,
    site: Site,
    units: &[u16],
    use_: &str,
) -> Result<(), QueryError> {
    for target in targets {
        let Target::Type(filter) = target else {
            continue;
        };
        let Some(selected) = filter.evaluate_units(units)? else {
            continue;
        };
        emit(location.row(
            site,
            Matched::Type(copy(units)?),
            selected,
            use_.to_owned(),
            depth,
        )?)?;
        break;
    }
    Ok(())
}

/// `(parameter-types)return-type` for one prototype.
pub(super) fn prototype_units(
    prototype: &diana_decomp::input::Prototype<'_, '_>,
) -> Result<Vec<u16>, QueryError> {
    let mut units = Vec::new();
    units
        .try_reserve(2 + prototype.parameter_descriptors().len())
        .map_err(|_| QueryError::from(allocation_error()))?;
    units.push(u16::from(b'('));
    for parameter in prototype.parameter_descriptors() {
        units.extend_from_slice(&parameter.units()?);
    }
    units.push(u16::from(b')'));
    units.extend_from_slice(&prototype.return_type.units()?);
    Ok(units)
}

fn allocation_error() -> diana_decomp::input::InputError {
    diana_decomp::input::InputError::Allocation {
        offset_byte: diana_decomp::ByteOffset(0),
    }
}
