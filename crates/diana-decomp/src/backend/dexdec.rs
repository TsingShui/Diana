//! Adapter from the original DEX facts to the recovery backend's inputs.
//!
//! The backend never reads an image: this module hands it the class declaration,
//! one method's original code and the identities its pool indices name.

use crate::input::{ProgramIndex, RawMethodView};
use crate::p0::ClassFacts;
use crate::p1::DecodedMethod;
use crate::raw::OperandReference;
use dexdec::{ClassNode, LocalSymbols, MethodCode};

/// Assemble the backend's class node from P0 class facts.
pub fn class_node(facts: &ClassFacts) -> Option<ClassNode> {
    dexdec::local::class_node(facts)
}

/// Copy one method's original instruction stream and register counts.
pub fn method_code(view: &RawMethodView<'_>) -> MethodCode {
    let bytes = view.instructions();
    let mut insns = Vec::with_capacity(bytes.len() / 2);
    for pair in bytes.chunks_exact(2) {
        insns.push(u16::from_le_bytes([pair[0], pair[1]]));
    }
    MethodCode {
        registers_size: view.registers_size(),
        ins_size: view.ins_size(),
        outs_size: view.outs_size(),
        insns,
        tries: Vec::new(),
        debug_info: None,
    }
}

/// Resolve every pool index the decoded method names into its identity.
pub fn symbols(index: &ProgramIndex<'_>, method: &DecodedMethod<'_>) -> LocalSymbols {
    let mut symbols = LocalSymbols::default();
    for instruction in method.instructions() {
        for reference in instruction.references() {
            match reference.target {
                OperandReference::String(id) => {
                    if !symbols.strings.contains_key(&id.0) {
                        if let Ok(text) = index.string(id) {
                            if let Ok(units) = text.units() {
                                symbols.strings.insert(id.0, units);
                            }
                        }
                    }
                }
                OperandReference::Type(id) => {
                    if !symbols.types.contains_key(&id.0) {
                        if let Ok((text, _)) = index.type_descriptor(id) {
                            if let Ok(name) = text.utf8_lossy() {
                                symbols.types.insert(id.0, name.into_owned());
                            }
                        }
                    }
                }
                OperandReference::Field(id) => {
                    if !symbols.fields.contains_key(&id.0) {
                        if let Ok(reference) = index.field_reference(id) {
                            let owner = reference
                                .owner
                                .utf8_lossy()
                                .map(|t| t.into_owned())
                                .unwrap_or_default();
                            let name = reference
                                .declaration
                                .name
                                .utf8_lossy()
                                .map(|t| t.into_owned())
                                .unwrap_or_default();
                            let ty = reference
                                .declaration
                                .field_type
                                .utf8_lossy()
                                .map(|t| t.into_owned())
                                .unwrap_or_default();
                            symbols.fields.insert(id.0, format!("{owner}->{name}:{ty}"));
                        }
                    }
                }
                OperandReference::Method(id) => {
                    if !symbols.methods.contains_key(&id.0) {
                        if let Ok(reference) = index.method_reference(id) {
                            let owner = reference
                                .owner
                                .utf8_lossy()
                                .map(|t| t.into_owned())
                                .unwrap_or_default();
                            let name = reference
                                .declaration
                                .name
                                .utf8_lossy()
                                .map(|t| t.into_owned())
                                .unwrap_or_default();
                            let mut prototype = String::from("(");
                            for parameter in reference.declaration.prototype.parameter_descriptors()
                            {
                                prototype.push_str(
                                    &parameter
                                        .utf8_lossy()
                                        .map(|t| t.into_owned())
                                        .unwrap_or_default(),
                                );
                            }
                            prototype.push(')');
                            prototype.push_str(
                                &reference
                                    .declaration
                                    .prototype
                                    .return_type
                                    .utf8_lossy()
                                    .map(|t| t.into_owned())
                                    .unwrap_or_default(),
                            );
                            symbols
                                .methods
                                .insert(id.0, format!("{owner}->{name}{prototype}"));
                        }
                    }
                }
                OperandReference::Proto(_)
                | OperandReference::MethodHandle(_)
                | OperandReference::CallSite(_) => {}
            }
        }
    }
    symbols
}
