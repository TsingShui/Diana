//! Standard JVM bootstrap shapes with a direct representation in shared IR.

use std::fmt::{self, Write};
use std::ops::Range;
use std::sync::Arc;

use super::{class_type, text, Lower, OpError, Read};
use crate::input::jvm::bytecode::{method_handle, Instruction};
use crate::input::jvm::{
    BootstrapMethod, ClassFile, Constant, JvmString, MemberReference as JvmMember, ReferenceKind,
};
use crate::ir::insn::LambdaForwarding;
use crate::ir::{
    ArgType, InsnArg, InsnNode, InsnType, InvokeType, LambdaSite, MemberReference,
    MethodDescriptor, MethodReference, Utf16String,
};
use crate::FailureKind;

#[path = "record.rs"]
mod record;

const OBJECT_METHODS: &[u8] = b"(Ljava/lang/invoke/MethodHandles$Lookup;Ljava/lang/String;Ljava/lang/invoke/TypeDescriptor;Ljava/lang/Class;Ljava/lang/String;[Ljava/lang/invoke/MethodHandle;)Ljava/lang/Object;";
const CONCAT: &[u8] = b"(Ljava/lang/invoke/MethodHandles$Lookup;Ljava/lang/String;Ljava/lang/invoke/MethodType;)Ljava/lang/invoke/CallSite;";
const CONCAT_WITH_CONSTANTS: &[u8] = b"(Ljava/lang/invoke/MethodHandles$Lookup;Ljava/lang/String;Ljava/lang/invoke/MethodType;Ljava/lang/String;[Ljava/lang/Object;)Ljava/lang/invoke/CallSite;";
const METAFACTORY: &[u8] = b"(Ljava/lang/invoke/MethodHandles$Lookup;Ljava/lang/String;Ljava/lang/invoke/MethodType;Ljava/lang/invoke/MethodType;Ljava/lang/invoke/MethodHandle;Ljava/lang/invoke/MethodType;)Ljava/lang/invoke/CallSite;";

pub(super) struct DynamicSite<'a> {
    pub bootstrap: u16,
    pub name: JvmString<'a>,
    pub descriptor: JvmString<'a>,
}

