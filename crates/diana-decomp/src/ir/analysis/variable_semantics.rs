use smallvec::SmallVec;
use std::collections::{BTreeMap, BTreeSet};

use crate::ir::{
    ArgType, ArithOp, InsnType, InstructionId, MemberReference, PrimitiveType, RegisterArg,
    SemanticBindingKind, SemanticExpression, SemanticNode, SemanticOperation, SemanticPredicate,
    SemanticStatement, SemanticStatementKind, SemanticVisitor,
};

use super::SourceTypeEnvironment;

#[derive(Clone)]
pub struct VariableSemanticGraph {
    variables: BTreeMap<u32, VariableNode>,
    operations: foldhash::HashMap<InstructionId, OperationNode>,
    edges: foldhash::HashMap<u32, SmallVec<[VariableEdge; 2]>>,
}

impl std::fmt::Debug for VariableSemanticGraph {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let operations = self.operations.iter().collect::<BTreeMap<_, _>>();
        let groups = self.edges.iter().collect::<BTreeMap<_, _>>();
        let edges = groups
            .into_values()
            .flat_map(|edges| edges.iter())
            .collect::<Vec<_>>();
        formatter
            .debug_struct("VariableSemanticGraph")
            .field("variables", &self.variables)
            .field("operations", &operations)
            .field("edges", &edges)
            .finish()
    }
}
impl VariableSemanticGraph {
    pub fn analyze(root: &SemanticNode, types: &SourceTypeEnvironment) -> Self {
        Self::build(root, types, None)
    }

    /// Sharing the immutable environment lets operation rows compute their
    /// operand facts on first access during naming.
    pub(crate) fn analyze_shared(
        root: &SemanticNode,
        types: &std::sync::Arc<SourceTypeEnvironment>,
    ) -> Self {
        Self::build(root, types, Some(types))
    }

    fn build(
        root: &SemanticNode,
        types: &SourceTypeEnvironment,
        shared: Option<&std::sync::Arc<SourceTypeEnvironment>>,
    ) -> Self {
        let mut builder = VariableSemanticGraphBuilder::new(types, shared);
        builder.visit_node(root);
        builder.finish()
    }

    pub fn variables(&self) -> impl Iterator<Item = &VariableNode> {
        self.variables.values()
    }

    pub fn variable(&self, identity: u32) -> Option<&VariableNode> {
        self.variables.get(&identity)
    }

    pub fn operation(&self, identity: InstructionId) -> Option<&OperationNode> {
        self.operations.get(&identity)
    }

    pub fn edges_for(&self, variable: u32) -> impl Iterator<Item = &VariableEdge> {
        self.edges.get(&variable).into_iter().flatten()
    }
}

#[derive(Debug, Clone)]
pub struct VariableNode {
    identity: u32,
    ty: ArgType,
    definitions: usize,
    uses: usize,
    source_binding: bool,
}

impl VariableNode {
    pub fn identity(&self) -> u32 {
        self.identity
    }

    pub fn ty(&self) -> &ArgType {
        &self.ty
    }

    pub fn definitions(&self) -> usize {
        self.definitions
    }

    pub fn uses(&self) -> usize {
        self.uses
    }

    pub fn is_source_binding(&self) -> bool {
        self.source_binding
    }

    fn can_be_integral(&self) -> bool {
        match &self.ty {
            ArgType::Primitive(
                PrimitiveType::Byte
                | PrimitiveType::Short
                | PrimitiveType::Char
                | PrimitiveType::Int,
            ) => true,
            ArgType::Unknown(possible) => {
                !possible.is_empty()
                    && possible.iter().all(|ty| {
                        matches!(
                            ty,
                            PrimitiveType::Byte
                                | PrimitiveType::Short
                                | PrimitiveType::Char
                                | PrimitiveType::Int
                        )
                    })
            }
            _ => false,
        }
    }

    fn can_be_reference(&self) -> bool {
        match &self.ty {
            ArgType::Object(_) | ArgType::Array(_) => true,
            ArgType::Unknown(possible) => {
                !possible.is_empty()
                    && possible
                        .iter()
                        .all(|ty| matches!(ty, PrimitiveType::Object | PrimitiveType::Array))
            }
            _ => false,
        }
    }
}

