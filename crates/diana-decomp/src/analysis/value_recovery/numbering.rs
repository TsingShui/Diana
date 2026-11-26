//! Global value numbering over pure semantic expressions.

use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};

use crate::ir::analysis::SsaVar;
use crate::ir::{
    ArgType, ArithOp, CmpBias, InsnArg, InsnType, RegisterArg, SemanticExpression,
    SemanticOperation, UnaryOp, Utf16String,
};

use super::domain::{ControlDomain, DomainLogic};
use super::flow::{DefinitionFact, LexicalScopes, ScopeId, ValueIdentity};
use super::ValueRecoveryError;

pub(super) trait ValueAvailability {
    fn supports(
        &self,
        source: crate::ir::analysis::SemanticFlowPoint,
        target: crate::ir::analysis::SemanticFlowPoint,
    ) -> bool;

    fn unchanged(
        &self,
        value: SsaVar,
        source: crate::ir::analysis::SemanticFlowPoint,
        target: crate::ir::analysis::SemanticFlowPoint,
    ) -> bool;
}

#[derive(Debug, Clone)]
pub(super) struct SyntheticUse {
    pub(super) representative: SsaVar,
    pub(super) eliminated: SsaVar,
}

#[derive(Debug, Default)]
pub(super) struct ValueNumberingResult {
    pub(super) replacements: BTreeMap<SsaVar, InsnArg>,
    pub(super) synthetic_uses: Vec<SyntheticUse>,
}

pub(super) struct ValueNumbering<'a> {
    identity: ValueIdentity,
    logic: &'a DomainLogic,
    scopes: &'a LexicalScopes,
    canonical_values: &'a BTreeMap<SsaVar, InsnArg>,
    availability: Option<&'a dyn ValueAvailability>,
    aliases: BTreeMap<SsaVar, SsaVar>,
    classes: HashMap<ValueExpression, Vec<NumberedDefinition<'a>>>,
}

impl<'a> ValueNumbering<'a> {
    /// Visits single-definition classes in the collector's event order.
    pub(super) fn analyze_ordered(
        identity: ValueIdentity,
        logic: &'a DomainLogic,
        scopes: &'a LexicalScopes,
        definitions: impl IntoIterator<Item = &'a DefinitionFact<'a>>,
        canonical_values: &'a BTreeMap<SsaVar, InsnArg>,
        availability: Option<&'a dyn ValueAvailability>,
    ) -> Result<ValueNumberingResult, ValueRecoveryError> {
        let mut numbering = Self {
            identity,
            logic,
            scopes,
            canonical_values,
            availability,
            aliases: BTreeMap::new(),
            classes: HashMap::new(),
        };
        let mut result = ValueNumberingResult::default();
        for definition in definitions {
            numbering.record_alias(definition);
            let Some(operation) = definition.operation() else {
                continue;
            };
            if matches!(
                operation.insn_type,
                InsnType::Const | InsnType::ConstStr | InsnType::Move
            ) {
                continue;
            }
            let Some(expression) = numbering.expression(operation) else {
                continue;
            };
            let mut representative = None;
            for candidate in numbering
                .classes
                .get(&expression)
                .into_iter()
                .flatten()
                .rev()
            {
                if numbering.available(candidate, definition, &expression)? {
                    representative = Some(*candidate);
                    break;
                }
            }
            if let Some(representative) = representative {
                result.replacements.insert(
                    definition.key,
                    InsnArg::Reg(representative.register.clone()),
                );
                result.synthetic_uses.push(SyntheticUse {
                    representative: representative.key,
                    eliminated: definition.key,
                });
                numbering.aliases.insert(definition.key, representative.key);
                continue;
            }
            numbering
                .classes
                .entry(expression)
                .or_default()
                .push(NumberedDefinition {
                    key: definition.key,
                    register: definition.result,
                    domain: definition.domain,
                    scope: definition.scope,
                    event: definition.event,
                    repetitive: definition.repetitive,
                    site: definition.site,
                });
        }
        Ok(result)
    }

    fn record_alias(&mut self, definition: &DefinitionFact) {
        let Some(operation) = definition.operation() else {
            return;
        };
        if operation.insn_type != InsnType::Move {
            return;
        }
        let Some(source) = operation
            .operands()
            .first()
            .and_then(SemanticExpression::as_register)
            .and_then(|register| self.key(register))
        else {
            return;
        };
        let source = self.root(source);
        self.aliases.insert(definition.key, source);
    }

    fn available(
        &self,
        candidate: &NumberedDefinition<'_>,
        current: &DefinitionFact,
        expression: &ValueExpression,
    ) -> Result<bool, ValueRecoveryError> {
        if !(candidate.event < current.event
            && self.scopes.contains(candidate.scope, current.scope)
            && (!candidate.repetitive || current.repetitive)
            && self.logic.implies(current.domain, candidate.domain)?)
        {
            return Ok(false);
        }
        if self.identity == ValueIdentity::Ssa {
            return Ok(true);
        }
        let Some(availability) = self.availability else {
            return Ok(false);
        };
        let (Some(candidate_site), Some(current_site)) = (candidate.site, current.site) else {
            return Ok(false);
        };
        let candidate_input = crate::ir::analysis::SemanticFlowPoint::before(candidate_site);
        let candidate_result = crate::ir::analysis::SemanticFlowPoint::after(candidate_site);
        let current_input = crate::ir::analysis::SemanticFlowPoint::before(current_site);
        if !availability.supports(candidate_input, current_input)
            || !availability.supports(candidate_result, current_input)
            || !availability.unchanged(candidate.key, candidate_result, current_input)
        {
            return Ok(false);
        }
        Ok(expression
            .variables()
            .all(|value| availability.unchanged(value, candidate_input, current_input)))
    }