pub(super) fn dynamic(
    lower: &mut Lower<'_>,
    class: &ClassFile<'_>,
    instruction: Instruction<'_>,
    site: DynamicSite<'_>,
    descriptor: MethodDescriptor,
    args: &[Read],
) -> Result<(), OpError> {
    let row = match bootstrap_row(class, site.bootstrap) {
        Ok(row) => row,
        Err(error) => {
            return lower.effect(
                instruction,
                descriptor.return_type,
                args,
                error.kind(),
                format_args!("{site}: {error}"),
            );
        }
    };
    let (row, kind, member) = row;
    if kind == 6 {
        match (
            member.owner.bytes(),
            member.name.bytes(),
            member.descriptor.bytes(),
        ) {
            (b"java/lang/invoke/StringConcatFactory", b"makeConcat", CONCAT)
            | (
                b"java/lang/invoke/StringConcatFactory",
                b"makeConcatWithConstants",
                CONCAT_WITH_CONSTANTS,
            ) => {
                let with_constants = member.descriptor.bytes() == CONCAT_WITH_CONSTANTS;
                let plan = ConcatPlan::decode(class, row, &descriptor, args, with_constants);
                match plan {
                    Ok(Some(plan)) => {
                        let materialized = if lower.output.is_some() {
                            Some(plan.operands(args, instruction.offset())?)
                        } else {
                            None
                        };
                        return lower.expr(
                            InsnType::StringConcat,
                            descriptor.return_type,
                            args,
                            |node| {
                                if let Some(operands) = materialized {
                                    node.args = operands;
                                }
                            },
                        );
                    }
                    Ok(None) => {}
                    Err(error) => {
                        return lower.effect(
                            instruction,
                            descriptor.return_type,
                            args,
                            error.kind(),
                            format_args!("{site} with {}: {error}", OriginalMember(member)),
                        );
                    }
                }
            }
            (b"java/lang/runtime/ObjectMethods", b"bootstrap", OBJECT_METHODS)
                if member.kind == ReferenceKind::Method =>
            {
                match record::site(class, row, site.name, &descriptor) {
                    Ok(Some(record)) => {
                        return lower.expr(
                            InsnType::RecordOperation,
                            descriptor.return_type,
                            args,
                            |node| {
                                node.payload.record_operation = Some(Arc::new(record));
                            },
                        );
                    }
                    Ok(None) => {}
                    Err(error) => {
                        return lower.effect(
                            instruction,
                            descriptor.return_type,
                            args,
                            error.kind(),
                            format_args!("{site} with {}: {error}", OriginalMember(member)),
                        );
                    }
                }
            }
            (b"java/lang/invoke/LambdaMetafactory", b"metafactory", METAFACTORY) => {
                match method_reference(class, row, &descriptor, site.name, args) {
                    Ok(Some(lambda)) => {
                        let result = descriptor.return_type.clone();
                        return lower.expr(InsnType::Invoke, result, args, |node| {
                            node.payload.invoke_type = Some(InvokeType::Custom);
                            node.payload.reference =
                                Some(Arc::new(MemberReference::Method(MethodReference {
                                    owner: ArgType::object("java/lang/invoke/LambdaMetafactory"),
                                    name: String::from("metafactory"),
                                    descriptor,
                                })));
                            node.payload.lambda = Some(Box::new(lambda));
                        });
                    }
                    Ok(None) => {}
                    Err(error) => {
                        return lower.effect(
                            instruction,
                            descriptor.return_type,
                            args,
                            error.kind(),
                            format_args!("{site} with {}: {error}", OriginalMember(member)),
                        );
                    }
                }
            }
            _ => {}
        }
    }
    lower.effect(
        instruction,
        descriptor.return_type,
        args,
        FailureKind::Unsupported,
        format_args!("{site} with {}", OriginalMember(member)),
    )
}

fn bootstrap_row<'a>(
    class: &ClassFile<'a>,
    index: u16,
) -> Result<(BootstrapMethod<'a>, u8, JvmMember<'a>), OpError> {
    let row = class.bootstrap_method(index)?;
    let Constant::MethodHandle {
        kind,
        reference_index,
    } = class.constant(row.method_ref)?
    else {
        return Err(OpError::Reference("bootstrap method handle"));
    };
    Ok((row, kind, method_handle(class, kind, reference_index)?))
}

enum Term<'a> {
    Argument(usize),
    Recipe(Range<usize>),
    Constant(JvmString<'a>),
}

struct ConcatPlan<'a> {
    recipe: Vec<u16>,
    terms: Vec<Term<'a>>,
}

