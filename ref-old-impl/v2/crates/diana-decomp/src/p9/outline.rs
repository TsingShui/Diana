//! Optional layout facts captured by the one P9 writer, not recovered from text.
use super::{Origin, RenderError, TextOffset, expressions::Expressions};
use crate::{CodeUnitOffset, MethodId, p2::InvokeKind, p7::*};
use serde::Serialize;

#[derive(Serialize)]
pub struct OutlineLine {
    pub start: TextOffset,
    pub end: TextOffset,
    pub depth: usize,
    pub method: Option<MethodId>,
    pub header: Option<Origin>,
    pub kind: &'static str,
}

#[derive(Debug, Eq, PartialEq, Hash, Serialize)]
pub struct OutlineTarget {
    pub owner: Vec<u16>,
    pub name: Vec<u16>,
    pub prototype: Vec<u16>,
}

#[derive(Serialize)]
pub struct OutlineCall {
    pub method: MethodId,
    pub position: TextOffset,
    pub pc: CodeUnitOffset,
    pub reference: ReferenceId,
    pub origin_only: bool,
    pub abi_attachment: bool,
    #[serde(serialize_with = "crate::substrate::serialization::debug")]
    pub dispatch: InvokeKind,
    /// None retains the call and its reference; it does not invent an external
    /// method for an unavailable declaration or a dynamic call site.
    pub target: Option<OutlineTarget>,
}

#[derive(Serialize)]
pub struct Outline {
    pub(super) lines: Vec<OutlineLine>,
    pub(super) calls: Vec<OutlineCall>,
}
impl Outline {
    pub(super) fn new() -> Self {
        Self {
            lines: Vec::new(),
            calls: Vec::new(),
        }
    }
    pub fn lines(&self) -> &[OutlineLine] {
        &self.lines
    }
    pub fn calls(&self) -> &[OutlineCall] {
        &self.calls
    }
}

fn append(to: &mut Vec<u16>, units: &[u16]) -> Result<(), RenderError> {
    to.try_reserve(units.len())
        .map_err(RenderError::Allocation)?;
    to.extend_from_slice(units);
    Ok(())
}
fn owned(units: &[u16]) -> Result<Vec<u16>, RenderError> {
    let mut result = Vec::new();
    append(&mut result, units)?;
    Ok(result)
}

impl<S: super::expressions::ExpressionSource> Expressions<'_, '_, S> {
    pub(super) fn outline_call(
        &mut self,
        target: ReferenceId,
        dispatch: InvokeKind,
    ) -> Result<(), RenderError> {
        if self.out.outline.is_none() {
            return Ok(());
        }
        let reference =
            self.code
                .references()
                .get(target.0 as usize)
                .ok_or(RenderError::Invariant(
                    "outline call reference is unavailable",
                ))?;
        let signature = if let Ok(ReferenceDeclaration::Member(MemberDeclaration::Method {
            owner,
            name,
            prototype,
        })) = &reference.declaration
        {
            let units = |id: TypeUseId| {
                self.code
                    .types()
                    .get(id.0 as usize)
                    .map(|t| t.units.as_slice())
                    .ok_or(RenderError::Invariant("outline call type is unavailable"))
            };
            let mut proto = Vec::new();
            append(&mut proto, &[40])?;
            for &parameter in &prototype.parameters {
                append(&mut proto, units(parameter)?)?;
            }
            append(&mut proto, &[41])?;
            append(&mut proto, units(prototype.returns)?)?;
            Some(OutlineTarget {
                owner: owned(units(*owner)?)?,
                name: owned(&name.units)?,
                prototype: proto,
            })
        } else {
            None
        };
        let call = OutlineCall {
            method: self.method,
            position: self.out.position(),
            pc: reference.position.pc,
            reference: target,
            origin_only: self.out.origin_only,
            abi_attachment: self.out.abi_comment,
            dispatch,
            target: signature,
        };
        if let Some(outline) = &mut self.out.outline {
            outline
                .calls
                .try_reserve(1)
                .map_err(RenderError::Allocation)?;
            outline.calls.push(call);
        }
        Ok(())
    }
}
