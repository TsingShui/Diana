//! Member rendering and method-local failure containment.

use crate::p9::{
    Origin, RenderError,
    buffer::Buffer,
    expressions::Expressions,
    names::{Lookup, MethodNames},
    values::Attachments,
};
use crate::{MethodId, p7, p8};

use super::declarations::{header_line, original_header};

struct MethodRenderState {
    prints_nothing: bool,
    header_printed: bool,
}

pub(super) fn member(
    out: &mut Buffer,
    lookup: &Lookup<'_>,
    facts: &crate::p0::ClassFacts,
    attachments: Option<&Attachments<'_>>,
    member: &p8::MethodMember,
    class_name: Option<&p8::ValueName>,
    prints_nothing: bool,
) -> Result<(), RenderError> {
    let initial_depth = out.depth;
    let initial_comment = out.abi_comment;
    let initial_origin_only = out.origin_only;
    let initial_method = out.outline_method.replace(member.id);
    let start = out.position();
    let mut state = MethodRenderState {
        prints_nothing,
        header_printed: false,
    };
    let result = method(
        out,
        lookup,
        facts,
        attachments,
        member,
        class_name,
        &mut state,
    );
    // A failed attachment cannot comment out a sibling's recovered code or its
    // method-local recovery diagnostic.
    out.abi_comment = initial_comment;
    out.origin_only = initial_origin_only;
    match result {
        Ok(()) => {}
        Err(RenderError::Invariant(message)) => {
            out.problem(member.id, crate::p9::LocalRenderError::Invariant(message))?;
            while out.depth > initial_depth {
                out.close()?;
            }
            if !state.header_printed
                && let Some(attachments) = attachments
            {
                attachments.print_unrepresented_method_signature(out, lookup, member.id)?;
            }
        }
        Err(RenderError::MethodScratch(error)) => {
            if out.depth == initial_depth {
                original_header(out, facts, member.id)?;
                out.put(";\n")?;
            }
            out.problem(member.id, crate::p9::LocalRenderError::Scratch(error))?;
            while out.depth > initial_depth {
                out.close()?;
            }
            if !state.header_printed
                && let Some(attachments) = attachments
            {
                attachments.print_unrepresented_method_signature(out, lookup, member.id)?;
            }
        }
        Err(e) => return Err(e),
    }
    out.outline_method = initial_method;
    out.span(start, Origin::Method(member.id))
}

fn method(
    out: &mut Buffer,
    lookup: &Lookup<'_>,
    facts: &crate::p0::ClassFacts,
    attachments: Option<&Attachments<'_>>,
    member: &p8::MethodMember,
    class_name: Option<&p8::ValueName>,
    state: &mut MethodRenderState,
) -> Result<(), RenderError> {
    // P8 omits the `static { ... }` block of a class initializer whose printed
    // body states nothing, because Java writes no empty initializer. The member
    // stays in the product; its own span still anchors it here.
    if state.prints_nothing {
        return Ok(());
    }
    // A record of this member prints above the header that names it; that header
    // is the declaration, so no owner prefix is printed with it. A standalone
    // publication has no class header to carry the residual block, which stays
    // in this member's local error scope.
    if let Some(attachments) = attachments {
        if attachments.prints_residual_here() {
            attachments.print_residual(out, lookup)?;
        }
        attachments.print_method(out, lookup, member.id)?;
    }
    match &member.delivery {
        Ok(p8::MethodDelivery::Expressed(result) | p8::MethodDelivery::Declared(result)) => {
            complete_method(
                out,
                lookup,
                attachments,
                member.id,
                result,
                class_name,
                state,
            )?;
        }
        Ok(p8::MethodDelivery::Stopped(stop)) => {
            super::recovery::stopped_method(out, lookup, facts, attachments, member.id, stop)?;
        }
        Err(error) => {
            original_header(out, facts, member.id)?;
            out.open()?;
            out.indent()?;
            out.fmt(format_args!("member_delivery_unavailable<{error}>;\n"))?;
            out.close()?;
        }
    }
    Ok(())
}

fn complete_method(
    out: &mut Buffer,
    lookup: &Lookup<'_>,
    attachments: Option<&Attachments<'_>>,
    method: MethodId,
    result: &p7::MethodResult,
    class_name: Option<&p8::ValueName>,
    state: &mut MethodRenderState,
) -> Result<(), RenderError> {
    let header = result.header();
    if let Some(error) = &header.metadata_error {
        out.indent()?;
        out.fmt(format_args!("method_metadata_unavailable<{error}>;\n"))?;
    }
    let names = MethodNames::collect(
        method,
        result.code().map_or(0, |c| c.types().len()),
        header.declaration.parameter_count(),
        lookup,
    )?;
    let suspending = result
        .code()
        .is_some_and(|code| code.emission().kotlin_continuation_view());
    if suspending {
        out.line("// Diana: ABI-derived suspend body; original entry/reentry and error code retained below")?;
    }
    header_line(
        out,
        lookup,
        &names,
        header,
        (class_name, lookup.find_use(p8::TypeSource::Owner)),
        attachments,
        suspending,
    )?;
    state.header_printed = true;
    if let Some(code) = result.code() {
        let block_start = out.position();
        out.open()?;
        let body_start = out.position();
        let mut pending = Vec::new();
        pending
            .try_reserve_exact(code.controls().len())
            .map_err(RenderError::MethodScratch)?;
        pending.resize(code.controls().len(), None);
        let mut context = Expressions {
            out,
            method,
            code,
            names: &names,
            lookup,
            pending_controls: Some(&mut pending),
            statement: None,
            check_carrier: None,
        };
        context.body()?;
        if out.position() == body_start {
            out.close_empty(block_start)?;
        } else {
            out.close()?;
        }
    } else if matches!(header.kind, p7::MethodKind::ClassInitializer) {
        out.put(" { }\n")?;
    } else {
        out.put(";\n")?;
    }
    Ok(())
}
