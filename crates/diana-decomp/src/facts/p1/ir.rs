//! P1's directly consumable recovery IR.
//!
//! P1 already owns the checked instruction directory, reference facts, invoke
//! layouts and protection boundaries. This module turns those facts into the
//! instruction list and exception handlers the recovery backend consumes, so
//! the engine performs no second decode or symbol mapping.

use crate::input::operands::RawRegisters;
use crate::ir::{ArgType, ExceptionHandler, RegNum, RegisterArg};
use crate::ir::{
    FieldReference, InsnArg, InsnNode, InsnType, InvokeType, LambdaSite, MemberReference,
    MethodDescriptor, MethodReference,
};
use crate::p1::{
    BootstrapDeclaration, CheckedBootstrapFact, CheckedMemberFact, CheckedPrototype,
    CheckedReferenceFact, DecodedMethod, InvokeLayout, MethodError,
};
use crate::raw::ReferenceSlot;
use crate::{CodeUnitOffset, TypeId};

use super::MethodIr;

mod decode;

/// Build the backend-ready instruction list and handlers from the published P1
/// facts. Failures are retained as an explicit product gap instead of widening
/// the method result.
pub(super) fn build(facts: &DecodedMethod<'_>) -> Result<MethodIr, String> {
    let mut instructions = Vec::new();
    instructions
        .try_reserve_exact(facts.instructions().count())
        .map_err(|_| "IR instruction allocation failed".to_owned())?;
    let mut references = facts.reference_facts().peekable();
    let mut layouts = facts.layout().invoke_layouts().peekable();
    for item in facts.instructions() {
        let pc = item.pc();
        // Both checked directories are in original instruction and slot order.
        // Reading a directory entry need not require its fact (for example a
        // const-method-handle currently contributes only an instruction boundary).
        let first = references
            .next_if(|(site, slot, _)| *site == pc && *slot == ReferenceSlot::First)
            .map(|(_, _, fact)| fact);
        let second = references
            .next_if(|(site, slot, _)| *site == pc && *slot == ReferenceSlot::Second)
            .map(|(_, _, fact)| fact);
        let layout = layouts
            .next_if(|(site, _)| *site == pc)
            .and_then(|(_, state)| state.ok());
        if let Some(mut instruction) = decode::decode(facts, item) {
            instruction.set_offset(pc.0);
            bind_instruction(
                &mut instruction,
                item.operands().registers(),
                pc,
                first,
                second,
                layout,
            )?;
            instructions.push(instruction);
        }
    }
    Ok(MethodIr::new(
        instructions,
        handlers(facts),
        u32::from(facts.source().registers_size()),
        u32::from(facts.source().ins_size()),
    ))
}

type ReferenceRead<'a, 'image> = Result<&'a CheckedReferenceFact<'image>, &'a MethodError>;

fn required_reference<'a, 'image>(
    read: Option<ReferenceRead<'a, 'image>>,
    pc: CodeUnitOffset,
) -> Result<&'a CheckedReferenceFact<'image>, String> {
    read.ok_or_else(|| format!("missing reference fact at {pc:?}"))?
        .map_err(|error| error.to_string())
}

