//! Resolve JVM operands to values in the diff namespace. Constant-pool
//! offsets are input coordinates, so they never enter comparable records.

use std::collections::HashMap;

use diana_decomp::input::jvm::{
    BootstrapMethod, ClassFile, Constant, JvmString, MemberReference, ReferenceKind,
    bytecode::{Instruction, Reference, bootstrap_argument_reference, method_handle},
};

use crate::analysis::jvm_code::display;

use super::super::body::value;
use super::super::reference::Symbol;
use super::{Body, ExpansionBudget, RESOURCE_LIMIT, Symbols};

#[derive(Default)]
pub(super) struct DynamicSites {
    // CONSTANT_Dynamic linkage belongs to the constant-pool entry. Preserve
    // repeated uses of that same entry without comparing its physical index.
    ids: HashMap<u16, u32>,
}

#[derive(Clone, Copy)]
enum Encoded {
    Symbol(Symbol, ReferenceMarker),
    Integer(i32),
    Float(u32),
    Long(i64),
    Double(u64),
}

#[derive(Clone, Copy)]
enum ReferenceMarker {
    None,
    ClassMethod,
    Interface,
}

impl Encoded {
    fn write(self, body: &mut Body) -> Result<(), String> {
        match self {
            Self::Symbol(symbol, marker) => {
                write_symbol(body, symbol)?;
                match marker {
                    ReferenceMarker::None => Ok(()),
                    ReferenceMarker::ClassMethod => {
                        value(&mut body.operands, b'Z', b"class-method")
                    }
                    ReferenceMarker::Interface => {
                        value(&mut body.operands, b'Z', b"interface-method")
                    }
                }
            }
            Self::Integer(n) => value(&mut body.operands, b'I', &n.to_le_bytes()),
            Self::Float(n) => value(&mut body.operands, b'L', &n.to_le_bytes()),
            Self::Long(n) => value(&mut body.operands, b'J', &n.to_le_bytes()),
            Self::Double(n) => value(&mut body.operands, b'L', &n.to_le_bytes()),
        }
    }
}

pub(super) struct ConstantReader<'a, 'f> {
    class: &'a ClassFile<'f>,
    symbols: &'a Symbols,
    plain: HashMap<(u8, u16), Encoded>,
    bootstraps: Option<(Vec<BootstrapMethod<'f>>, Option<String>)>,
}

impl<'a, 'f> ConstantReader<'a, 'f> {
    pub(super) fn new(class: &'a ClassFile<'f>, symbols: &'a Symbols) -> Self {
        Self {
            class,
            symbols,
            plain: HashMap::new(),
            bootstraps: None,
        }
    }

    pub(super) fn class(&self) -> &ClassFile<'f> {
        self.class
    }

    pub(super) fn instruction(
        &mut self,
        body: &mut Body,
        dynamic: &mut DynamicSites,
        budget: &mut ExpansionBudget,
        instruction: Instruction<'f>,
        index: u16,
    ) -> Result<(), String> {
        let cache_key = (instruction.opcode(), index);
        if let Some(encoding) = self.plain.get(&cache_key) {
            return encoding.write(body);
        }
        let reference = instruction
            .reference(self.class)
            .map_err(|error| error.to_string())?;
        if let Some(Reference::Dynamic {
            invoke,
            bootstrap_index,
            name,
            descriptor,
        }) = reference
        {
            let checkpoint = dynamic.ids.len();
            let result = self.dynamic(
                body,
                dynamic,
                budget,
                Node {
                    index,
                    invoke,
                    bootstrap: bootstrap_index,
                    name,
                    descriptor,
                },
            );
            if result.is_err() {
                dynamic.ids.retain(|_, id| (*id as usize) < checkpoint);
            }
            return result;
        }
        let encoding = self.plain(index, reference, budget)?;
        self.plain
            .try_reserve(1)
            .map_err(|error| error.to_string())?;
        self.plain.insert(cache_key, encoding);
        encoding.write(body)
    }

