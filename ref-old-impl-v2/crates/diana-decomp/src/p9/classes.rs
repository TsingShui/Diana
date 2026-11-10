mod declarations;
mod methods;
mod recovery;

use super::{
    Origin, RenderError, SourceProduct, buffer::Buffer, names::Lookup, values::Attachments,
};
use crate::{p0::Fact, p8};
use methods::member;

pub(super) fn render(out: &mut Buffer, source: &SourceProduct) -> Result<(), RenderError> {
    match source {
        SourceProduct::Class(class) => class_result(out, class),
        SourceProduct::Method(method) => {
            let lookup = Lookup::complete(method.names())?;
            let attachments = Attachments::standalone(method)?;
            member(
                out,
                &lookup,
                method.facts(),
                Some(&attachments),
                method.member(),
                None,
                false,
            )
        }
        SourceProduct::External(external) => out.put(&external.text),
        SourceProduct::ClassStopped(stop) | SourceProduct::MethodStopped(stop) => {
            let lookup = Lookup::new(stop.partial_name_uses(), stop.partial_name_targets())?;
            out.fmt(format_args!("assembly_unavailable<{:?}>;\n", stop.error()))?;
            if matches!(source, SourceProduct::ClassStopped(_)) {
                out.put("class original_type<")?;
                out.units(stop.facts().descriptor().units(), true)?;
                out.put(">")?;
                out.open()?;
                for f in stop.facts().fields() {
                    out.indent()?;
                    let a = out.position();
                    out.fmt(format_args!("field#{} {:?} ", f.field().0, f.storage()))?;
                    match f.declaration() {
                        Fact::Present(d) => {
                            out.units(d.descriptor.units(), true)?;
                            out.put(" ")?;
                            out.units(d.name.units(), true)?;
                        }
                        Fact::Absent => out.put("absent_declaration")?,
                        Fact::NotRead => out.put("declaration_not_read")?,
                        Fact::Unavailable(e) => {
                            out.fmt(format_args!("unavailable_declaration<{e}>"))?
                        }
                    }
                    out.put(";\n")?;
                    out.span(a, Origin::Field(f.field()))?;
                }
            }
            super::raw::assembly(out, stop, matches!(source, SourceProduct::ClassStopped(_)))?;
            for m in stop.methods() {
                member(out, &lookup, stop.facts(), None, m, None, false)?;
            }
            if matches!(source, SourceProduct::ClassStopped(_)) {
                out.close()?;
            }
            Ok(())
        }
    }
}
fn preamble(
    out: &mut Buffer,
    lookup: &Lookup<'_>,
    class: &p8::ClassResult,
) -> Result<(), RenderError> {
    if let Some(package) = class.names().package()
        && !package.units.is_empty()
    {
        out.mark_outline("package");
        out.put("package ")?;
        let owner = &lookup.uses[class.header().owner.0 as usize];
        let target = &lookup.targets[owner.target.0 as usize];
        let start = target.descriptor.len() - target.full.len() - 1;
        out.path(&target.descriptor[start..start + target.package_end], 47)?;
        out.put(";\n\n")?;
    }
    for import in class.names().imports() {
        out.mark_outline("import");
        let target = &lookup.targets[import.target.0 as usize];
        out.put("import ")?;
        out.units(&target.full, false)?;
        out.put(";\n")?;
    }
    Ok(())
}

fn class_header(
    out: &mut Buffer,
    lookup: &Lookup<'_>,
    class: &p8::ClassResult,
) -> Result<(), RenderError> {
    out.outline_header = Some(Origin::Class);
    let header = class.header();
    super::names::flags(
        out,
        header.flags,
        super::names::FlagScope::Class {
            interface: matches!(
                header.kind,
                p8::ClassKind::Interface | p8::ClassKind::Annotation
            ),
        },
    )?;
    out.put(match header.kind {
        p8::ClassKind::Class => "class ",
        p8::ClassKind::Interface => "interface ",
        p8::ClassKind::Annotation => "@interface ",
        p8::ClassKind::Enum => "enum ",
    })?;
    out.name(&header.name.units, header.name.form)?;
    if let Some(parameters) = lookup.class_parameters() {
        super::names::type_parameters(out, lookup, parameters)?;
    }
    // P8 already decided whether the clause prints at all; a delivered generic
    // superclass prints its interpreted arguments.
    if let p8::SuperclassClause::Printed(superclass) = &header.superclass {
        out.put(" extends ")?;
        match lookup.signature(p8::SignatureOwner::Superclass) {
            Some(root) => super::names::signature(out, lookup, root)?,
            None => type_expression(out, lookup, superclass)?,
        }
    }
    match &header.interfaces {
        p8::InterfaceList::Present(list) if !list.is_empty() => {
            out.put(" implements ")?;
            for (i, interface) in list.iter().enumerate() {
                if i > 0 {
                    out.put(", ")?;
                }
                let index = u32::try_from(i).map_err(|_| {
                    RenderError::Invariant("interface index exceeds the delivered coordinate")
                })?;
                match lookup.signature(p8::SignatureOwner::Interface(index)) {
                    Some(root) => super::names::signature(out, lookup, root)?,
                    None => type_expression(out, lookup, &interface.name)?,
                }
            }
        }
        p8::InterfaceList::NotRead => out.put(" interfaces_not_read")?,
        p8::InterfaceList::Unavailable => out.put(" interfaces_unavailable")?,
        _ => {}
    }
    Ok(())
}