impl<'a> ConcatPlan<'a> {
    fn decode(
        class: &ClassFile<'a>,
        row: BootstrapMethod<'a>,
        descriptor: &MethodDescriptor,
        args: &[Read],
        with_constants: bool,
    ) -> Result<Option<Self>, OpError> {
        // String conversion of an arbitrary object can execute user code.
        // Exact String and primitive conversions have no such user calls.
        if descriptor.return_type != ArgType::string()
            || args.len() != descriptor.parameters.len()
            || !args.iter().any(|arg| arg.value.runtime)
            || !args
                .iter()
                .zip(&descriptor.parameters)
                .all(|(arg, parameter)| {
                    arg.value.ty == *parameter
                        && (parameter.is_primitive() || *parameter == ArgType::string())
                })
            || args
                .iter()
                .map(|arg| usize::from(arg.value.category.words()))
                .sum::<usize>()
                > 200
        {
            return Ok(None);
        }

        let mut constants = row.arguments();
        if !with_constants {
            if constants.len() != 0 || args.len() < 2 {
                return Ok(None);
            }
            let mut terms = Vec::new();
            terms
                .try_reserve_exact(args.len())
                .map_err(|_| OpError::Allocation)?;
            terms.extend((0..args.len()).map(Term::Argument));
            return Ok(Some(Self {
                recipe: Vec::new(),
                terms,
            }));
        }
        let Some(recipe) = constants.next() else {
            return Ok(None);
        };
        let Some(recipe) = string_constant(class, recipe)? else {
            return Ok(None);
        };
        let recipe = recipe.to_utf16()?;
        let mut terms = Vec::new();
        let mut after_last = 0;
        let mut arguments = 0;
        let mut expanded = recipe.len();
        for (index, &unit) in recipe.iter().enumerate() {
            if !matches!(unit, 1 | 2) {
                continue;
            }
            if after_last < index {
                push_term(&mut terms, Term::Recipe(after_last..index))?;
            }
            after_last = index + 1;
            let term = if unit == 1 {
                if arguments >= args.len() {
                    return Ok(None);
                }
                arguments += 1;
                Term::Argument(arguments - 1)
            } else {
                let Some(constant) = constants.next() else {
                    return Ok(None);
                };
                // Calling toString during bootstrap or replaying a differently
                // timed conversion would change the original operation.
                let Some(value) = string_constant(class, constant)? else {
                    return Ok(None);
                };
                expanded = expanded
                    .checked_add(value.bytes().len())
                    .ok_or(OpError::Allocation)?;
                // Keep one operation's expansion bounded even if the same
                // large pool entry is referenced by many recipe tags.
                if expanded > usize::from(u16::MAX) {
                    return Err(OpError::Allocation);
                }
                Term::Constant(value)
            };
            push_term(&mut terms, term)?;
        }
        if after_last < recipe.len() {
            push_term(&mut terms, Term::Recipe(after_last..recipe.len()))?;
        }
        if terms.len() < 2 || arguments != args.len() || constants.len() != 0 {
            return Ok(None);
        }
        // Retain recipe order, combining neighboring constant pieces before
        // constructing the Java expression. Dynamic slots already obey the
        // factory's 200-slot linkage limit.
        Ok(Some(Self { recipe, terms }))
    }

    fn operands(self, args: &[Read], pc: u16) -> Result<Vec<InsnArg>, OpError> {
        let mut output = Vec::new();
        let mut literal = Vec::new();
        output
            .try_reserve_exact(args.len() * 2 + 1)
            .map_err(|_| OpError::Allocation)?;
        for term in self.terms {
            match term {
                Term::Argument(index) => {
                    if !literal.is_empty() || output.is_empty() {
                        output.push(literal_operand(std::mem::take(&mut literal), pc));
                    }
                    output.push(args[index].arg());
                }
                Term::Recipe(range) => {
                    literal
                        .try_reserve(range.len())
                        .map_err(|_| OpError::Allocation)?;
                    literal.extend_from_slice(&self.recipe[range]);
                }
                Term::Constant(value) => {
                    literal
                        .try_reserve(value.bytes().len())
                        .map_err(|_| OpError::Allocation)?;
                    literal.extend(value.units());
                }
            }
        }
        if !literal.is_empty() {
            output.push(literal_operand(literal, pc));
        }
        Ok(output)
    }
}

fn push_term<'a>(terms: &mut Vec<Term<'a>>, term: Term<'a>) -> Result<(), OpError> {
    terms.try_reserve(1).map_err(|_| OpError::Allocation)?;
    terms.push(term);
    Ok(())
}

fn literal_operand(units: Vec<u16>, pc: u16) -> InsnArg {
    let mut literal = InsnNode::new(InsnType::ConstStr, 0);
    literal.offset = u32::from(pc);
    literal.payload.string_value = Some(Box::new(Utf16String::from_utf16(units)));
    InsnArg::wrap(literal)
}

fn string_constant<'a>(
    class: &ClassFile<'a>,
    index: u16,
) -> Result<Option<JvmString<'a>>, OpError> {
    match class.constant(index)? {
        Constant::String { .. } => Ok(Some(class.string_constant(index)?)),
        _ => Ok(None),
    }
}

