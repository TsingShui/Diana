//! Read-only complete/partial input views and method-local rendering state.

use crate::MethodId;
use crate::p7::*;
use crate::p9::{
    buffer::Buffer,
    names::{Lookup, MethodNames},
};

pub(in crate::p9) trait ExpressionSource {
    fn boolean_use(&self, _id: ExprId) -> bool {
        false
    }
    fn boolean_local(&self, _id: StorageId) -> bool {
        false
    }
    fn kotlin_notation(&self) -> bool {
        false
    }
    fn payloads(&self) -> &[ArrayPayload];
    fn types(&self) -> &[TypeUse];
    fn references(&self) -> &[Reference];
    fn locals(&self) -> &[LocalDeclaration];
    fn expressions(&self) -> &[Expression];
    fn constructions(&self) -> &[Construction];
    fn copies(&self) -> &[CopyBundle];
    fn local_initializer(&self, _local: StorageId) -> Option<(StatementId, LocalSpelling)> {
        None
    }
    /// A suspension surface makes one continuation occurrence implicit.
    fn suspension_call(&self, _id: ExprId) -> Option<&Suspension> {
        None
    }
    fn composed_use(&self, _id: ExprId) -> Option<ComposedExpression> {
        None
    }
    /// A partial product has no selected display plan, so it keeps the type.
    fn field_receiver_owner(&self, _reference: ReferenceId) -> Option<TypeUseId> {
        None
    }
    fn field_qualification(&self, _reference: ReferenceId) -> bool {
        true
    }
    fn inline_check(
        &self,
        _carrier: CheckCarrier,
        _occurrence: ExprId,
        _value: StorageId,
    ) -> Option<InlineCheck> {
        None
    }
    /// A check the declaration carries prints on the stored value, not on a read.
    fn definition_check(&self, _statement: StatementId) -> Option<InlineCheck> {
        None
    }
    fn statement_sites(&self, _statement: StatementId) -> &[SiteId] {
        &[]
    }
    fn statement_construction(&self, _statement: StatementId) -> Option<&Construction> {
        None
    }
    fn operation_pc(&self, _operation: crate::p2::OpId) -> Option<crate::CodeUnitOffset> {
        None
    }
    /// A partial product has no selected store form.
    fn natural_store(&self, _statement: StatementId) -> bool {
        false
    }
    /// A partial product has no selected allocation spelling.
    fn array_literal(&self, _statement: StatementId) -> Option<ArrayLiteral> {
        None
    }
}
impl ExpressionSource for MethodCode {
    fn boolean_use(&self, id: ExprId) -> bool {
        self.emission().boolean_use(id)
    }
    fn boolean_local(&self, id: StorageId) -> bool {
        self.emission().boolean_local(id)
    }
    fn local_initializer(&self, local: StorageId) -> Option<(StatementId, LocalSpelling)> {
        Some((
            self.emission().local_initializer(local)?,
            self.emission().local_types()[local.0 as usize],
        ))
    }
    fn suspension_call(&self, id: ExprId) -> Option<&Suspension> {
        self.emission().suspension_call(id)
    }
    fn kotlin_notation(&self) -> bool {
        self.emission().kotlin_continuation_view()
    }
    fn payloads(&self) -> &[ArrayPayload] {
        self.payloads()
    }
    fn types(&self) -> &[TypeUse] {
        self.types()
    }
    fn references(&self) -> &[Reference] {
        self.references()
    }
    fn locals(&self) -> &[LocalDeclaration] {
        self.locals()
    }
    fn expressions(&self) -> &[Expression] {
        self.expressions()
    }
    fn constructions(&self) -> &[Construction] {
        self.constructions()
    }
    fn copies(&self) -> &[CopyBundle] {
        self.copies()
    }
    fn composed_use(&self, id: ExprId) -> Option<ComposedExpression> {
        self.emission().composed_use(id)
    }
    fn field_receiver_owner(&self, reference: ReferenceId) -> Option<TypeUseId> {
        self.emission().field_receiver_owner(reference)
    }
    fn field_qualification(&self, reference: ReferenceId) -> bool {
        self.emission().field_qualification(reference)
    }
    fn inline_check(
        &self,
        carrier: CheckCarrier,
        occurrence: ExprId,
        value: StorageId,
    ) -> Option<InlineCheck> {
        self.emission().inline_check(carrier, occurrence, value)
    }
    fn definition_check(&self, statement: StatementId) -> Option<InlineCheck> {
        self.emission().definition_check(statement)
    }
    fn statement_sites(&self, statement: StatementId) -> &[SiteId] {
        self.statements()
            .get(statement.0 as usize)
            .map_or(&[], |statement| statement.sites.as_slice())
    }
    fn statement_construction(&self, statement: StatementId) -> Option<&Construction> {
        let StatementKind::Construction(id) = self.statements().get(statement.0 as usize)?.kind
        else {
            return None;
        };
        self.constructions().get(id.0 as usize)
    }
    fn operation_pc(&self, operation: crate::p2::OpId) -> Option<crate::CodeUnitOffset> {
        self.operations()
            .get(operation.0 as usize)
            .map(|source| source.pc)
    }
    fn natural_store(&self, statement: StatementId) -> bool {
        self.emission().natural_store(statement)
    }
    fn array_literal(&self, statement: StatementId) -> Option<ArrayLiteral> {
        self.emission().array_literal(statement)
    }
}
impl ExpressionSource for ExpressionPartial {
    fn payloads(&self) -> &[ArrayPayload] {
        self.payloads()
    }
    fn types(&self) -> &[TypeUse] {
        self.type_uses()
    }
    fn references(&self) -> &[Reference] {
        self.reference_declarations()
    }
    fn locals(&self) -> &[LocalDeclaration] {
        self.local_declarations()
    }
    fn expressions(&self) -> &[Expression] {
        self.expressions()
    }
    fn constructions(&self) -> &[Construction] {
        self.constructions()
    }
    fn copies(&self) -> &[CopyBundle] {
        self.copies()
    }
}
pub(in crate::p9) struct Expressions<'a, 'b, S: ExpressionSource = MethodCode> {
    pub out: &'b mut Buffer,
    pub method: MethodId,
    pub code: &'a S,
    pub names: &'a MethodNames,
    pub lookup: &'a Lookup<'a>,
    pub pending_controls: Option<&'b mut [Option<crate::p9::TextOffset>]>,
    /// The statement currently printing, so an inlined check-cast or an
    /// allocation's array literal can find the text it states.
    pub statement: Option<StatementId>,
    /// The printed occurrence currently rendering an inlined check-cast. A copy
    /// bundle is not a statement, so this context names its exact step; a copy
    /// phase, gap report or unrelated occurrence renders with no carrier.
    pub check_carrier: Option<CheckCarrier>,
}
