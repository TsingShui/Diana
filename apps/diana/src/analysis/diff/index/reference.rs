//! Symbolic identities of checked P1 reference operands. DEX-local pool indices
//! and encoded-value widths never become part of a cross-image reference key.
use std::collections::HashMap;
use std::fmt::Write;

use diana_decomp::input::DexString;
use diana_decomp::p1::{
    BootstrapToken, BootstrapValue, CheckedBootstrapFact, CheckedMemberFact, CheckedPrototype,
    CheckedReferenceFact,
};
use diana_decomp::raw::OperandReference;

use crate::analysis::at::escape_units;

pub(super) struct Cache<'a> {
    entries: HashMap<(u8, u32), Symbol>,
    pub budget: &'a mut super::ExpansionBudget,
}

impl<'a> Cache<'a> {
    pub(super) fn new(budget: &'a mut super::ExpansionBudget) -> Self {
        Self {
            entries: HashMap::new(),
            budget,
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct Symbol {
    pub tag: u8,
    pub id: u32,
    pub anchor: Option<u64>,
}

pub(super) fn resolve<'c>(
    cache: &'c mut Cache,
    symbols: &super::Symbols,
    target: OperandReference,
    fact: &CheckedReferenceFact<'_>,
) -> Result<&'c Symbol, String> {
    let key = match target {
        OperandReference::String(i) => (b's', i.0),
        OperandReference::Type(i) => (b't', i.0),
        OperandReference::Field(i) => (b'f', i.0),
        OperandReference::Method(i) => (b'm', i.0),
        OperandReference::Proto(i) => (b'p', i.0),
        OperandReference::MethodHandle(i) => (b'h', i.0),
        OperandReference::CallSite(i) => (b'c', i.0),
    };
    Ok(match cache.entries.entry(key) {
        std::collections::hash_map::Entry::Occupied(e) => e.into_mut(),
        std::collections::hash_map::Entry::Vacant(e) => {
            let text = reference(fact)?;
            if !cache
                .budget
                .charge(text.len().saturating_add(size_of::<Symbol>() + 32))
            {
                return Err(super::ROOT_CAPACITY.to_owned());
            }
            e.insert(symbols.intern(key.0, text)?)
        }
    })
}

pub(super) fn text(value: DexString<'_>) -> Result<String, String> {
    if value.bytes().len() > super::body::MAX_BYTES {
        return Err("string exceeds the per-method operand capacity".into());
    }
    if let Some(s) = value.utf8()
        && !s
            .bytes()
            .any(|byte| matches!(byte, b'\\' | b'\t' | b'\r' | b'\n' | 0))
        && (s.is_ascii() && s.len() as u64 == value.utf16_len().0 as u64)
    {
        return Ok(s.to_owned());
    }
    value
        .units()
        .map(|u| escape_units(&u))
        .map_err(|e| e.to_string())
}

// The reader deliberately offers a readable replacement projection for a
// damaged string. Keep its actual modified-UTF8 payload in the comparison key
// whenever ordinary UTF8 text does not identify those bytes unambiguously.
fn literal(value: DexString<'_>) -> Result<String, String> {
    let displayed = text(value)?;
    if let Some(utf8) = value.utf8()
        && (utf8.is_ascii() && utf8.len() == value.utf16_len().0 as usize
            || !utf8.is_ascii()
                && utf8.chars().all(|c| u32::from(c) <= 0xffff)
                && utf8.chars().count() == value.utf16_len().0 as usize)
    {
        return Ok(displayed);
    }
    let length = value
        .bytes()
        .len()
        .saturating_mul(2)
        .saturating_add(displayed.len());
    if length > super::body::MAX_BYTES {
        return Err("string exceeds the per-method operand capacity".into());
    }
    let mut escaped = format!("\\MUTF8({}:", value.utf16_len().0);
    for byte in value.bytes() {
        write!(&mut escaped, "{byte:02x}").map_err(|e| e.to_string())?;
    }
    escaped.push_str(") ");
    escaped.push_str(&displayed);
    Ok(escaped)
}

fn prototype(p: &CheckedPrototype<'_>) -> Result<String, String> {
    let mut out = String::from("(");
    for parameter in p.parameters() {
        let name = text(parameter.descriptor())?;
        if out.len().saturating_add(name.len()) > super::body::MAX_BYTES {
            return Err("prototype exceeds the per-method operand capacity".into());
        }
        out.push_str(&name);
    }
    out.push(')');
    out.push_str(&text(p.return_type())?);
    Ok(out)
}

fn field(owner: DexString<'_>, name: DexString<'_>, ty: DexString<'_>) -> Result<String, String> {
    Ok(format!("{}->{}:{}", text(owner)?, text(name)?, text(ty)?))
}

fn method(
    owner: DexString<'_>,
    name: DexString<'_>,
    proto: &CheckedPrototype<'_>,
) -> Result<String, String> {
    Ok(format!(
        "{}->{}{}",
        text(owner)?,
        text(name)?,
        prototype(proto)?
    ))
}

fn member(m: &CheckedMemberFact<'_>) -> Result<String, String> {
    match m {
        CheckedMemberFact::Field {
            owner,
            name,
            field_type,
            ..
        } => field(*owner, *name, *field_type),
        CheckedMemberFact::Method {
            owner,
            name,
            prototype,
            ..
        } => method(*owner, *name, prototype),
    }
}

fn bootstrap(b: &CheckedBootstrapFact<'_>) -> Result<String, String> {
    match b {
        CheckedBootstrapFact::String { value } => literal(*value),
        CheckedBootstrapFact::Type { descriptor, .. } => text(*descriptor),
        CheckedBootstrapFact::Member(m) => member(m),
        CheckedBootstrapFact::Proto { prototype: p } => prototype(p),
        CheckedBootstrapFact::MethodHandle { handle, member: m } => {
            Ok(format!("{} {}", handle.kind(), member(m)?))
        }
    }
}

fn part(out: &mut String, value: &str) -> Result<(), String> {
    if out.len().saturating_add(value.len()) > super::body::MAX_BYTES {
        return Err("call-site reference exceeds the per-method index capacity".into());
    }
    write!(out, "{}:{value}", value.len()).map_err(|e| e.to_string())
}

fn reference(r: &CheckedReferenceFact<'_>) -> Result<String, String> {
    match r {
        CheckedReferenceFact::String { value } => literal(*value),
        CheckedReferenceFact::Type { descriptor, .. } => text(*descriptor),
        CheckedReferenceFact::Field {
            owner,
            name,
            field_type,
            ..
        } => field(*owner, *name, *field_type),
        CheckedReferenceFact::Method {
            owner,
            name,
            prototype,
            ..
        } => method(*owner, *name, prototype),
        CheckedReferenceFact::Proto { prototype: p } => prototype(p),
        CheckedReferenceFact::MethodHandle { handle, member: m } => {
            Ok(format!("{} {}", handle.kind(), member(m)?))
        }
        CheckedReferenceFact::CallSite {
            name,
            handle,
            member,
            declarations,
            prototype: p,
            bootstrap: tokens,
            ..
        } => {
            let decl = |id: u32| {
                declarations
                    .get(id as usize)
                    .ok_or_else(|| "missing checked bootstrap declaration".to_owned())
                    .and_then(|d| bootstrap(&d.fact))
            };
            let mut out = format!("handle={} ", handle.kind());
            part(&mut out, &decl(member.0)?)?;
            part(&mut out, &text(*name)?)?;
            part(&mut out, &prototype(p)?)?;
            for token in tokens {
                match token {
                    BootstrapToken::EndContainer => out.push(']'),
                    BootstrapToken::Value {
                        name, tag, value, ..
                    } => {
                        write!(&mut out, "/{tag}/").map_err(|e| e.to_string())?;
                        if let Some(name) = name {
                            out.push('n');
                            part(&mut out, &text(*name)?)?;
                        }
                        let s = match value {
                            BootstrapValue::Signed(v) => v.to_string(),
                            BootstrapValue::Unsigned(v) | BootstrapValue::Bits { bits: v, .. } => {
                                v.to_string()
                            }
                            BootstrapValue::Null => "null".into(),
                            BootstrapValue::Boolean(v) => v.to_string(),
                            BootstrapValue::Reference { fact, .. } => decl(fact.0)?,
                            BootstrapValue::Array { count } => count.to_string(),
                            BootstrapValue::Annotation {
                                descriptor, count, ..
                            } => format!("{count} {}", text(*descriptor)?),
                        };
                        part(&mut out, &s)?;
                    }
                }
            }
            Ok(out)
        }
    }
}