fn method_reference(
    class: &ClassFile<'_>,
    row: BootstrapMethod<'_>,
    factory: &MethodDescriptor,
    sam_name: JvmString<'_>,
    captures: &[Read],
) -> Result<Option<LambdaSite>, OpError> {
    let mut arguments = row.arguments();
    if arguments.len() != 3
        || !factory.return_type.is_object()
        || sam_name.bytes().starts_with(b"<")
    {
        return Ok(None);
    }
    let (Some(sam), Some(target), Some(instantiated)) =
        (arguments.next(), arguments.next(), arguments.next())
    else {
        return Ok(None);
    };
    let (Some(sam), Some(instantiated)) =
        (method_type(class, sam)?, method_type(class, instantiated)?)
    else {
        return Ok(None);
    };
    let Constant::MethodHandle {
        kind,
        reference_index,
    } = class.constant(target)?
    else {
        return Ok(None);
    };
    if !matches!(kind, 5 | 6 | 9) {
        return Ok(None);
    }
    let implementation = method_handle(class, kind, reference_index)?;
    let own_type = class.name()?;
    let is_interface = class.access_flags() & 0x0200 != 0;
    let synthetic_target = implementation.name.bytes().starts_with(b"lambda$");
    if own_type != implementation.owner
        || (implementation.kind == ReferenceKind::InterfaceMethod) != is_interface
        || own_type.bytes().contains(&b'$')
        || (synthetic_target && kind != 6)
    {
        return Ok(None);
    }

    let static_method = kind == 6;
    let owner = class_type(implementation.owner)?;
    let sam_descriptor: MethodDescriptor = text(sam)?.parse()?;
    let adapted = if sam != instantiated {
        if !synthetic_target {
            return Ok(None);
        }
        let descriptor = text(instantiated)?.parse::<MethodDescriptor>()?;
        if !exact_reference_adaptation(&sam_descriptor, &descriptor) {
            return Ok(None);
        }
        Some(descriptor)
    } else {
        None
    };
    let invocation = adapted.as_ref().unwrap_or(&sam_descriptor);
    let implementation_descriptor: MethodDescriptor = text(implementation.descriptor)?.parse()?;
    let forwarding = match (kind, factory.parameters.as_slice(), captures) {
        (6, [], []) if adapted.is_none() && sam_descriptor == implementation_descriptor => None,
        (6, factory_parameters, captures)
            if synthetic_target
                && implementation_descriptor.return_type == invocation.return_type
                && implementation_descriptor.parameters.len()
                    == factory_parameters.len() + invocation.parameters.len()
                && implementation_descriptor
                    .parameters
                    .starts_with(factory_parameters)
                && implementation_descriptor.parameters[factory_parameters.len()..]
                    == invocation.parameters =>
        {
            let mut parameters = Vec::new();
            if captures.len() != factory_parameters.len() {
                return Ok(None);
            }
            parameters
                .try_reserve_exact(captures.len())
                .map_err(|_| OpError::Allocation)?;
            for (capture, ty) in captures.iter().zip(factory_parameters) {
                let Some(index) = capture
                    .value
                    .input_parameter
                    .filter(|_| &capture.value.ty == ty)
                else {
                    return Ok(None);
                };
                parameters.push(index);
            }
            let mut argument_casts = Vec::new();
            if let Some(adapted) = adapted {
                argument_casts
                    .try_reserve_exact(adapted.parameters.len())
                    .map_err(|_| OpError::Allocation)?;
                argument_casts.extend(
                    sam_descriptor
                        .parameters
                        .iter()
                        .zip(adapted.parameters)
                        .map(|(erased, target)| (erased != &target).then_some(target)),
                );
            }
            Some(LambdaForwarding {
                captured_parameters: parameters.into_boxed_slice(),
                argument_casts: argument_casts.into_boxed_slice(),
            })
        }
        // A bound Java method reference reads its receiver and null-checks
        // it at creation, exactly where the original linkage captures it.
        (5 | 9, [declared_receiver], [receiver])
            if declared_receiver == &owner
                && receiver.value.ty == owner
                && sam_descriptor == implementation_descriptor =>
        {
            None
        }
        // In the unbound form, the SAM's first argument is the receiver
        // supplied at invocation. With no unknown base declarations, a
        // competing inherited static overload cannot change that binding.
        (5 | 9, [], [])
            if sam_descriptor.return_type == implementation_descriptor.return_type
                && sam_descriptor
                    .parameters
                    .split_first()
                    .is_some_and(|(receiver, tail)| {
                        receiver == &owner && tail == implementation_descriptor.parameters
                    })
                && class
                    .super_name()?
                    .is_some_and(|parent| parent.bytes() == b"java/lang/Object")
                && class.interfaces().next().is_none() =>
        {
            None
        }
        _ => return Ok(None),
    };

    // A single original declaration in this class establishes the target
    // without substituting a guessed overload or dispatch mode.
    let mut declaration_flags = None;
    for method in class.methods()? {
        let method = method?;
        if method.name(class)? == implementation.name {
            if declaration_flags.is_some()
                || (method.access_flags() & 0x0008 != 0) != static_method
                || method.descriptor(class)? != implementation.descriptor
            {
                return Ok(None);
            }
            declaration_flags = Some(method.access_flags());
        }
    }
    let Some(declaration_flags) = declaration_flags else {
        return Ok(None);
    };
    if synthetic_target && declaration_flags & 0x1000 == 0 {
        return Ok(None);
    }
    Ok(Some(LambdaSite {
        interface: factory.return_type.clone(),
        sam_name: text(sam_name)?.into_owned(),
        sam_descriptor,
        implementation: MethodReference {
            owner,
            name: text(implementation.name)?.into_owned(),
            descriptor: implementation_descriptor,
        },
        implementation_static: static_method,
        implementation_constructor: false,
        checked_synthetic_target: synthetic_target,
        forwarding,
    }))
}