    fn plain(
        &self,
        index: u16,
        reference: Option<Reference<'f>>,
        budget: &mut ExpansionBudget,
    ) -> Result<Encoded, String> {
        let (tag, text, marker) = match reference {
            Some(Reference::Class(name)) => (b't', class_descriptor(name)?, ReferenceMarker::None),
            Some(Reference::String(text)) => (b's', show(text)?, ReferenceMarker::None),
            Some(Reference::Member(member)) => {
                let (tag, marker) = match member.kind {
                    ReferenceKind::Field => (b'f', ReferenceMarker::None),
                    ReferenceKind::Method => (b'm', ReferenceMarker::ClassMethod),
                    ReferenceKind::InterfaceMethod => (b'm', ReferenceMarker::Interface),
                };
                (tag, member_text(member)?, marker)
            }
            Some(Reference::MethodType(descriptor)) => {
                (b'p', show(descriptor)?, ReferenceMarker::None)
            }
            Some(Reference::MethodHandle { kind, member, .. }) => (
                b'h',
                format!("handle[{kind}, {:?}] {}", member.kind, member_text(member)?),
                ReferenceMarker::None,
            ),
            Some(Reference::Dynamic { .. }) => {
                return Err("dynamic operand has no linkage node".to_owned());
            }
            None => {
                return Ok(
                    match self
                        .class
                        .constant(index)
                        .map_err(|error| error.to_string())?
                    {
                        Constant::Integer(n) => Encoded::Integer(n),
                        Constant::Float(n) => Encoded::Float(n),
                        Constant::Long(n) => Encoded::Long(n),
                        Constant::Double(n) => Encoded::Double(n),
                        _ => return Err(format!("unexpected non-numeric JVM constant #{index}")),
                    },
                );
            }
        };
        Ok(Encoded::Symbol(self.intern(tag, text, budget)?, marker))
    }

    fn intern(
        &self,
        tag: u8,
        text: String,
        budget: &mut ExpansionBudget,
    ) -> Result<Symbol, String> {
        if !budget.charge(text.len().saturating_add(size_of::<Symbol>() + 32)) {
            return Err(RESOURCE_LIMIT.to_owned());
        }
        self.symbols.intern(tag, text)
    }

