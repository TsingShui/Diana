//! Reference sites in checked JVM declaration metadata.

use diana_decomp::input::jvm::{
    ClassFile, JvmString, Member,
    annotations::{Annotation, ElementValue, Target as Placement, Value, Visibility},
    constants::FieldValue,
};

use super::{Event, FilterMatch, QueryError, Search, Site, Target, display, match_literal};
use crate::analysis::xref::Issue;

pub(super) struct Metadata<'a> {
    target: &'a Target,
    pub annotations: bool,
    field_values: bool,
}

impl<'a> Metadata<'a> {
    pub(super) fn new(class: &ClassFile<'_>, search: Search<'a>, pool_match: bool) -> Option<Self> {
        let Search::Initial(target) = search else {
            return None;
        };
        let matching = matches!(
            target,
            Target::Type(_) | Target::String(_) | Target::JvmResource(_) | Target::Field { .. }
        );
        let annotations = matching && class.may_have_annotations();
        let field_values =
            pool_match && matches!(target, Target::String(_) | Target::JvmResource(_));
        (annotations || field_values).then_some(Self {
            target,
            annotations,
            field_values,
        })
    }

    pub(super) fn field(
        &self,
        class: &ClassFile<'_>,
        field: Member<'_>,
        site: Site<'_>,
        emit: &mut dyn FnMut(Event) -> Result<(), QueryError>,
    ) -> Result<(), QueryError> {
        self.read(class, Some(field), site, emit)?;
        if self.field_values {
            let attribute = match class.field_constant(field) {
                Ok(attribute) => attribute,
                Err(error) => return issue(site, error, emit),
            };
            if let Some(value) = attribute
                && let FieldValue::String(text) = value.value
                && let Some(selected) = match_literal(self.target, text)?
            {
                let role = if matches!(self.target, Target::JvmResource(_)) {
                    "resource-path-constant"
                } else if field.access_flags() & 0x0008 != 0 {
                    "static-value"
                } else {
                    "field-constant"
                };
                site.hit(
                    display(text)?,
                    format!("{role} at class-file byte {}", value.offset),
                    selected.relevance,
                    emit,
                )?;
            }
        }
        Ok(())
    }

    pub(super) fn read(
        &self,
        class: &ClassFile<'_>,
        member: Option<Member<'_>>,
        site: Site<'_>,
        emit: &mut dyn FnMut(Event) -> Result<(), QueryError>,
    ) -> Result<(), QueryError> {
        if !self.annotations {
            return Ok(());
        }
        let facts = match class.annotation_facts(member) {
            Ok(facts) => facts,
            Err(error) => return issue(site, error, emit),
        };
        for error in facts.issues {
            issue(site, error, emit)?;
        }
        for entry in facts.annotations {
            let origin = Origin::new(entry.target, entry.annotation.visibility);
            self.annotation(&entry.annotation, origin, site, emit)?;
        }
        for value in facts.defaults {
            self.value(&value, Origin::Default, site, emit)?;
        }
        Ok(())
    }

    fn annotation(
        &self,
        annotation: &Annotation<'_>,
        origin: Origin,
        site: Site<'_>,
        emit: &mut dyn FnMut(Event) -> Result<(), QueryError>,
    ) -> Result<(), QueryError> {
        if let Some(descriptor) = annotation.descriptor {
            self.type_value(descriptor, origin, "type", annotation.offset, site, emit)?;
        }
        // Even a damaged element name is independent of the bound value at
        // that position. Checked values can still witness references.
        for element in &annotation.elements {
            self.value(&element.value, origin, site, emit)?;
        }
        Ok(())
    }

