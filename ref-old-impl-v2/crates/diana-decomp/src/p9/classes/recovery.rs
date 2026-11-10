//! Retained partial facts and original code for a stopped member.

use crate::p9::{
    Origin, RenderError,
    buffer::Buffer,
    expressions::Expressions,
    names::{Lookup, MethodNames},
    values::Attachments,
};
use crate::{MethodId, p7, p8};

use super::declarations::{header_line, original_header};

pub(super) fn stopped_method(
    out: &mut Buffer,
    lookup: &Lookup<'_>,
    facts: &crate::p0::ClassFacts,
    attachments: Option<&Attachments<'_>>,
    method: MethodId,
    stop: &p8::MemberStop,
) -> Result<(), RenderError> {
    if let Some(header) = stop.expression.as_ref().and_then(|p| p.header()) {
        let names = MethodNames::collect(
            method,
            stop.expression.as_ref().map_or(0, |p| p.type_uses().len()),
            header.declaration.parameter_count(),
            lookup,
        )?;
        header_line(
            out,
            lookup,
            &names,
            header,
            (None, None),
            attachments,
            false,
        )?;
    } else {
        original_header(out, facts, method)?;
    }
    out.open()?;
    out.indent()?;
    out.fmt(format_args!(
        "unrecovered_control<stage={:?}; kind={:?}; ",
        stop.stage, stop.cause_kind
    ))?;
    match &stop.cause {
        Ok(c) => out.quote(&c.detail)?,
        Err(e) => out.fmt(format_args!("diagnostic_unavailable: {e}"))?,
    }
    out.put(">;\n")?;
    if let Some(prefix) = &stop.expression {
        partial_facts(out, lookup, method, prefix)?;
    }
    retained_code(out, method, stop)?;
    out.close()?;
    Ok(())
}

fn partial_facts(
    out: &mut Buffer,
    lookup: &Lookup<'_>,
    method: MethodId,
    prefix: &p7::ExpressionPartial,
) -> Result<(), RenderError> {
    let names = MethodNames::collect(
        method,
        prefix.type_uses().len(),
        prefix
            .header()
            .map_or(0, |h| h.declaration.parameter_count()),
        lookup,
    )?;
    let mut context = Expressions {
        out,
        method,
        code: prefix,
        names: &names,
        lookup,
        pending_controls: None,
        statement: None,
        check_carrier: None,
    };
    for local in prefix.local_declarations() {
        context.out.indent()?;
        context.out.put("storage_fact ")?;
        crate::p9::expressions::machine_type(context.out, local.original.storage_type())?;
        context.out.put(" ")?;
        context.out.put(&local.name)?;
        context
            .out
            .put("; // original storage, display not published\n")?;
    }
    for bundle in prefix.copies() {
        context.out.indent()?;
        context.out.fmt(format_args!(
            "copy_fact #{} at {:?}: {{",
            bundle.id.0, bundle.location
        ))?;
        context.copy(bundle.id)?;
        context
            .out
            .put("} // source obligation, not a reconstructed schedule\n")?;
    }
    for (op, statement) in prefix.fragments() {
        context.out.indent()?;
        let a = context.out.position();
        context.out.fmt(format_args!("fragment @{} {{ ", op.pc.0))?;
        context.assignment(statement)?;
        context.semantic(statement)?;
        context.out.put("; } // control not published\n")?;
        context.out.span(
            a,
            Origin::Operation {
                method,
                operation: op.source,
                pc: op.pc,
            },
        )?;
    }
    Ok(())
}

fn retained_code(
    out: &mut Buffer,
    method: MethodId,
    stop: &p8::MemberStop,
) -> Result<(), RenderError> {
    match &stop.decoded {
        Ok(items) => {
            for item in items {
                out.indent()?;
                let a = out.position();
                out.fmt(format_args!("decoded @{}: {item:?};\n", item.pc().0))?;
                out.span(
                    a,
                    Origin::Raw {
                        method,
                        pc: Some(item.pc()),
                    },
                )?;
            }
        }
        Err(e) => {
            out.indent()?;
            out.fmt(format_args!("decoded_prefix_unavailable<{e}>;\n"))?;
        }
    }
    match &stop.raw {
        Ok(Some(raw)) => {
            out.indent()?;
            let a = out.position();
            out.fmt(format_args!(
                "raw_code<file_bytes={}..{}; registers={}; incoming={}; outgoing={}> [",
                raw.range.start.0, raw.range.end.0, raw.registers, raw.incoming, raw.outgoing
            ))?;
            for b in &raw.bytes {
                out.fmt(format_args!("{b:02x} "))?;
            }
            out.put("];\n")?;
            out.span(a, Origin::Raw { method, pc: None })?;
        }
        Ok(None) => out.line("raw_code_not_available;")?,
        Err(e) => {
            out.indent()?;
            out.fmt(format_args!("raw_code_retention_failed<{e}>;\n"))?;
        }
    }
    Ok(())
}