    fn bootstrap(&mut self, index: u16) -> Result<BootstrapMethod<'f>, String> {
        let (rows, issue) = self.bootstraps.get_or_insert_with(|| {
            let mut rows = Vec::new();
            let mut issue = None;
            match self.class.bootstrap_methods() {
                Ok(Some(methods)) => {
                    for method in methods {
                        match method {
                            Ok(method) => rows.push(method),
                            Err(error) => {
                                issue = Some(error.to_string());
                                break;
                            }
                        }
                    }
                }
                Ok(None) => {}
                Err(error) => issue = Some(error.to_string()),
            }
            (rows, issue)
        });
        rows.get(usize::from(index)).copied().ok_or_else(|| {
            issue
                .clone()
                .unwrap_or_else(|| format!("JVM bootstrap row {index} is absent"))
        })
    }

    fn begin(
        &mut self,
        body: &mut Body,
        dynamic: &mut DynamicSites,
        budget: &mut ExpansionBudget,
        node: Node<'f>,
    ) -> Result<Option<impl Iterator<Item = u16> + 'f>, String> {
        if !node.invoke {
            if let Some(&id) = dynamic.ids.get(&node.index) {
                value(&mut body.operands, b'Z', b"constant-ref")?;
                value(&mut body.operands, b'I', &id.to_le_bytes())?;
                return Ok(None);
            }
            let id = dynamic.ids.len() as u32;
            dynamic
                .ids
                .try_reserve(1)
                .map_err(|error| error.to_string())?;
            dynamic.ids.insert(node.index, id);
            value(&mut body.operands, b'Z', b"constant-def")?;
            value(&mut body.operands, b'I', &id.to_le_bytes())?;
        } else {
            value(&mut body.operands, b'Z', b"call-site")?;
        }
        write_symbol(body, self.intern(b'y', member_name(node.name)?, budget)?)?;
        write_symbol(body, self.intern(b'y', show(node.descriptor)?, budget)?)?;
        let bootstrap = self.bootstrap(node.bootstrap)?;
        let handle = match self
            .class
            .constant(bootstrap.method_ref)
            .map_err(|error| error.to_string())?
        {
            Constant::MethodHandle {
                kind,
                reference_index,
            } if matches!(kind, 6 | 8) => {
                let member = method_handle(self.class, kind, reference_index)
                    .map_err(|error| error.to_string())?;
                Reference::MethodHandle {
                    kind,
                    reference_index,
                    member,
                }
            }
            _ => {
                return Err(format!(
                    "invalid JVM bootstrap method handle #{}",
                    bootstrap.method_ref
                ));
            }
        };
        self.plain(bootstrap.method_ref, Some(handle), budget)?
            .write(body)?;
        let arguments = bootstrap.arguments();
        value(
            &mut body.operands,
            b'I',
            &(arguments.len() as u32).to_le_bytes(),
        )?;
        Ok(Some(arguments))
    }

    fn dynamic(
        &mut self,
        body: &mut Body,
        dynamic: &mut DynamicSites,
        budget: &mut ExpansionBudget,
        node: Node<'f>,
    ) -> Result<(), String> {
        let Some(arguments) = self.begin(body, dynamic, budget, node)? else {
            return Ok(());
        };
        let mut stack = vec![arguments];
        while let Some(arguments) = stack.last_mut() {
            let Some(index) = arguments.next() else {
                stack.pop();
                continue;
            };
            // 0 is outside the JVM opcode namespace for any pool operand.
            let key = (0, index);
            if let Some(encoded) = self.plain.get(&key) {
                encoded.write(body)?;
                continue;
            }
            let argument = bootstrap_argument_reference(self.class, index)
                .map_err(|error| error.to_string())?;
            if let Some(Reference::Dynamic {
                invoke,
                bootstrap_index,
                name,
                descriptor,
            }) = argument
            {
                if let Some(arguments) = self.begin(
                    body,
                    dynamic,
                    budget,
                    Node {
                        index,
                        invoke,
                        bootstrap: bootstrap_index,
                        name,
                        descriptor,
                    },
                )? {
                    stack.push(arguments);
                }
                continue;
            }
            let encoded = self.plain(index, argument, budget)?;
            self.plain
                .try_reserve(1)
                .map_err(|error| error.to_string())?;
            self.plain.insert(key, encoded);
            encoded.write(body)?;
        }
        Ok(())
    }
}

struct Node<'a> {
    index: u16,
    invoke: bool,
    bootstrap: u16,
    name: JvmString<'a>,
    descriptor: JvmString<'a>,
}

fn write_symbol(body: &mut Body, symbol: Symbol) -> Result<(), String> {
    value(&mut body.operands, symbol.tag, &symbol.id.to_le_bytes())?;
    body.has_symbol_dependency |= symbol.tag != b's';
    if symbol.tag == b'm' {
        body.calls.push(symbol.id);
    }
    if let Some(anchor) = symbol.anchor {
        body.anchors.push(anchor);
    }
    Ok(())
}

fn show(value: JvmString<'_>) -> Result<String, String> {
    display(value).map_err(|error| error.to_string())
}

pub(super) fn member_name(name: JvmString<'_>) -> Result<String, String> {
    let mut name = show(name)?;
    // These units can appear in bytecode member names but also delimit the
    // canonical reference spelling. Original literal backslashes are escaped
    // by display(), so the spelling remains injective.
    if name.contains(['(', ':']) {
        name = name.replace('(', "\\u0028").replace(':', "\\u003a");
    }
    Ok(name)
}

fn class_descriptor(name: JvmString<'_>) -> Result<String, String> {
    let name = show(name)?;
    if name.starts_with('[') {
        Ok(name)
    } else {
        Ok(format!("L{name};"))
    }
}

fn member_text(member: MemberReference<'_>) -> Result<String, String> {
    let owner = class_descriptor(member.owner)?;
    let name = member_name(member.name)?;
    let descriptor = show(member.descriptor)?;
    let separator = if member.kind == ReferenceKind::Field {
        ":"
    } else {
        ""
    };
    Ok(format!("{owner}->{name}{separator}{descriptor}"))
}