    fn value(
        &self,
        value: &ElementValue<'_>,
        origin: Origin,
        site: Site<'_>,
        emit: &mut dyn FnMut(Event) -> Result<(), QueryError>,
    ) -> Result<(), QueryError> {
        let at = value.offset;
        match &value.value {
            Value::String(text) => {
                if let Some(selected) = match_literal(self.target, *text)? {
                    site.hit(
                        display(*text)?,
                        origin.label(
                            if matches!(self.target, Target::JvmResource(_)) {
                                "resource-path"
                            } else {
                                "string"
                            },
                            at,
                        ),
                        selected.relevance,
                        emit,
                    )?;
                }
            }
            Value::Enum {
                descriptor: Some(descriptor),
                name,
            } => {
                self.type_value(*descriptor, origin, "enum-type", at, site, emit)?;
                if let Some(name) = name
                    && let Some(selected) = self.enum_field(*descriptor, *name)?
                {
                    let ty = display(*descriptor)?;
                    let name = display(*name)?;
                    site.hit(
                        format!("{ty}->{name}:{ty}"),
                        origin.label("enum-field", at),
                        selected.relevance,
                        emit,
                    )?;
                }
            }
            Value::Class(descriptor) => {
                self.type_value(*descriptor, origin, "class-literal", at, site, emit)?;
            }
            Value::Annotation(annotation) => self.annotation(annotation, origin, site, emit)?,
            Value::Array { values, .. } => {
                for value in values {
                    self.value(value, origin, site, emit)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn type_value(
        &self,
        descriptor: JvmString<'_>,
        origin: Origin,
        role: &str,
        at: usize,
        site: Site<'_>,
        emit: &mut dyn FnMut(Event) -> Result<(), QueryError>,
    ) -> Result<(), QueryError> {
        if let Target::Type(filter) = self.target
            && let Some(selected) = filter.evaluate_jvm(descriptor)?
        {
            site.hit(
                display(descriptor)?,
                origin.label(role, at),
                selected.relevance,
                emit,
            )?;
        }
        Ok(())
    }

    fn enum_field(
        &self,
        descriptor: JvmString<'_>,
        field: JvmString<'_>,
    ) -> Result<Option<FilterMatch>, QueryError> {
        let Target::Field {
            owner,
            name,
            field_type,
        } = self.target
        else {
            return Ok(None);
        };
        if field_type
            .as_ref()
            .is_some_and(|ty| !descriptor.units().eq(ty.iter().copied()))
        {
            return Ok(None);
        }
        let Some(name) = name.evaluate_jvm(field)? else {
            return Ok(None);
        };
        let owner = match owner {
            Some(filter) => match filter.evaluate_jvm(descriptor)? {
                Some(selected) => selected,
                None => return Ok(None),
            },
            None => FilterMatch::default(),
        };
        Ok(Some(name.combine(owner)?))
    }
}

#[derive(Clone, Copy)]
enum Origin {
    Annotation(Visibility),
    Parameter(Visibility, u8, Option<u8>),
    TypeUse(Visibility, u8),
    Default,
}

impl Origin {
    fn new(placement: Placement<'_>, visibility: Visibility) -> Self {
        match placement {
            Placement::Declaration => Self::Annotation(visibility),
            Placement::Parameter {
                encoded_index,
                descriptor_index,
            } => Self::Parameter(visibility, encoded_index, descriptor_index),
            Placement::TypeUse { target_type, .. } => Self::TypeUse(visibility, target_type),
        }
    }

    fn label(self, role: &str, offset: usize) -> String {
        let visibility = |visibility| match visibility {
            Visibility::RuntimeVisible => "runtime-visible",
            Visibility::RuntimeInvisible => "runtime-invisible",
        };
        match self {
            Self::Annotation(kind) => {
                format!(
                    "{} annotation-{role} at class-file byte {offset}",
                    visibility(kind)
                )
            }
            Self::Parameter(kind, table, index) => {
                let parameter = index.map_or_else(
                    || format!("encoded {table}, descriptor slot unavailable"),
                    |index| index.to_string(),
                );
                format!(
                    "{} parameter-annotation[{parameter}]-{role} at class-file byte {offset}",
                    visibility(kind)
                )
            }
            Self::TypeUse(kind, tag) => {
                format!(
                    "{} type-use-annotation[{tag:#04x}]-{role} at class-file byte {offset}",
                    visibility(kind)
                )
            }
            Self::Default => format!("annotation-default-{role} at class-file byte {offset}"),
        }
    }
}

fn issue(
    site: Site<'_>,
    error: diana_decomp::input::jvm::ClassError,
    emit: &mut dyn FnMut(Event) -> Result<(), QueryError>,
) -> Result<(), QueryError> {
    emit(Event::Issue(Issue::new(
        site.context(),
        &error.into(),
        None,
    )))
}
