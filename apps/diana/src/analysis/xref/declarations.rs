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
    targets: &'a [Target],
    emit: &'e mut dyn FnMut(Event) -> Result<(), QueryError>,
    depth: u32,
}
impl Scan<'_, '_> {
    fn value(&mut self, site: &Site, value: Matched, use_: &str) -> Result<(), QueryError> {
        match matched_value(value, self.targets) {
            Ok(Some((value, selected))) => (self.emit)(self.location.row(
                site.clone(),
                value,
                selected,
                use_.to_owned(),
                self.depth,
            )?),
            Ok(None) => Ok(()),
            Err(error) => self.issue(use_, &error),
        }
    }
    fn issue(&mut self, scope: &str, error: &QueryError) -> Result<(), QueryError> {
        (self.emit)(Event::Issue(Issue::new(
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
                self.value(
                    site,
                    Matched::Type(copy(&owner.descriptor)?),
                    &format!("{use_} owner-type"),
                )?;
                self.value(
                    site,
                    Matched::Type(copy(&field_type.descriptor)?),
                    &format!("{use_} field-type"),
                )?;
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
                self.value(
                    site,
                    Matched::Type(copy(&owner.descriptor)?),
                    &format!("{use_} owner-type"),
                )?;
                self.prototype(site, signature, use_)?;
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
        self.value(
            site,
            Matched::Type(copy(&signature.returns.descriptor)?),
            &format!("{use_} return-type"),
        )?;
        for (ordinal, parameter) in signature.parameters.iter().enumerate() {
            self.value(
                site,
                Matched::Type(copy(&parameter.descriptor)?),
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
            match value.kind() {
                AnnotationValueKind::Signed(value) => {
                    if let Ok(value) = i32::try_from(*value)
                        && let Some((matched, selected)) = super::resource_match(
                            u32::from_le_bytes(value.to_le_bytes()),
                            self.targets,
                        )?
                    {
                        (self.emit)(self.location.row(
                            site.clone(),
                            matched,
                            selected,
                            format!("{use_} resource-id-literal candidate"),
                            self.depth,
                        )?)?;
                    }
                }
                AnnotationValueKind::Unsigned(value) => {
                    if let Ok(value) = u32::try_from(*value)
                        && let Some((matched, selected)) =
                            super::resource_match(value, self.targets)?
                    {
                        (self.emit)(self.location.row(
                            site.clone(),
                            matched,
                            selected,
                            format!("{use_} resource-id-literal candidate"),
                            self.depth,
                        )?)?;
                    }
                }
                AnnotationValueKind::String { units, .. } => {
                    self.value(site, Matched::String(copy(units)?), use_)?
                }
                AnnotationValueKind::Annotation { descriptor, .. } => self.value(
                    site,
                    Matched::Type(copy(descriptor)?),
                    &format!("{use_} annotation-type"),
                )?,
                AnnotationValueKind::Reference { declaration, .. } => {
                    let index = usize::try_from(declaration.0)
                        .map_err(|_| crate::analysis::input::InputError::PlatformCapacity)?;
                    let reference = references.get(index).ok_or({
                        QueryError::Engine(diana_decomp::input::InputError::InternalInvariant(
                            "P0 value lost its reference",
                        ))
                    })?;
                    match &reference.declaration {
                        Err(error) => self.issue(use_, &(*error).into())?,
                        Ok(ValueDeclaration::String(units)) => {
                            self.value(site, Matched::String(copy(units)?), use_)?
                        }
                        Ok(ValueDeclaration::Type(value)) => {
                            self.value(site, Matched::Type(copy(&value.descriptor)?), use_)?
                        }
                        Ok(ValueDeclaration::Prototype(signature)) => {
                            self.prototype(site, signature, use_)?
                        }
                        Ok(ValueDeclaration::Member(member)) => self.member(site, member, use_)?,
                        Ok(ValueDeclaration::Handle { declaration, .. }) => {
                            self.member(site, declaration, &format!("{use_} method-handle"))?
                        }
                    }
                }
                _ => (),
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
        targets,
        emit,
        depth,
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
                    Fact::Unavailable(error) => scan.issue("interface", &borrowed_error(error))?,
                    _ => (),
                }
            }
        }
        Fact::Unavailable(error) => scan.issue("interfaces", &borrowed_error(error))?,
        _ => (),
    }
    for field in facts.fields() {
        let site = field_site(facts, field.field())?;
        match field.declaration() {
            Fact::Present(value) => scan.value(
                &site,
                Matched::Type(copy(value.descriptor.units())?),
                "field-type",
            )?,
            Fact::Unavailable(error) => scan.issue(
                &format!("field:{}", field.field().0),
                &borrowed_error(error),
            )?,
            _ => (),
        }
        match field.initial_value() {
            Fact::Present(InitialValue::Encoded {
                tokens, references, ..
            }) => scan.tokens(&site, tokens, references, "static-value")?,
            Fact::Unavailable(error) => scan.issue(
                &format!("field:{} preparation", field.field().0),
                &borrowed_error(error),
            )?,
            _ => (),
        }
    }
    for method in facts.methods() {
        let site = method_site(facts, method.method())?;
        match method.declaration() {
            Fact::Present(value) => {
                scan.value(
                    &site,
                    Matched::Type(copy(value.return_type.units())?),
                    "return-type",
                )?;
                for (ordinal, parameter) in value.parameter_types.iter().enumerate() {
                    scan.value(
                        &site,
                        Matched::Type(copy(parameter.units())?),
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
    }
    for record in facts.annotations().records() {
        let site = match record.location() {
            AnnotationLocation::Class => Site::Class,
            AnnotationLocation::Field(id) => field_site(facts, id)?,
            AnnotationLocation::Method(id) | AnnotationLocation::Parameter { method: id, .. } => {
                method_site(facts, id)?
            }
        };
        let use_ = match record.location() {
            AnnotationLocation::Parameter { parameter, .. } => {
                format!("annotation parameter[{}]", parameter.0)
            }
            _ => "annotation".to_owned(),
        };
        scan.value(
            &site,
            Matched::Type(copy(record.descriptor())?),
            &format!("{use_} annotation-type"),
        )?;
        scan.tokens(
            &site,
            record.tokens(),
            facts.annotations().references(),
            &use_,
        )?;
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
