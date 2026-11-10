//! Original and delivered method declarations, including parameter metadata.

use crate::p9::{
    Origin, RenderError,
    buffer::Buffer,
    names::{Lookup, MethodNames},
    values::Attachments,
};
use crate::{MethodId, p0::Fact, p7, p8};

pub(super) fn header_line(
    out: &mut Buffer,
    lookup: &Lookup<'_>,
    names: &MethodNames,
    h: &p7::MethodHeader,
    owner: (Option<&p8::ValueName>, Option<p8::TypeUseId>),
    attachments: Option<&Attachments<'_>>,
    suspending: bool,
) -> Result<(), RenderError> {
    out.outline_header = Some(Origin::Method(h.key.method));
    out.indent()?;
    // A class initializer is a static block. DEX may give it access bits that
    // Java cannot express there; they stay in the original flags of the product.
    if matches!(h.kind, p7::MethodKind::ClassInitializer) {
        out.put("static")?;
        return Ok(());
    }
    let varargs = varargs_parameter(lookup, names, h);
    crate::p9::names::flags(
        out,
        h.access_flags,
        crate::p9::names::FlagScope::Method {
            initializer: matches!(h.kind, p7::MethodKind::Constructor),
            varargs,
        },
    )?;
    if suspending {
        out.put("suspend ")?;
    }
    let method = h.key.method;
    if let Some(parameters) = lookup.method_parameters(method)
        && !parameters.is_empty()
    {
        crate::p9::names::type_parameters(out, lookup, parameters)?;
        out.put(" ")?;
    }
    match h.kind {
        p7::MethodKind::ClassInitializer => unreachable!("handled above"),
        // A constructor carries the owner name and no return type.
        p7::MethodKind::Constructor => {
            // The class declaration name is the constructor name. A reference
            // spelling can be qualified by an unrelated field or local shadow.
            match owner {
                (Some(name), _) => out.name(&name.units, name.form)?,
                (None, Some(id)) => lookup.print(out, id)?,
                (None, None) => out.name(h.declaration.name_units(), h.name_form)?,
            }
            out.put("(")?;
            parameters(out, lookup, names, h, attachments, varargs)?;
            out.put(")")?;
            return Ok(());
        }
        p7::MethodKind::Ordinary => {}
    }
    match lookup.signature(p8::SignatureOwner::Return(method)) {
        Some(root) => crate::p9::names::signature(out, lookup, root)?,
        None => match names.result {
            Some(ty) => lookup.print(out, ty)?,
            None => {
                out.put("type<")?;
                out.units(h.declaration.return_type.units(), true)?;
                out.put("; display unavailable>")?;
            }
        },
    }
    out.put(" ")?;
    out.name(h.declaration.name_units(), h.name_form)?;
    out.put("(")?;
    parameters(out, lookup, names, h, attachments, varargs)?;
    out.put(")")
}

fn varargs_parameter(lookup: &Lookup<'_>, names: &MethodNames, h: &p7::MethodHeader) -> bool {
    if h.access_flags & crate::substrate::flags::VARARGS == 0 {
        return false;
    }
    let Some((index, ty)) = h.declaration.parameter_types.iter().enumerate().next_back() else {
        return false;
    };
    if ty.units().first() != Some(&u16::from(b'[')) {
        return false;
    }
    let Ok(parameter) = u32::try_from(index) else {
        return false;
    };
    match lookup.signature(p8::SignatureOwner::Parameter {
        method: h.key.method,
        index: parameter,
    }) {
        Some(root) => crate::p9::names::signature_varargs_available(lookup, root),
        None => names
            .parameters
            .get(index)
            .copied()
            .flatten()
            .and_then(|id| lookup.varargs_dimensions(id))
            .is_some(),
    }
}

fn parameters(
    out: &mut Buffer,
    lookup: &Lookup<'_>,
    names: &MethodNames,
    h: &p7::MethodHeader,
    attachments: Option<&Attachments<'_>>,
    varargs: bool,
) -> Result<(), RenderError> {
    let method = h.key.method;
    for (i, ty) in h.declaration.parameter_types.iter().enumerate() {
        if i > 0 {
            out.put(", ")?;
        }
        let index = u32::try_from(i).map_err(|_| RenderError::Capacity)?;
        // A parameter record prints inline before the type it describes, because
        // this parameter list is that record's declaration.
        if let Some(attachments) = attachments {
            attachments.print_parameter(out, lookup, method, index)?;
        }
        if let Some(root) = lookup.signature(p8::SignatureOwner::Parameter { method, index }) {
            if varargs && i + 1 == h.declaration.parameter_types.len() {
                crate::p9::names::signature_varargs(out, lookup, root)?;
            } else {
                crate::p9::names::signature(out, lookup, root)?;
            }
            out.put(" ")?;
            out.put(&h.parameter_names[i])?;
            continue;
        }
        if let Some(id) = names.parameters[i] {
            if varargs && i + 1 == h.declaration.parameter_types.len() {
                lookup.print_varargs(out, id)?;
            } else {
                lookup.print(out, id)?;
            }
        } else {
            out.put("type<")?;
            out.units(ty.units(), true)?;
            out.put("; display unavailable>")?;
        }
        out.put(" ")?;
        out.put(&h.parameter_names[i])?;
    }
    Ok(())
}

pub(super) fn original_header(
    out: &mut Buffer,
    facts: &crate::p0::ClassFacts,
    id: MethodId,
) -> Result<(), RenderError> {
    out.outline_header = Some(Origin::Method(id));
    out.indent()?;
    out.fmt(format_args!("method#{} ", id.0))?;
    if let Some(entry) = facts.method_entry(id) {
        match entry.declaration() {
            Fact::Present(d) => {
                out.units(d.name_units(), true)?;
                out.put(" ")?;
                out.units(d.prototype_units(), true)?;
            }
            Fact::Unavailable(e) => out.fmt(format_args!("unavailable_declaration<{e}>"))?,
            Fact::NotRead => out.put("declaration_not_read")?,
            Fact::Absent => out.put("absent_declaration")?,
        }
    } else {
        out.put("directory_entry_unavailable")?;
    }
    Ok(())
}