#[derive(Clone)]
pub struct OperationNode {
    operation: SemanticOperation,
    operands: OperationOperands,
    loop_depth: usize,
}

#[derive(Clone)]
enum OperationOperands {
    Recorded(Vec<OperationOperand>),
    Shared {
        types: std::sync::Arc<SourceTypeEnvironment>,
        operands: std::sync::OnceLock<Vec<OperationOperand>>,
    },
}

impl std::fmt::Debug for OperationNode {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("OperationNode")
            .field("identity", &self.identity())
            .field("kind", &self.kind())
            .field("reference", &self.reference())
            .field("operands", &self.operands())
            .field("loop_depth", &self.loop_depth)
            .finish()
    }
}

impl OperationNode {
    pub fn identity(&self) -> InstructionId {
        self.operation.id
    }

    pub fn kind(&self) -> InsnType {
        self.operation.insn_type
    }

    pub fn reference(&self) -> Option<&MemberReference> {
        self.operation.payload.reference.as_deref()
    }

    pub fn operands(&self) -> &[OperationOperand] {
        match &self.operands {
            OperationOperands::Recorded(operands) => operands,
            OperationOperands::Shared { types, operands } => operands.get_or_init(|| {
                VariableSemanticGraphBuilder::operation_operands(types, &self.operation)
            }),
        }
    }

