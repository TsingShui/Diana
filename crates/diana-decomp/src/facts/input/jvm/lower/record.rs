//! Evidence for the standard Java record bootstrap. Its component access and
//! original operation are kept independently of any JDK implementation recipe.

use std::collections::HashMap;

use super::{class_type, text, OpError};
use crate::input::jvm::bytecode::method_handle;
use crate::input::jvm::{BootstrapMethod, ClassFile, Constant, JvmString, ReferenceKind};
use crate::ir::{ArgType, FieldReference, MethodDescriptor, RecordOperation, RecordSite};

/// A record declaration and the bootstrap's actual component arguments must
/// agree. An arbitrary use of the same library bootstrap remains an unknown
/// dynamic operation when its getter shape is outside this representation.
pub(super) fn site(
    class: &ClassFile<'_>,
    row: BootstrapMethod<'_>,
    name: JvmString<'_>,
    descriptor: &MethodDescriptor,
) -> Result<Option<RecordSite>, OpError> {
    let owner_name = class.name()?;
    if class.access_flags() & 0x0010 == 0
        || !class
            .super_name()?
            .is_some_and(|name| name.bytes() == b"java/lang/Record")
    {
        return Ok(None);
    }
    let owner = class_type(owner_name)?;
    let operation = match (
        name.bytes(),
        descriptor.parameters.as_slice(),
        &descriptor.return_type,
    ) {
        (b"equals", [first, other], &ArgType::BOOLEAN)
            if first == &owner && other == &ArgType::object("java/lang/Object") =>
        {
            RecordOperation::Equals
        }
        (b"hashCode", [first], &ArgType::INT) if first == &owner => RecordOperation::HashCode,
        (b"toString", [first], result) if first == &owner && result == &ArgType::string() => {
            RecordOperation::ToString
        }
        _ => return Ok(None),
    };
    let mut arguments = row.arguments();
    let (Some(class_name), Some(names)) = (arguments.next(), arguments.next()) else {
        return Ok(None);
    };
    if class.class_name(class_name)? != owner_name {
        return Ok(None);
    }
    let names = class.string_constant(names)?;
    let mut names = names
        .bytes()
        .split(|&byte| byte == b';')
        .filter(|_| !names.bytes().is_empty());
    let Some(declared) = class.record_components()? else {
        return Ok(None);
    };
    let count = usize::from(declared.declared_count());
    if arguments.len() != count {
        return Ok(None);
    }

    let mut fields = HashMap::new();
    fields.try_reserve(count).map_err(|_| OpError::Allocation)?;
    let mut components = Vec::new();
    components
        .try_reserve_exact(count)
        .map_err(|_| OpError::Allocation)?;
    let mut declared = declared;
    for argument in arguments {
        let Some(component) = declared.next().transpose()? else {
            return Ok(None);
        };
        if names.next() != Some(component.name.bytes()) {
            return Ok(None);
        }

        let Constant::MethodHandle {
            kind: 1,
            reference_index,
        } = class.constant(argument)?
        else {
            return Ok(None);
        };
        let getter = method_handle(class, 1, reference_index)?;
        if getter.kind != ReferenceKind::Field
            || getter.owner != owner_name
            || getter.name != component.name
            || getter.descriptor != component.descriptor
            || fields
                .insert((getter.name.bytes(), getter.descriptor.bytes()), false)
                .is_some()
        {
            return Ok(None);
        }
        components.push(FieldReference {
            owner: owner.clone(),
            name: text(getter.name)?.into_owned(),
            field_type: text(getter.descriptor)?.parse()?,
        });
    }
    if declared.next().transpose()?.is_some() || names.next().is_some() {
        return Ok(None);
    }

    // A pool Fieldref alone does not supply instance/static or ownership
    // facts. The actual stored class establishes each component read.
    for field in class.fields()? {
        let field = field?;
        let key = (field.name(class)?.bytes(), field.descriptor(class)?.bytes());
        if let Some(seen) = fields.get_mut(&key) {
            const STATIC_PRIVATE_FINAL: u16 = 0x0008 | 0x0002 | 0x0010;
            if *seen || field.access_flags() & STATIC_PRIVATE_FINAL != 0x0012 {
                return Ok(None);
            }
            *seen = true;
        }
    }
    if !fields.values().all(|&seen| seen) {
        return Ok(None);
    }
    Ok(Some(RecordSite {
        operation,
        owner,
        components,
    }))
}