fn bind_instruction(
    insn: &mut InsnNode,
    registers: &RawRegisters,
    pc: CodeUnitOffset,
    first: Option<ReferenceRead<'_, '_>>,
    second: Option<ReferenceRead<'_, '_>>,
    layout: Option<&InvokeLayout>,
) -> Result<(), String> {
    if insn.insn_type == InsnType::Invoke {
        match insn.payload.invoke_type {
            Some(InvokeType::Polymorphic) => {
                let fact = required_reference(first, pc)?;
                let CheckedReferenceFact::Method { owner, name, .. } = fact else {
                    return Err(format!("polymorphic invoke at {pc:?} has another kind"));
                };
                let proto = required_reference(second, pc)?;
                let CheckedReferenceFact::Proto { prototype } = proto else {
                    return Err(format!(
                        "polymorphic invoke at {pc:?} has no checked prototype"
                    ));
                };
                let method = MethodReference {
                    owner: descriptor(*owner)?,
                    name: text(*name)?,
                    descriptor: prototype_descriptor(prototype)?,
                };
                apply_invoke_return_type(insn, &method);
                bind_invoke_args(insn, registers, layout, pc, &method.descriptor)?;
                insn.payload.reference = Some(std::sync::Arc::new(MemberReference::Method(method)));
                return Ok(());
            }
            Some(InvokeType::Custom) => {
                let fact = required_reference(first, pc)?;
                let CheckedReferenceFact::CallSite {
                    name: sam_name,
                    prototype,
                    handle,
                    declarations,
                    ..
                } = fact
                else {
                    return Err(format!("custom invoke at {pc:?} has another kind"));
                };
                let mut bootstrap = None;
                for declaration in declarations {
                    if declaration.reference != handle.member() {
                        continue;
                    }
                    if let CheckedBootstrapFact::Member(CheckedMemberFact::Method {
                        owner,
                        name,
                        ..
                    }) = &declaration.fact
                    {
                        bootstrap = Some((*owner, *name));
                    }
                    break;
                }
                let Some((owner, name)) = bootstrap else {
                    return Err(format!(
                        "custom invoke at {pc:?} has no bootstrap method declaration"
                    ));
                };
                let bootstrap_owner = text(owner)?;
                let bootstrap_name = text(name)?;
                let bootstrap_owner_name = bootstrap_owner
                    .strip_prefix('L')
                    .and_then(|owner| owner.strip_suffix(';'))
                    .unwrap_or(&bootstrap_owner);
                if bootstrap_owner_name == "java/lang/invoke/LambdaMetafactory"
                    && matches!(bootstrap_name.as_str(), "metafactory" | "altMetafactory")
                {
                    insn.payload.lambda =
                        lambda_site(*sam_name, prototype, declarations)?.map(Box::new);
                }
                let method = MethodReference {
                    owner: descriptor(owner)?,
                    name: bootstrap_name,
                    descriptor: prototype_descriptor(prototype)?,
                };
                apply_invoke_return_type(insn, &method);
                bind_invoke_args(insn, registers, layout, pc, &method.descriptor)?;
                insn.payload.reference = Some(std::sync::Arc::new(MemberReference::Method(method)));
                return Ok(());
            }
            _ => {}
        }
    }
    if insn.payload.method_index.is_some() {
        let fact = required_reference(first, pc)?;
        let CheckedReferenceFact::Method {
            owner,
            name,
            prototype,
            ..
        } = fact
        else {
            return Err(format!("method reference at {pc:?} has another kind"));
        };
        let method = MethodReference {
            owner: descriptor(*owner)?,
            name: text(*name)?,
            descriptor: prototype_descriptor(prototype)?,
        };
        apply_invoke_return_type(insn, &method);
        bind_invoke_args(insn, registers, layout, pc, &method.descriptor)?;
        insn.payload.reference = Some(std::sync::Arc::new(MemberReference::Method(method)));
    }
    if insn.payload.field_index.is_some() {
        let fact = required_reference(first, pc)?;
        let CheckedReferenceFact::Field {
            owner,
            name,
            field_type,
            ..
        } = fact
        else {
            return Err(format!("field reference at {pc:?} has another kind"));
        };
        let field = FieldReference {
            owner: descriptor(*owner)?,
            name: text(*name)?,
            field_type: descriptor(*field_type)?,
        };
        insn.payload.reference = Some(std::sync::Arc::new(MemberReference::Field(field)));
    }
    if insn.payload.string_index.is_some() && insn.payload.string_value.is_none() {
        let fact = required_reference(first, pc)?;
        let CheckedReferenceFact::String { value } = fact else {
            return Err(format!("string reference at {pc:?} has another kind"));
        };
        let units = value.units().map_err(|error| error.to_string())?;
        insn.payload.string_value = Some(Box::new(crate::ir::Utf16String::from_utf16(units)));
    }
    if insn.payload.type_index.is_some() && insn.payload.class_type.is_none() {
        let fact = required_reference(first, pc)?;
        let CheckedReferenceFact::Type { descriptor, .. } = fact else {
            return Err(format!("type reference at {pc:?} has another kind"));
        };
        insn.payload.class_type = Some(Box::new(self::descriptor(*descriptor)?));
    }
    Ok(())
}