    pub fn loop_depth(&self) -> usize {
        self.loop_depth
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationOperand {
    ty: Option<ArgType>,
    literal: Option<i64>,
    reference: Option<std::sync::Arc<MemberReference>>,
}

impl OperationOperand {
    pub fn ty(&self) -> Option<&ArgType> {
        self.ty.as_ref()
    }

    pub fn literal(&self) -> Option<i64> {
        self.literal
    }

    pub fn reference(&self) -> Option<&MemberReference> {
        self.reference.as_deref()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VariableEdge {
    variable: u32,
    operation: Option<InstructionId>,
    kind: VariableEdgeKind,
}

impl VariableEdge {
    pub fn variable(&self) -> u32 {
        self.variable
    }

    pub fn operation(&self) -> Option<InstructionId> {
        self.operation
    }

    pub fn kind(&self) -> &VariableEdgeKind {
        &self.kind
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VariableEdgeKind {
    Definition,
    Operand(usize),
    Predicate,
    LoopIndex,
    LoopBound,
    LoopValue,
    RecurrenceStep,
    Return,
    Throw,
    Lock,
    Selector,
    ArrayIndex,
    ArrayValue,
    Dimension,
    FieldValue,
    Binding(SemanticBindingKind),
    Recurrence(RecurrenceKind),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RecurrenceKind {
    Increment,
    Decrement,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum VariableRole {
    Exception,
    Element,
    Index,
    Limit,
    Size,
    Counter,
    Remaining,
    Step,
    Value,
    Accumulator,
    Condition,
    Result,
    Lock,
    Selector,
    Iterator,
}

#[derive(Debug, Clone, Default)]
pub struct VariableRoleScores {
    scores: BTreeMap<u32, BTreeMap<VariableRole, u16>>,
}

impl VariableRoleScores {
    pub fn score(&self, variable: u32, role: VariableRole) -> u16 {
        self.scores
            .get(&variable)
            .and_then(|roles| roles.get(&role))
            .copied()
            .unwrap_or_default()
    }

    pub fn roles(&self, variable: u32) -> impl Iterator<Item = (VariableRole, u16)> + '_ {
        self.scores
            .get(&variable)
            .into_iter()
            .flat_map(|roles| roles.iter().map(|(role, score)| (*role, *score)))
    }

    fn add(&mut self, variable: u32, role: VariableRole, score: u16) {
        let current = self
            .scores
            .entry(variable)
            .or_default()
            .entry(role)
            .or_default();
        *current = current.saturating_add(score);
    }
}

pub trait VariableRoleAnalysis {
    fn analyze(&self, graph: &VariableSemanticGraph) -> VariableRoleScores;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct StructuralVariableRoleAnalysis;

impl VariableRoleAnalysis for StructuralVariableRoleAnalysis {
    fn analyze(&self, graph: &VariableSemanticGraph) -> VariableRoleScores {
        let mut scores = VariableRoleScores::default();
        let return_variables = graph
            .edges
            .values()
            .flatten()
            .filter_map(|edge| {
                matches!(edge.kind(), VariableEdgeKind::Return).then_some(edge.variable())
            })
            .collect::<BTreeSet<_>>();
        for variable in graph.variables() {
            if variable.ty() == &ArgType::BOOLEAN {
                scores.add(variable.identity(), VariableRole::Condition, 20);
            }
            if variable.ty().as_object() == Some("java/util/Iterator") {
                scores.add(variable.identity(), VariableRole::Iterator, 80);
            }
            for edge in graph.edges_for(variable.identity()) {
                let role = match edge.kind() {
                    VariableEdgeKind::Binding(SemanticBindingKind::Catch) => {
                        Some((VariableRole::Exception, 100))
                    }
                    VariableEdgeKind::Throw if variable.can_be_reference() => {
                        Some((VariableRole::Exception, 100))
                    }
                    VariableEdgeKind::Binding(SemanticBindingKind::ForEach) => {
                        Some((VariableRole::Element, 100))
                    }
                    VariableEdgeKind::LoopIndex if variable.can_be_integral() => {
                        Some((VariableRole::Index, 100))
                    }
                    VariableEdgeKind::LoopBound if variable.can_be_integral() => {
                        Some((VariableRole::Limit, 80))
                    }
                    VariableEdgeKind::LoopValue => Some((VariableRole::Value, 70)),
                    VariableEdgeKind::RecurrenceStep => Some((VariableRole::Step, 75)),
                    VariableEdgeKind::ArrayIndex if variable.can_be_integral() => {
                        Some((VariableRole::Index, 100))
                    }
                    VariableEdgeKind::Dimension if variable.can_be_integral() => {
                        Some((VariableRole::Size, 90))
                    }
                    VariableEdgeKind::Predicate if variable.ty() == &ArgType::BOOLEAN => {
                        Some((VariableRole::Condition, 70))
                    }
                    VariableEdgeKind::Return if return_variables.len() == 1 => {
                        Some((VariableRole::Result, 35))
                    }
                    VariableEdgeKind::Lock if variable.can_be_reference() => {
                        Some((VariableRole::Lock, 100))
                    }
                    VariableEdgeKind::Selector => Some((VariableRole::Selector, 80)),
                    VariableEdgeKind::Recurrence(
                        RecurrenceKind::Increment | RecurrenceKind::Decrement,
                    ) => {
                        let indexes_values = graph.edges_for(variable.identity()).any(|edge| {
                            matches!(
                                edge.kind(),
                                VariableEdgeKind::ArrayIndex | VariableEdgeKind::LoopIndex
                            )
                        });
                        Some(if indexes_values {
                            (VariableRole::Index, 90)
                        } else {
                            match edge.kind() {
                                VariableEdgeKind::Recurrence(RecurrenceKind::Increment) => {
                                    (VariableRole::Counter, 70)
                                }
                                VariableEdgeKind::Recurrence(RecurrenceKind::Decrement) => {
                                    (VariableRole::Remaining, 70)
                                }
                                _ => unreachable!(),
                            }
                        })
                    }
                    VariableEdgeKind::Recurrence(RecurrenceKind::Other) => {
                        Some((VariableRole::Accumulator, 70))
                    }
                    VariableEdgeKind::Definition => edge
                        .operation()
                        .and_then(|operation| graph.operation(operation))
                        .and_then(|operation| {
                            (operation.kind() == InsnType::ArrayLength)
                                .then_some((VariableRole::Size, 100))
                                .or_else(|| {
                                    (operation.kind() == InsnType::Aget
                                        && operation.loop_depth() != 0)
                                        .then_some((VariableRole::Element, 65))
                                })
                        }),
                    VariableEdgeKind::ArrayIndex
                    | VariableEdgeKind::Dimension
                    | VariableEdgeKind::Throw
                    | VariableEdgeKind::Lock
                    | VariableEdgeKind::LoopIndex
                    | VariableEdgeKind::LoopBound
                    | VariableEdgeKind::Operand(_)
                    | VariableEdgeKind::ArrayValue
                    | VariableEdgeKind::FieldValue
                    | VariableEdgeKind::Predicate
                    | VariableEdgeKind::Return => None,
                };
                if let Some((role, weight)) = role {
                    scores.add(variable.identity(), role, weight);
                }
            }
        }
        scores
    }
}

struct VariableSemanticGraphBuilder<'a> {
    types: &'a SourceTypeEnvironment,
    shared: Option<&'a std::sync::Arc<SourceTypeEnvironment>>,
    graph: VariableSemanticGraph,
    seen_edges: foldhash::HashSet<VariableEdge>,
    loop_depth: usize,
}

impl<'a> VariableSemanticGraphBuilder<'a> {
    fn new(
        types: &'a SourceTypeEnvironment,
        shared: Option<&'a std::sync::Arc<SourceTypeEnvironment>>,
    ) -> Self {
        Self {
            types,
            shared,
            graph: VariableSemanticGraph {
                variables: BTreeMap::new(),
                operations: Default::default(),
                edges: Default::default(),
            },
            seen_edges: Default::default(),
            loop_depth: 0,
        }
    }

    fn finish(self) -> VariableSemanticGraph {
        self.graph
    }

    fn ensure_variable(&mut self, register: &RegisterArg) -> Option<&mut VariableNode> {
        let identity = register.code_var?;
        let observed_type = || self.types.register_type(register).unwrap_or(&register.ty);
        Some(match self.graph.variables.entry(identity) {
            std::collections::btree_map::Entry::Occupied(entry) => {
                let variable = entry.into_mut();
                if !variable.ty.is_known() {
                    let ty = observed_type();
                    if ty.is_known() {
                        variable.ty = ty.clone();
                    }
                }
                variable
            }
            std::collections::btree_map::Entry::Vacant(entry) => entry.insert(VariableNode {
                identity,
                ty: observed_type().clone(),
                definitions: 0,
                uses: 0,
                source_binding: false,
            }),
        })
    }

    fn expression_type(
        types: &SourceTypeEnvironment,
        expression: &SemanticExpression,
    ) -> Option<ArgType> {
        let mut expression = expression;
        loop {
            let register = match expression {
                SemanticExpression::Register(register) => register,
                SemanticExpression::Literal(literal) => return Some(literal.ty.clone()),
                SemanticExpression::Operation(operation) => operation.result.as_ref()?,
                SemanticExpression::Select { when_true, .. } => {
                    expression = when_true;
                    continue;
                }
            };
            return Some(
                types
                    .register_type(register)
                    .unwrap_or(&register.ty)
                    .clone(),
            );
        }
    }

    fn operation_operands(
        types: &SourceTypeEnvironment,
        operation: &SemanticOperation,
    ) -> Vec<OperationOperand> {
        operation
            .operands()
            .iter()
            .map(|operand| OperationOperand {
                ty: Self::expression_type(types, operand),
                literal: operand.literal_value(),
                reference: Self::expression_reference(operand),
            })
            .collect()
    }

    fn expression_reference(
        expression: &SemanticExpression,
    ) -> Option<std::sync::Arc<MemberReference>> {
        let mut expression = expression;
        loop {
            let SemanticExpression::Operation(operation) = expression else {
                return None;
            };
            if matches!(operation.insn_type, InsnType::Move | InsnType::CheckCast)
                && operation.operands().len() == 1
            {
                expression = &operation.operands()[0];
                continue;
            }
            return operation.payload.reference.clone();
        }
    }

    fn source_binding(&mut self, register: &RegisterArg) {
        if let Some(variable) = self.ensure_variable(register) {
            variable.source_binding = true;
        }
    }

    fn definition(&mut self, register: &RegisterArg, operation: Option<InstructionId>) {
        if let Some(variable) = self.ensure_variable(register) {
            variable.definitions += 1;
            let identity = variable.identity;
            self.edge(identity, operation, VariableEdgeKind::Definition);
        }
    }

    fn edge(&mut self, variable: u32, operation: Option<InstructionId>, kind: VariableEdgeKind) {
        let edge = VariableEdge {
            variable,
            operation,
            kind,
        };
        let edges = self.graph.edges.entry(variable).or_default();
        if edges.len() < 12 {
            if !edges.contains(&edge) {
                edges.push(edge);
                if edges.len() == 12 {
                    self.seen_edges.extend(edges.iter().copied());
                }
            }
        } else if self.seen_edges.insert(edge) {
            edges.push(edge);
        }
    }

    fn expression_root_edge(
        &mut self,
        expression: &SemanticExpression,
        operation: Option<InstructionId>,
        kind: VariableEdgeKind,
    ) {
        if let Some(variable) = VariableReferences::expression_root(expression) {
            self.edge(variable, operation, kind);
        }
    }

    fn predicate_edges(&mut self, predicate: &SemanticPredicate) {
        for variable in VariableReferences::predicate(predicate) {
            self.edge(variable, None, VariableEdgeKind::Predicate);
        }
    }

    fn for_loop_edges(&mut self, update: &SemanticStatement, condition: &SemanticPredicate) {
        let Some(index) = Self::statement_definition(update) else {
            return;
        };
        if !Self::statement_uses_variable(update, index) {
            self.edge(
                index,
                Self::statement_value_operation(update),
                VariableEdgeKind::LoopValue,
            );
            return;
        }
        self.edge(index, None, VariableEdgeKind::LoopIndex);
        for bound in VariableReferences::predicate_roots(condition) {
            if bound != index {
                self.edge(bound, None, VariableEdgeKind::LoopBound);
            }
        }
    }

    fn statement_definition(statement: &SemanticStatement) -> Option<u32> {
        match &statement.kind {
            SemanticStatementKind::Instruction(operation) => operation
                .result
                .as_ref()
                .and_then(|result| result.code_var)
                .or_else(|| {
                    operation
                        .compound_target()
                        .and_then(VariableReferences::expression_root)
                }),
            SemanticStatementKind::Definition { result, .. } => result.code_var,
        }
    }

    fn statement_uses_variable(statement: &SemanticStatement, variable: u32) -> bool {
        match &statement.kind {
            SemanticStatementKind::Instruction(operation) => operation
                .operands()
                .iter()
                .chain(operation.compound_target())
                .any(|expression| VariableReferences::expression(expression).contains(&variable)),
            SemanticStatementKind::Definition { value, .. } => {
                VariableReferences::expression(value).contains(&variable)
            }
        }
    }

    fn statement_value_operation(statement: &SemanticStatement) -> Option<InstructionId> {
        let expression_operation = |expression: &SemanticExpression| {
            expression.as_operation().map(|operation| operation.id)
        };
        match &statement.kind {
            SemanticStatementKind::Instruction(operation) => (operation.insn_type
                == InsnType::Invoke)
                .then_some(operation.id)
                .or_else(|| operation.operands().first().and_then(expression_operation)),
            SemanticStatementKind::Definition { value, .. } => expression_operation(value),
        }
    }

    fn recurrence(&mut self, operation: &SemanticOperation) {
        if self.loop_depth == 0 || operation.insn_type != InsnType::Arith {
            return;
        }
        let Some(variable) = operation.result.as_ref().and_then(|result| result.code_var) else {
            return;
        };
        let operands = operation.operands();
        if !operands
            .iter()
            .any(|operand| VariableReferences::expression(operand).contains(&variable))
        {
            return;
        }
        let literal = operands.iter().find_map(|operand| operand.literal_value());
        let kind = match (operation.payload.arith_op, literal) {
            (Some(ArithOp::Add), Some(1)) | (Some(ArithOp::Sub), Some(-1)) => {
                RecurrenceKind::Increment
            }
            (Some(ArithOp::Add), Some(-1)) | (Some(ArithOp::Sub), Some(1)) => {
                RecurrenceKind::Decrement
            }
            _ => RecurrenceKind::Other,
        };
        for step in operands
            .iter()
            .filter_map(VariableReferences::expression_root)
            .filter(|step| *step != variable)
        {
            self.edge(step, Some(operation.id), VariableEdgeKind::RecurrenceStep);
        }
        self.edge(
            variable,
            Some(operation.id),
            VariableEdgeKind::Recurrence(kind),
        );
    }

    fn visit_predicate_tests(&mut self, predicate: &SemanticPredicate) {
        let mut pending = vec![predicate];
        while let Some(predicate) = pending.pop() {
            match predicate {
                SemanticPredicate::Test(operation) => self.visit_operation(operation),
                SemanticPredicate::Not(inner) => pending.push(inner),
                SemanticPredicate::And(terms) | SemanticPredicate::Or(terms) => {
                    pending.extend(terms.iter().rev())
                }
                SemanticPredicate::True | SemanticPredicate::False => {}
            }
        }
    }
}

impl SemanticVisitor for VariableSemanticGraphBuilder<'_> {
    fn enter_node(&mut self, node: &SemanticNode) {
        let nested_loop = matches!(
            node,
            SemanticNode::Loop { .. } | SemanticNode::For { .. } | SemanticNode::ForEach { .. }
        );
        self.loop_depth += usize::from(nested_loop);
        match node {
            SemanticNode::Synchronized { lock, .. } => {
                self.expression_root_edge(lock, None, VariableEdgeKind::Lock)
            }
            SemanticNode::Switch { selector, .. } => {
                self.expression_root_edge(selector, None, VariableEdgeKind::Selector)
            }
            SemanticNode::For {
                update, condition, ..
            } => self.for_loop_edges(update, condition),
            SemanticNode::Leave(leave) => match &leave.kind {
                crate::ir::SemanticLeaveKind::Return(Some(value)) => {
                    self.expression_root_edge(value, None, VariableEdgeKind::Return)
                }
                crate::ir::SemanticLeaveKind::Throw(value) => {
                    self.expression_root_edge(value, None, VariableEdgeKind::Throw)
                }
                _ => {}
            },
            _ => {}
        }
    }

    fn exit_node(&mut self, node: &SemanticNode) {
        let nested_loop = matches!(
            node,
            SemanticNode::Loop { .. } | SemanticNode::For { .. } | SemanticNode::ForEach { .. }
        );
        self.loop_depth -= usize::from(nested_loop);
    }

    fn visit_statement(&mut self, statement: &SemanticStatement) {
        match &statement.kind {
            SemanticStatementKind::Instruction(operation) => {
                if let Some(result) = &operation.result {
                    self.source_binding(result);
                }
                self.visit_operation(operation);
            }
            SemanticStatementKind::Definition { result, value, .. } => {
                self.source_binding(result);
                self.definition(result, Some(statement.id()));
                self.visit_expression(value);
            }
        }
    }

    fn visit_predicate(&mut self, predicate: &SemanticPredicate) {
        self.predicate_edges(predicate);
        self.visit_predicate_tests(predicate);
    }

    fn visit_operation(&mut self, operation: &SemanticOperation) {
        self.graph
            .operations
            .entry(operation.id)
            .or_insert_with(|| OperationNode {
                operation: operation.clone(),
                operands: match self.shared {
                    Some(types) if !operation.operands().is_empty() => OperationOperands::Shared {
                        types: std::sync::Arc::clone(types),
                        operands: std::sync::OnceLock::new(),
                    },
                    _ => {
                        OperationOperands::Recorded(Self::operation_operands(self.types, operation))
                    }
                },
                loop_depth: self.loop_depth,
            });
        if let Some(result) = &operation.result {
            self.definition(result, Some(operation.id));
        }
        if matches!(operation.insn_type, InsnType::Iput | InsnType::Sput) {
            if let Some(variable) = operation
                .operands()
                .first()
                .and_then(VariableReferences::expression_root)
            {
                self.edge(variable, Some(operation.id), VariableEdgeKind::FieldValue);
            }
        }
        for (index, operand) in operation.operands().iter().enumerate() {
            let contextual = match (operation.insn_type, index) {
                (InsnType::Aget, 1) | (InsnType::Aput, 2) => Some(VariableEdgeKind::ArrayIndex),
                (InsnType::Aput, 0) => Some(VariableEdgeKind::ArrayValue),
                (InsnType::NewArray, _) => Some(VariableEdgeKind::Dimension),
                (InsnType::MonitorEnter | InsnType::MonitorExit, 0) => Some(VariableEdgeKind::Lock),
                _ => None,
            };
            let mut record = |variable| {
                self.edge(
                    variable,
                    Some(operation.id),
                    VariableEdgeKind::Operand(index),
                );
                if let Some(kind) = contextual {
                    self.edge(variable, Some(operation.id), kind);
                }
            };
            match operand {
                SemanticExpression::Register(register) => {
                    if let Some(variable) = register.code_var {
                        record(variable);
                    }
                }
                SemanticExpression::Literal(_) => {}
                _ => VariableReferences::expression(operand)
                    .into_iter()
                    .for_each(record),
            }
            self.visit_expression(operand);
        }
        if let Some(target) = operation.compound_target() {
            self.visit_expression(target);
        }
        self.recurrence(operation);
    }

    fn visit_register(&mut self, register: &RegisterArg) {
        if let Some(variable) = self.ensure_variable(register) {
            variable.uses += 1;
        }
    }

    fn visit_binding(&mut self, kind: SemanticBindingKind, register: &RegisterArg) {
        if let Some(variable) = self.ensure_variable(register) {
            variable.definitions += 1;
            variable.source_binding = true;
            let identity = variable.identity;
            self.edge(identity, None, VariableEdgeKind::Binding(kind));
        }
    }
}

#[derive(Default)]
struct VariableReferences {
    variables: BTreeSet<u32>,
}

impl VariableReferences {
    fn expression(expression: &SemanticExpression) -> BTreeSet<u32> {
        let mut collector = Self::default();
        collector.visit_expression(expression);
        collector.variables
    }

    fn predicate(predicate: &SemanticPredicate) -> BTreeSet<u32> {
        let mut collector = Self::default();
        collector.visit_predicate(predicate);
        collector.variables
    }

    fn predicate_roots(predicate: &SemanticPredicate) -> BTreeSet<u32> {
        let mut variables = BTreeSet::new();
        let mut pending = vec![predicate];
        while let Some(predicate) = pending.pop() {
            match predicate {
                SemanticPredicate::Test(operation) => {
                    variables.extend(
                        operation
                            .operands()
                            .iter()
                            .filter_map(Self::expression_root),
                    );
                }
                SemanticPredicate::Not(inner) => pending.push(inner),
                SemanticPredicate::And(terms) | SemanticPredicate::Or(terms) => {
                    pending.extend(terms)
                }
                SemanticPredicate::True | SemanticPredicate::False => {}
            }
        }
        variables
    }

    fn expression_root(expression: &SemanticExpression) -> Option<u32> {
        match expression {
            SemanticExpression::Register(register) => register.code_var,
            SemanticExpression::Operation(operation) => {
                operation.result.as_ref().and_then(|result| result.code_var)
            }
            SemanticExpression::Literal(_) | SemanticExpression::Select { .. } => None,
        }
    }
}

impl SemanticVisitor for VariableReferences {
    fn visit_register(&mut self, register: &RegisterArg) {
        if let Some(variable) = register.code_var {
            self.variables.insert(variable);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn graph_with_edge(ty: ArgType, kind: VariableEdgeKind) -> VariableSemanticGraph {
        VariableSemanticGraph {
            variables: BTreeMap::from([(
                1,
                VariableNode {
                    identity: 1,
                    ty,
                    definitions: 1,
                    uses: 1,
                    source_binding: true,
                },
            )]),
            operations: Default::default(),
            edges: [(
                1,
                smallvec::smallvec![VariableEdge {
                    variable: 1,
                    operation: None,
                    kind,
                }],
            )]
            .into_iter()
            .collect(),
        }
    }

    #[test]
    fn array_index_requires_an_integral_variable() {
        let reference = graph_with_edge(ArgType::unknown_object(), VariableEdgeKind::ArrayIndex);
        let index = graph_with_edge(ArgType::INT, VariableEdgeKind::ArrayIndex);

        let analysis = StructuralVariableRoleAnalysis;
        assert_eq!(
            analysis.analyze(&reference).score(1, VariableRole::Index),
            0
        );
        assert_eq!(analysis.analyze(&index).score(1, VariableRole::Index), 100);
    }

    #[test]
    fn predicate_role_requires_a_boolean_variable() {
        let integer = graph_with_edge(ArgType::INT, VariableEdgeKind::Predicate);
        let boolean = graph_with_edge(ArgType::BOOLEAN, VariableEdgeKind::Predicate);

        let analysis = StructuralVariableRoleAnalysis;
        assert_eq!(
            analysis.analyze(&integer).score(1, VariableRole::Condition),
            0
        );
        assert_eq!(
            analysis.analyze(&boolean).score(1, VariableRole::Condition),
            90
        );
    }
}

#[cfg(test)]
#[path = "../../../tests/internal/semantic_variable_edges.rs"]
mod edge_tests;