/// The instantiated method type proves casts from Object at invocation. A
/// reference returned by the exact target always widens to Object, without
/// needing any external type declarations.
fn exact_reference_adaptation(sam: &MethodDescriptor, instantiated: &MethodDescriptor) -> bool {
    let compatible = |erased: &ArgType, actual: &ArgType| {
        erased == actual
            || (erased.as_object() == Some("java/lang/Object") && actual.is_reference())
    };
    sam.parameters.len() == instantiated.parameters.len()
        && sam
            .parameters
            .iter()
            .zip(&instantiated.parameters)
            .all(|(erased, actual)| compatible(erased, actual))
        && compatible(&sam.return_type, &instantiated.return_type)
}

fn method_type<'a>(class: &ClassFile<'a>, index: u16) -> Result<Option<JvmString<'a>>, OpError> {
    let Constant::MethodType { descriptor_index } = class.constant(index)? else {
        return Ok(None);
    };
    Ok(Some(class.utf8(descriptor_index)?.descriptor(true)?))
}

impl fmt::Display for DynamicSite<'_> {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            output,
            "{}{} using bootstrap #{}",
            OriginalText(self.name),
            OriginalText(self.descriptor),
            self.bootstrap,
        )
    }
}

pub(super) struct OriginalText<'a>(pub JvmString<'a>);

impl fmt::Display for OriginalText<'_> {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        for scalar in char::decode_utf16(self.0.units()) {
            match scalar {
                Ok(ch) => {
                    for escaped in ch.escape_debug() {
                        output.write_char(escaped)?;
                    }
                }
                Err(error) => write!(output, "\\u{:04x}", error.unpaired_surrogate())?,
            }
        }
        Ok(())
    }
}

pub(super) struct OriginalMember<'a>(pub JvmMember<'a>);

impl fmt::Display for OriginalMember<'_> {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        let member = self.0;
        if member.owner.bytes().starts_with(b"[") {
            write!(output, "{}", OriginalText(member.owner))?;
        } else {
            write!(output, "L{};", OriginalText(member.owner))?;
        }
        let separator = if member.kind == ReferenceKind::Field {
            ":"
        } else {
            ""
        };
        write!(
            output,
            "->{}{separator}{}",
            OriginalText(member.name),
            OriginalText(member.descriptor)
        )
    }
}