/// Recover the `LambdaMetafactory` bootstrap arguments that describe a SAM
/// implementation. The first prototype is the SAM signature and the first
/// method handle is the implementation.
fn lambda_site<'a>(
    sam_name: crate::input::DexString<'a>,
    interface: &CheckedPrototype<'a>,
    declarations: &[BootstrapDeclaration<'a>],
) -> Result<Option<LambdaSite>, String> {
    let mut sam = None;
    let mut implementation = None;
    let mut prototypes = 0usize;
    for declaration in declarations {
        match &declaration.fact {
            CheckedBootstrapFact::Proto { prototype } => {
                if prototypes == 0 {
                    sam = Some(prototype);
                }
                prototypes += 1;
            }
            CheckedBootstrapFact::MethodHandle {
                handle,
                member:
                    CheckedMemberFact::Method {
                        owner,
                        name,
                        prototype,
                        ..
                    },
            } if implementation.is_none() => {
                implementation = Some((handle, *owner, *name, prototype));
            }
            _ => {}
        }
    }
    let (Some(sam), Some((handle, owner, name, implementation_prototype))) = (sam, implementation)
    else {
        return Ok(None);
    };
    let interface = descriptor(interface.return_type())?;
    Ok(Some(LambdaSite {
        interface,
        sam_name: text(sam_name)?,
        sam_descriptor: prototype_descriptor(sam)?,
        implementation: MethodReference {
            owner: descriptor(owner)?,
            name: text(name)?,
            descriptor: prototype_descriptor(implementation_prototype)?,
        },
        // DEX method-handle kinds: 0x04 invoke-static, 0x05 invoke-instance,
        // 0x06 invoke-constructor, 0x07 invoke-direct, 0x08 invoke-interface.
        implementation_static: handle.kind() == 4,
        implementation_constructor: handle.kind() == 6,
    }))
}

/// Bind the first word of each declared parameter. When P1 could not
/// establish the layout, preserve every original encoded operand.
fn bind_invoke_args(
    insn: &mut InsnNode,
    registers: &RawRegisters,
    layout: Option<&InvokeLayout>,
    pc: CodeUnitOffset,
    descriptor: &MethodDescriptor,
) -> Result<(), String> {
    let Some(layout) = layout else {
        insn.args = decode::raw_arguments(registers).collect();
        return Ok(());
    };
    let parameters = layout.arguments().iter().zip(&descriptor.parameters);
    insn.args
        .reserve_exact(usize::from(layout.has_receiver()) + parameters.len());
    let mut words = registers.iter();
    if layout.has_receiver() {
        let receiver = words
            .next()
            .ok_or_else(|| format!("invoke at {pc:?} lost its receiver"))?;
        insn.add_arg(InsnArg::Reg(RegisterArg::new(
            receiver.0 as RegNum,
            ArgType::unknown(),
        )));
    }
    for (width, parameter) in parameters {
        let register = words
            .next()
            .ok_or_else(|| format!("invoke at {pc:?} lost a declared argument"))?;
        insn.add_arg(InsnArg::Reg(RegisterArg::new(
            register.0 as RegNum,
            parameter.clone(),
        )));
        if width.words() == 2 {
            words.next();
        }
    }
    Ok(())
}

fn text(value: crate::input::DexString<'_>) -> Result<String, String> {
    value
        .utf8_lossy()
        .map(|text| text.into_owned())
        .map_err(|error| error.to_string())
}

fn descriptor(value: crate::input::DexString<'_>) -> Result<crate::ir::ArgType, String> {
    value
        .utf8_lossy()
        .map_err(|error| error.to_string())?
        .parse()
        .map_err(|error: crate::ir::DescriptorParseError| error.to_string())
}

fn prototype_descriptor(prototype: &CheckedPrototype<'_>) -> Result<MethodDescriptor, String> {
    let mut parameters = Vec::new();
    parameters
        .try_reserve_exact(prototype.parameters().len())
        .map_err(|_| "prototype parameter allocation failed".to_owned())?;
    for parameter in prototype.parameters() {
        parameters.push(descriptor(parameter.descriptor())?);
    }
    Ok(MethodDescriptor {
        parameters,
        return_type: descriptor(prototype.return_type())?,
    })
}

fn apply_invoke_return_type(insn: &mut InsnNode, method: &MethodReference) {
    if insn.insn_type != InsnType::Invoke {
        return;
    }
    if method.descriptor.return_type == crate::ir::ArgType::VOID {
        return;
    }
    if let Some(result) = insn.result.as_mut() {
        result.ty = method.descriptor.return_type.clone();
    }
}

fn handlers(facts: &DecodedMethod<'_>) -> Vec<ExceptionHandler> {
    let mut boundaries = facts.try_boundaries();
    let mut handlers = Vec::new();
    while let (Some(start), Some(end)) = (boundaries.next(), boundaries.next()) {
        let clauses = facts
            .catch_clauses_at(CodeUnitOffset(start))
            .unwrap_or_default();
        for (exception, target) in clauses {
            let catch_type = match exception {
                Some(id) => facts
                    .catch_type(TypeId(id))
                    .and_then(|fact| fact.descriptor().ok())
                    .and_then(|text| text.utf8_lossy().ok())
                    .and_then(|text| text.parse::<crate::ir::ArgType>().ok()),
                None => None,
            };
            handlers.push(ExceptionHandler::new(start, end, target, catch_type));
        }
    }
    handlers
}