fn field(
    out: &mut Buffer,
    lookup: &Lookup<'_>,
    class: &p8::ClassResult,
    attachments: &Attachments<'_>,
    f: &p8::FieldMember,
) -> Result<(), RenderError> {
    attachments.print_field(out, lookup, f.id)?;
    out.indent()?;
    let a = out.position();
    out.outline_header = Some(Origin::Field(f.id));
    super::names::flags(out, f.flags, super::names::FlagScope::Field)?;
    match &f.expression {
        p8::FieldExpression::Declared {
            name,
            form,
            field_type,
        } => {
            match lookup.signature(p8::SignatureOwner::Field(f.id)) {
                Some(root) => super::names::signature(out, lookup, root)?,
                None => lookup.print(out, *field_type)?,
            }
            out.put(" ")?;
            out.name(name, *form)?;
        }
        p8::FieldExpression::Absent => {
            out.fmt(format_args!("field#{} absent_declaration", f.id.0))?
        }
        p8::FieldExpression::NotRead => {
            out.fmt(format_args!("field#{} declaration_not_read", f.id.0))?
        }
        p8::FieldExpression::Unavailable => {
            out.fmt(format_args!("field#{} unavailable_declaration", f.id.0))?;
            if let Some(original) = class.facts().field_entry(f.id)
                && let Fact::Unavailable(error) = original.declaration()
            {
                out.fmt(format_args!("<{error}>"))?;
            }
        }
    }
    out.put(";\n")?;
    out.span(a, Origin::Field(f.id))?;
    Ok(())
}

fn preparations(
    out: &mut Buffer,
    lookup: &Lookup<'_>,
    class: &p8::ClassResult,
) -> Result<(), RenderError> {
    if class.preparation_has_values() {
        out.indent()?;
        out.put("static_preparation_before_clinit")?;
        out.open()?;
        for &field in class.preparation_order() {
            let f = &class.fields()[field];
            out.indent()?;
            let a = out.position();
            // The declaration name identifies the field; an unavailable
            // declaration keeps the original index instead of a guessed name.
            match &f.expression {
                p8::FieldExpression::Declared { name, form, .. } => out.name(name, *form)?,
                _ => out.fmt(format_args!("field#{}", f.id.0))?,
            }
            out.put(" = ")?;
            match f.preparation {
                p8::Preparation::ImplicitDefault => out.put("implicit_default")?,
                p8::Preparation::Encoded | p8::Preparation::EncodedPrefix => {
                    let range = class
                        .values()
                        .preparations()
                        .iter()
                        .find(|(id, _)| *id == f.id)
                        .map(|(_, range)| range.clone());
                    match range {
                        Some(range) => super::values::values(
                            out,
                            lookup,
                            &class.values().expressions()[range.start as usize..range.end as usize],
                        )?,
                        None => out.put("encoded_value_unavailable")?,
                    }
                    if matches!(f.preparation, p8::Preparation::EncodedPrefix) {
                        out.put(" /* encoded prefix */")?;
                    }
                }
                p8::Preparation::Unavailable => {
                    out.put("unavailable<")?;
                    match class
                        .facts()
                        .field_entry(f.id)
                        .map(|entry| entry.initial_value())
                    {
                        Some(Fact::Unavailable(error)) => out.fmt(format_args!("{error}"))?,
                        _ => out.put("preparation state unknown")?,
                    }
                    out.put(">")?;
                }
                p8::Preparation::Absent => out.put("encoded_value_absent")?,
                p8::Preparation::NotRead => out.put("preparation_not_read")?,
            }
            out.put(";\n")?;
            out.span(a, Origin::Preparation(f.id))?;
        }
        out.close()?;
    }
    Ok(())
}

fn class_issues(out: &mut Buffer, class: &p8::ClassResult) -> Result<(), RenderError> {
    for (kind, error) in [
        ("fields", class.facts().fields_issue()),
        ("methods", class.facts().methods_issue()),
        ("annotations", class.facts().annotations_issue()),
    ] {
        if let Some(error) = error {
            out.indent()?;
            out.fmt(format_args!("unavailable_{kind}<{error}>;\n"))?;
        }
    }
    if let Some(offset) = class.facts().coverage_terminated_at() {
        out.indent()?;
        out.fmt(format_args!(
            "directory_suffix_not_located<file_byte={offset}>;\n"
        ))?;
    }
    Ok(())
}

fn class_result(out: &mut Buffer, class: &p8::ClassResult) -> Result<(), RenderError> {
    let lookup = Lookup::complete(class.names())?;
    preamble(out, &lookup, class)?;
    // Annotation records print at the declaration they describe. A record whose
    // owner is not a printed declaration keeps the residual block in the
    // metadata position, so no record is dropped or printed twice.
    let attachments = Attachments::class(class)?;
    attachments.print_residual(out, &lookup)?;
    attachments.print_class(out, &lookup)?;
    let start = out.position();
    class_header(out, &lookup, class)?;
    out.open()?;
    for f in class.fields() {
        field(out, &lookup, class, &attachments, f)?;
    }
    preparations(out, &lookup, class)?;
    for m in class.methods() {
        member(
            out,
            &lookup,
            class.facts(),
            Some(&attachments),
            m,
            Some(&class.header().name),
            class.initializer_prints_nothing(m.id),
        )?;
    }
    class_issues(out, class)?;
    out.close()?;
    out.span(start, Origin::Class)
}
fn type_expression(
    out: &mut Buffer,
    lookup: &Lookup<'_>,
    ty: &p8::TypeExpression,
) -> Result<(), RenderError> {
    match ty {
        p8::TypeExpression::Present(id) => lookup.print(out, *id),
        p8::TypeExpression::Absent => out.put("absent_type"),
        p8::TypeExpression::NotRead => out.put("type_not_read"),
        p8::TypeExpression::Unavailable => out.put("unavailable_type"),
    }
}
