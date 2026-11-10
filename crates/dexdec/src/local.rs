//! Symbol sources for IR reference resolution.
//!
//! The IR decoder leaves pool indices in each instruction. Resolution turns
//! those indices into method, field, string and type identities. A whole-DEX
//! reader is one source; a caller that already decoded the method can supply the
//! same identities directly, so the recovery pipeline does not have to re-parse
//! the image for a local request.

use std::collections::HashMap;

/// Resolved pool entries the IR needs while its instructions are still indexed.
pub trait SymbolSource {
    fn method(&self, index: u32) -> Option<&str>;
    fn field(&self, index: u32) -> Option<&str>;
    fn ty(&self, index: u32) -> Option<&str>;
    fn string(&self, index: u32) -> Option<&[u16]>;
}

/// Identities a caller already resolved for one method's pool indices.
#[derive(Default)]
pub struct LocalSymbols {
    pub methods: HashMap<u32, String>,
    pub fields: HashMap<u32, String>,
    pub types: HashMap<u32, String>,
    pub strings: HashMap<u32, Vec<u16>>,
}

impl SymbolSource for LocalSymbols {
    fn method(&self, index: u32) -> Option<&str> {
        self.methods.get(&index).map(String::as_str)
    }
    fn field(&self, index: u32) -> Option<&str> {
        self.fields.get(&index).map(String::as_str)
    }
    fn ty(&self, index: u32) -> Option<&str> {
        self.types.get(&index).map(String::as_str)
    }
    fn string(&self, index: u32) -> Option<&[u16]> {
        self.strings.get(&index).map(Vec::as_slice)
    }
}

/// Build the frontend class node dexdec's recovery expects from the original
/// class facts. Fields the facts do not carry stay empty rather than inferred.
pub fn class_node(facts: &diana_facts::p0::ClassFacts) -> Option<crate::frontend::ClassNode> {
    use crate::frontend::{
        AccessInfo, ClassInfo, ClassNode, FieldInfo, FieldNode, MethodInfo, MethodNode,
    };
    use diana_facts::p0::Fact;

    let descriptor = String::from_utf16(facts.descriptor().units()).ok()?;
    let info = ClassInfo::from_type_descriptor(&descriptor).ok()?;
    let mut node = ClassNode::new(
        facts.class_def().0,
        info,
        AccessInfo::for_class(facts.access_flags().raw()),
    );
    if let Fact::Present(super_class) = facts.superclass() {
        if let Some(ty) = parse_type(super_class.units()) {
            node.set_super_class(ty);
        }
    }
    if let Fact::Present(interfaces) = facts.interfaces() {
        for interface in interfaces {
            if let diana_facts::p0::Fact::Present(descriptor) = &interface.descriptor {
                if let Some(ty) = parse_type(descriptor.units()) {
                    node.add_interface(ty);
                }
            }
        }
    }
    for entry in facts.methods() {
        let Fact::Present(declaration) = entry.declaration() else {
            continue;
        };
        let Ok(name) = String::from_utf16(declaration.name_units()) else {
            continue;
        };
        let mut parameters = Vec::new();
        let mut usable = true;
        for parameter in &declaration.parameter_types {
            match parse_type(parameter.units()) {
                Some(ty) => parameters.push(ty),
                None => usable = false,
            }
        }
        let return_type = parse_type(declaration.return_type.units());
        let (Some(return_type), true) = (return_type, usable) else {
            continue;
        };
        let info = MethodInfo::new(descriptor.clone(), name, parameters, return_type);
        let method = MethodNode::new(
            entry.method().0,
            info,
            AccessInfo::for_method(entry.access_flags().raw()),
        );
        node.add_method(method);
    }
    for entry in facts.fields() {
        let Fact::Present(declaration) = entry.declaration() else {
            continue;
        };
        let Ok(name) = String::from_utf16(declaration.name.units()) else {
            continue;
        };
        let Some(ty) = parse_type(declaration.descriptor.units()) else {
            continue;
        };
        let info = FieldInfo::new(descriptor.clone(), name, ty);
        node.add_field(FieldNode::new(
            entry.field().0,
            info,
            AccessInfo::for_field(entry.access_flags().raw()),
        ));
    }
    Some(node)
}

/// Parse one original descriptor into the frontend argument type.
fn parse_type(units: &[u16]) -> Option<crate::ir::ty::ArgType> {
    String::from_utf16(units)
        .ok()?
        .parse::<crate::ir::ty::ArgType>()
        .ok()
}