    fn expression(&self, instruction: &SemanticOperation) -> Option<ValueExpression> {
        if !Self::pure_root(instruction) {
            return None;
        }
        let mut key = ValueExpression {
            root: OperationKey::of(instruction)?,
            children: Vec::new(),
            operands: Vec::with_capacity(instruction.operands().len()),
        };
        // Arity partitions the operand stream into rows. Each Operation slot
        // introduces exactly one child row, in encounter order. Hashing and
        // equality therefore need neither recursive descent nor boxed keys.
        let mut pending = VecDeque::new();
        let mut current = instruction;
        let mut owned;
        loop {
            for operand in current.operands() {
                let slot = match operand {
                    SemanticExpression::Register(register) => {
                        let value = self.root(self.key(register)?);
                        self.canonical_operand(value, &mut pending)?
                    }
                    SemanticExpression::Literal(literal) => ValueOperand::Literal {
                        value: literal.value,
                        ty: literal.ty.clone(),
                    },
                    SemanticExpression::Operation(operation) => {
                        pending.push_back(operation.clone());
                        ValueOperand::Operation
                    }
                    SemanticExpression::Select { .. } => return None,
                };
                key.operands.push(slot);
            }
            let Some(next) = pending.pop_front() else {
                break;
            };
            if !Self::numberable(next.insn_type) {
                return None;
            }
            key.children.push(OperationKey::of(&next)?);
            owned = next;
            current = &owned;
        }
        Some(key)
    }

    fn canonical_operand(
        &self,
        mut value: SsaVar,
        pending: &mut VecDeque<SemanticOperation>,
    ) -> Option<ValueOperand> {
        let mut visited = super::AliasPath::default();
        while let Some(argument) = self.canonical_values.get(&value) {
            match argument {
                InsnArg::Lit(literal) => {
                    return Some(ValueOperand::Literal {
                        value: literal.value,
                        ty: literal.ty.clone(),
                    });
                }
                InsnArg::Wrapped(instruction) => {
                    let operation =
                        SemanticOperation::from_borrowed_instruction(instruction).ok()?;
                    if !Self::pure_root(&operation) {
                        return None;
                    }
                    pending.push_back(operation);
                    return Some(ValueOperand::Operation);
                }
                InsnArg::Reg(register) => {
                    value = self.root(self.key(register)?);
                    if !visited.insert(value) {
                        break;
                    }
                }
            }
        }
        Some(ValueOperand::Register(value))
    }

    /// Transitive effects include each nested operand, compound target and
    /// edge copy. A replacement taken from the CFG starts a separate tree.
    fn pure_root(operation: &SemanticOperation) -> bool {
        Self::numberable(operation.insn_type) && operation.effects().is_pure()
    }

    fn numberable(instruction: InsnType) -> bool {
        matches!(
            instruction,
            InsnType::Const
                | InsnType::ConstStr
                | InsnType::Arith
                | InsnType::Neg
                | InsnType::Not
                | InsnType::Cast
                | InsnType::Cmp
                | InsnType::InstanceOf
        )
    }

    fn root(&self, mut value: SsaVar) -> SsaVar {
        let mut visited = super::AliasPath::default();
        while let Some(&parent) = self.aliases.get(&value) {
            if !visited.insert(value) {
                break;
            }
            value = parent;
        }
        value
    }

    fn key(&self, register: &RegisterArg) -> Option<SsaVar> {
        self.identity.key(register)
    }
}

#[derive(Debug, Clone, Copy)]
struct NumberedDefinition<'a> {
    key: SsaVar,
    register: &'a RegisterArg,
    domain: ControlDomain,
    scope: ScopeId,
    event: usize,
    repetitive: bool,
    site: Option<crate::ir::SemanticSiteId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ValueExpression {
    root: OperationKey,
    children: Vec<OperationKey>,
    operands: Vec<ValueOperand>,
}

impl ValueExpression {
    fn variables(&self) -> impl Iterator<Item = SsaVar> {
        self.operands
            .iter()
            .filter_map(|operand| match operand {
                ValueOperand::Register(value) => Some(*value),
                _ => None,
            })
            .collect::<BTreeSet<_>>()
            .into_iter()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum ValueOperand {
    Register(SsaVar),
    Literal { value: i64, ty: ArgType },
    Operation,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct OperationKey {
    instruction: InsnType,
    result_type: ArgType,
    arithmetic: Option<ArithOp>,
    unary: Option<UnaryOp>,
    comparison_bias: Option<CmpBias>,
    cast_type: Option<ArgType>,
    class_type: Option<ArgType>,
    type_index: Option<u32>,
    string_value: Option<Utf16String>,
    argument_count: usize,
}

impl OperationKey {
    fn of(instruction: &SemanticOperation) -> Option<Self> {
        Some(Self {
            instruction: instruction.insn_type,
            result_type: instruction.result.as_ref()?.ty.clone(),
            arithmetic: instruction.payload.arith_op,
            unary: instruction.payload.unary_op,
            comparison_bias: instruction.payload.cmp_bias,
            cast_type: instruction.payload.cast_type.as_deref().cloned(),
            class_type: instruction.payload.class_type.as_deref().cloned(),
            type_index: instruction.payload.type_index,
            string_value: instruction.payload.string_value.as_deref().cloned(),
            argument_count: instruction.operands().len(),
        })
    }
}

#[cfg(test)]
#[path = "../../../tests/internal/numbering_effects.rs"]
mod tests;
