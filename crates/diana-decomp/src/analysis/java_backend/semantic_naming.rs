use smallvec::SmallVec;
use std::collections::{BTreeMap, BTreeSet};

use crate::analysis::jadx_local_names;
use crate::ir::{
    analysis::{
        StructuralVariableRoleAnalysis, VariableNode, VariableRole, VariableRoleAnalysis,
        VariableRoleScores, VariableSemanticGraph,
    },
    ArgType, InsnType, MemberReference, PrimitiveType, SemanticNode,
};
use crate::language::java::{JavaIdentifier, JavaNameScope, JavaPrimitiveType, JavaType};

use super::type_names::JavaTypeNameResolver;

#[derive(Debug, Clone, PartialEq, Eq)]
struct NameCandidate {
    name: JavaIdentifier,
    score: u16,
}

trait IdentifierMorphology {
    fn lower_camel(&self, name: &JavaIdentifier) -> JavaIdentifier;
    fn plural(&self, name: &JavaIdentifier) -> JavaIdentifier;
}

#[derive(Debug, Clone, Copy, Default)]
struct JavaIdentifierMorphology;

impl IdentifierMorphology for JavaIdentifierMorphology {
    fn lower_camel(&self, name: &JavaIdentifier) -> JavaIdentifier {
        let source = name.as_str();
        let words = source
            .split('_')
            .filter(|word| !word.is_empty())
            .collect::<Vec<_>>();
        if words.len() > 1 {
            let mut result = words[0].to_ascii_lowercase();
            for word in &words[1..] {
                let lowered = word.to_ascii_lowercase();
                let mut characters = lowered.chars();
                if let Some(first) = characters.next() {
                    result.extend(first.to_uppercase());
                    result.extend(characters);
                }
            }
            return JavaIdentifier::from_hint(&result);
        }
        if source
            .chars()
            .filter(|character| character.is_alphabetic())
            .all(|character| character.is_uppercase())
        {
            return JavaIdentifier::from_hint(&source.to_lowercase());
        }
        let mut characters = source.chars();
        let Some(first) = characters.next() else {
            return name.clone();
        };
        let mut lowered = first.to_lowercase().collect::<String>();
        lowered.extend(characters);
        JavaIdentifier::from_hint(&lowered)
    }

    fn plural(&self, name: &JavaIdentifier) -> JavaIdentifier {
        let source = name.as_str();
        let plural = if source.ends_with('y')
            && source
                .chars()
                .rev()
                .nth(1)
                .is_some_and(|character| !"aeiou".contains(character))
        {
            format!("{}ies", &source[..source.len() - 1])
        } else if source.ends_with('s')
            || source.ends_with('x')
            || source.ends_with('z')
            || source.ends_with("ch")
            || source.ends_with("sh")
        {
            format!("{source}es")
        } else {
            format!("{source}s")
        };
        JavaIdentifier::from_hint(&plural)
    }
}

trait MethodResultSemantics {
    fn candidate(&self, name: &str) -> Option<NameCandidate>;
}

#[derive(Debug, Clone, Copy, Default)]
struct JavaMethodResultSemantics {
    morphology: JavaIdentifierMorphology,
}

impl MethodResultSemantics for JavaMethodResultSemantics {
    fn candidate(&self, name: &str) -> Option<NameCandidate> {
        if name.starts_with('<')
            || name.contains('$')
            || name
                .chars()
                .filter(|character| character.is_alphanumeric())
                .count()
                < 3
        {
            return None;
        }
        if let Some(name) = jadx_local_names::invoke_local_name(name) {
            return Some(NameCandidate {
                name: JavaIdentifier::from_hint(&name),
                score: 85,
            });
        }
        Some(NameCandidate {
            name: self
                .morphology
                .lower_camel(&JavaIdentifier::from_hint(name)),
            score: 55,
        })
    }
}

trait VariableNameModel {
    fn candidates(
        &self,
        variable: &VariableNode,
        graph: &VariableSemanticGraph,
        roles: &VariableRoleScores,
        conflict_penalty: u16,
    ) -> SmallVec<[NameCandidate; 3]>;
}

struct StructuralNameModel<'a> {
    resolved_types: Option<&'a BTreeMap<ArgType, JavaType>>,
    types: &'a JavaTypeNameResolver,
    morphology: JavaIdentifierMorphology,
    methods: JavaMethodResultSemantics,
    constants: Option<crate::analysis::TypedConstantNameInference>,
    relations: Option<crate::analysis::RelationalNameInference>,
}

impl<'a> StructuralNameModel<'a> {
    fn new(types: &'a JavaTypeNameResolver) -> Self {
        Self {
            types,
            resolved_types: None,
            morphology: JavaIdentifierMorphology,
            methods: JavaMethodResultSemantics::default(),
            constants: None,
            relations: None,
        }
    }

    fn for_graph(
        types: &'a JavaTypeNameResolver,
        resolved_types: &'a BTreeMap<ArgType, JavaType>,
        graph: &VariableSemanticGraph,
    ) -> Self {
        let mut model = Self::new(types);
        model.resolved_types = Some(resolved_types);
        model.constants = crate::platform_symbols::default_platform_symbols()
            .ok()
            .map(|symbols| crate::analysis::TypedConstantNameInference::analyze(graph, &symbols));
        model.relations = Some(crate::analysis::RelationalNameInference::analyze(graph));
        model
    }

    fn role_name(role: VariableRole) -> &'static str {
        match role {
            VariableRole::Exception => "exception",
            VariableRole::Element => "element",
            VariableRole::Index => "index",
            VariableRole::Limit => "limit",
            VariableRole::Size => "size",
            VariableRole::Counter => "count",
            VariableRole::Remaining => "remaining",
            VariableRole::Step => "step",
            VariableRole::Value => "value",
            VariableRole::Accumulator => "total",
            VariableRole::Condition => "condition",
            VariableRole::Result => "result",
            VariableRole::Lock => "lock",
            VariableRole::Selector => "selector",
            VariableRole::Iterator => "iterator",
        }
    }

    fn type_name(&self, ty: &ArgType) -> Option<JavaIdentifier> {
        if let Some(resolved) = self.resolved_types.and_then(|types| types.get(ty)) {
            return self.java_type_name(resolved);
        }
        let resolved = self.types.resolve_type(ty).ok()?;
        self.java_type_name(&resolved)
    }

    fn java_type_name(&self, ty: &JavaType) -> Option<JavaIdentifier> {
        Some(JavaIdentifier::from_hint(&java_jadx_type_name(ty)?))
    }

    fn lower_camel(name: &JavaIdentifier) -> JavaIdentifier {
        let source = name.as_str();
        if source
            .chars()
            .filter(|character| character.is_alphabetic())
            .all(|character| character.is_uppercase())
        {
            return JavaIdentifier::from_hint(&source.to_lowercase());
        }
        let mut characters = source.chars();
        let Some(first) = characters.next() else {
            return name.clone();
        };
        let mut lowered = first.to_lowercase().collect::<String>();
        lowered.extend(characters);
        JavaIdentifier::from_hint(&lowered)
    }

    fn edge_candidate(
        &self,
        edge: &crate::ir::analysis::VariableEdge,
        graph: &VariableSemanticGraph,
    ) -> Option<NameCandidate> {
        use crate::ir::analysis::VariableEdgeKind::{Definition, FieldValue, LoopValue};
        let kind = edge.kind();
        if !matches!(kind, Definition | FieldValue | LoopValue) {
            return None;
        }
        let operation = graph.operation(edge.operation()?)?;
        match (kind, operation.kind(), operation.reference()?) {
            (Definition, InsnType::Iget | InsnType::Sget, MemberReference::Field(field))
            | (FieldValue, InsnType::Iput | InsnType::Sput, MemberReference::Field(field)) => {
                Some(NameCandidate {
                    name: self
                        .morphology
                        .lower_camel(&JavaIdentifier::from_hint(&field.name)),
                    score: if *kind == Definition { 110 } else { 85 },
                })
            }
            (Definition, InsnType::Invoke, MemberReference::Method(method)) => {
                self.methods.candidate(&method.name)
            }
            (LoopValue, InsnType::Invoke, MemberReference::Method(method))
                if !method.name.starts_with('<') =>
            {
                Some(NameCandidate {
                    name: self
                        .morphology
                        .lower_camel(&JavaIdentifier::from_hint(&method.name)),
                    score: 90,
                })
            }
            _ => None,
        }
    }
}

impl VariableNameModel for StructuralNameModel<'_> {
    fn candidates(
        &self,
        variable: &VariableNode,
        graph: &VariableSemanticGraph,
        roles: &VariableRoleScores,
        conflict_penalty: u16,
    ) -> SmallVec<[NameCandidate; 3]> {
        let identity = variable.identity();
        let mut candidates = SmallVec::<[NameCandidate; 3]>::new();
        let mut type_score = 40;
        for (role, score) in roles.roles(identity) {
            candidates.push(NameCandidate {
                name: JavaIdentifier::from_hint(Self::role_name(role)),
                score,
            });
            if role == VariableRole::Element && score != 0 {
                type_score = type_score.max(score.saturating_add(25));
            }
        }
        candidates.extend(
            graph
                .edges_for(identity)
                .filter_map(|edge| self.edge_candidate(edge, graph)),
        );
        if let Some(field) = self
            .constants
            .as_ref()
            .and_then(|constants| constants.name(identity))
        {
            candidates.push(NameCandidate {
                name: self.morphology.lower_camel(&JavaIdentifier::from_hint(
                    field.strip_prefix("FLAG_").unwrap_or(field),
                )),
                score: 125,
            });
        }
        if let Some(name) = self
            .relations
            .as_ref()
            .and_then(|relations| relations.name(identity))
        {
            candidates.push(NameCandidate {
                name: JavaIdentifier::from_hint(name),
                score: 115,
            });
        }
        // Even a reserved name loses only the conflict penalty. Keep ties so
        // the solver can still choose the lexicographically first candidate.
        let best_score = candidates
            .iter()
            .map(|candidate| candidate.score)
            .max()
            .unwrap_or(0);
        if type_score >= best_score.saturating_sub(conflict_penalty) {
            if let Some(name) = self.type_name(variable.ty()) {
                candidates.push(NameCandidate {
                    name,
                    score: type_score,
                });
            }
        }
        candidates
    }
}

pub(super) struct ParameterNameRecovery<'a> {
    model: StructuralNameModel<'a>,
}

impl<'a> ParameterNameRecovery<'a> {
    pub(super) fn new(types: &'a JavaTypeNameResolver) -> Self {
        Self {
            model: StructuralNameModel::new(types),
        }
    }

    pub(super) fn candidate(&self, ty: &ArgType) -> Option<JavaIdentifier> {
        self.model.type_name(ty)
    }
}

trait VariableNameSolver {
    fn solve(
        &self,
        graph: &VariableSemanticGraph,
        roles: &VariableRoleScores,
        reserved: &[JavaIdentifier],
        excluded: &BTreeSet<u32>,
    ) -> (BTreeMap<u32, JavaIdentifier>, bool);
}

struct ConstrainedNameSolver<'a, Model> {
    model: &'a Model,
    minimum_score: u16,
    conflict_penalty: u16,
}

impl<'a, Model> ConstrainedNameSolver<'a, Model> {
    fn new(model: &'a Model, minimum_score: u16) -> Self {
        Self {
            model,
            minimum_score,
            conflict_penalty: 20,
        }
    }
}

impl<Model: VariableNameModel> VariableNameSolver for ConstrainedNameSolver<'_, Model> {
    fn solve(
        &self,
        graph: &VariableSemanticGraph,
        roles: &VariableRoleScores,
        reserved: &[JavaIdentifier],
        excluded: &BTreeSet<u32>,
    ) -> (BTreeMap<u32, JavaIdentifier>, bool) {
        let mut needs_fallback = false;
        let mut proposals = graph
            .variables()
            .filter(|variable| {
                variable.is_source_binding() && !excluded.contains(&variable.identity())
            })
            .filter_map(|variable| {
                let mut candidates =
                    self.model
                        .candidates(variable, graph, roles, self.conflict_penalty);
                let Some(score) = candidates
                    .iter()
                    .map(|candidate| candidate.score)
                    .max()
                    .filter(|score| *score >= self.minimum_score)
                else {
                    needs_fallback = true;
                    return None;
                };
                let floor = self
                    .minimum_score
                    .max(score.saturating_sub(self.conflict_penalty));
                candidates.retain(|candidate| candidate.score >= floor);
                Some((variable.identity(), score, candidates))
            })
            .collect::<Vec<_>>();
        proposals.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));

        let mut used = JavaNameScope::default();
        for name in reserved {
            used.reserve(name.clone());
        }

        let mut solution = BTreeMap::new();
        for (variable, _, candidates) in proposals {
            let selected = candidates
                .iter()
                .map(|candidate| {
                    let conflicts = used.contains(&candidate.name);
                    let utility = candidate
                        .score
                        .saturating_sub(u16::from(conflicts) * self.conflict_penalty);
                    (candidate, utility, conflicts)
                })
                .max_by(|left, right| {
                    left.1
                        .cmp(&right.1)
                        .then_with(|| right.0.name.cmp(&left.0.name))
                })
                .map(|(candidate, _, conflicts)| {
                    if conflicts {
                        return used.claim_hint_variant(&candidate.name);
                    }
                    used.reserve(candidate.name.clone())
                })
                .unwrap_or_else(|| used.claim_hint_variant(&candidates[0].name));
            solution.insert(variable, selected);
        }
        (solution, needs_fallback)
    }
}

#[derive(Debug, Clone, Default)]
pub(super) struct LocalNameEvidence {
    intrinsic_names: BTreeMap<u32, JavaIdentifier>,
    has_local_binding: bool,
}

impl LocalNameEvidence {
    pub(super) fn observe_binding(
        &mut self,
        register: &crate::ir::RegisterArg,
        parameters: &[Option<u32>],
        receiver: Option<u32>,
    ) {
        if !self.has_local_binding {
            self.has_local_binding = register.code_var.is_some_and(|variable| {
                Some(variable) != receiver && !parameters.contains(&Some(variable))
            });
        }
    }

    pub(super) fn observe_operation(&mut self, operation: &crate::ir::SemanticOperation) {
        if let Some((variable, name)) = jadx_local_names::intrinsic_binding(operation) {
            self.intrinsic_names
                .entry(variable)
                .or_insert_with(|| JavaIdentifier::from_hint(&name));
        }
    }
}

pub(super) struct SemanticNameRecovery<'a> {
    types: &'a JavaTypeNameResolver,
}

impl<'a> SemanticNameRecovery<'a> {
    pub(super) fn new(types: &'a JavaTypeNameResolver) -> Self {
        Self { types }
    }

    pub(super) fn recover(
        &self,
        root: &SemanticNode,
        source_types: &std::sync::Arc<crate::ir::analysis::SourceTypeEnvironment>,
        evidence: LocalNameEvidence,
        resolved_types: &BTreeMap<ArgType, JavaType>,
        parameter_names: &[JavaIdentifier],
        parameter_variables: &[Option<u32>],
        this_variable: Option<u32>,
    ) -> BTreeMap<u32, JavaIdentifier> {
        let LocalNameEvidence {
            intrinsic_names,
            has_local_binding,
        } = evidence;
        if !has_local_binding {
            return intrinsic_names;
        }
        let mut excluded = parameter_variables
            .iter()
            .copied()
            .flatten()
            .chain(this_variable)
            .collect::<BTreeSet<_>>();
        excluded.extend(intrinsic_names.keys().copied());
        let graph = VariableSemanticGraph::analyze_shared(root, source_types);
        if !graph.variables().any(|variable| {
            variable.is_source_binding() && !excluded.contains(&variable.identity())
        }) {
            return intrinsic_names;
        }
        let roles = StructuralVariableRoleAnalysis.analyze(&graph);
        let model = StructuralNameModel::for_graph(self.types, resolved_types, &graph);
        let (mut names, needs_fallback) = ConstrainedNameSolver::new(&model, 35).solve(
            &graph,
            &roles,
            parameter_names,
            &excluded,
        );
        for (identity, name) in intrinsic_names {
            names.entry(identity).or_insert(name);
        }
        if needs_fallback {
            fill_remaining_source_names(
                &graph,
                &roles,
                &model,
                parameter_names,
                &excluded,
                &mut names,
            );
        }
        names
    }
}

fn java_jadx_type_name(ty: &JavaType) -> Option<String> {
    match ty {
        JavaType::Primitive(primitive) => {
            jadx_local_names::primitive_local_name(java_primitive(*primitive)).map(str::to_string)
        }
        JavaType::Class(class) => Some(jadx_local_names::class_local_name_from_segments(
            class.segments.iter().map(|segment| segment.name.as_str()),
        )),
        JavaType::Variable(name) => Some(jadx_local_names::class_local_name(name.as_str())),
        JavaType::Array(element) => Some(jadx_local_names::array_local_name(&java_jadx_type_name(
            element,
        )?)),
    }
}

fn java_primitive(primitive: JavaPrimitiveType) -> PrimitiveType {
    match primitive {
        JavaPrimitiveType::Void => PrimitiveType::Void,
        JavaPrimitiveType::Boolean => PrimitiveType::Boolean,
        JavaPrimitiveType::Byte => PrimitiveType::Byte,
        JavaPrimitiveType::Short => PrimitiveType::Short,
        JavaPrimitiveType::Char => PrimitiveType::Char,
        JavaPrimitiveType::Int => PrimitiveType::Int,
        JavaPrimitiveType::Long => PrimitiveType::Long,
        JavaPrimitiveType::Float => PrimitiveType::Float,
        JavaPrimitiveType::Double => PrimitiveType::Double,
    }
}

fn fill_remaining_source_names(
    graph: &VariableSemanticGraph,
    roles: &VariableRoleScores,
    model: &StructuralNameModel<'_>,
    reserved: &[JavaIdentifier],
    excluded: &BTreeSet<u32>,
    names: &mut BTreeMap<u32, JavaIdentifier>,
) {
    let mut used = JavaNameScope::default();
    for name in names.values().chain(reserved) {
        used.reserve(name.clone());
    }
    for variable in graph.variables() {
        if !variable.is_source_binding() || excluded.contains(&variable.identity()) {
            continue;
        }
        if names.contains_key(&variable.identity()) {
            continue;
        }
        let preferred = model
            .type_name(variable.ty())
            .or_else(|| {
                roles
                    .roles(variable.identity())
                    .max_by_key(|(_, score)| *score)
                    .filter(|(_, score)| *score > 0)
                    .map(|(role, _)| {
                        JavaIdentifier::from_hint(StructuralNameModel::role_name(role))
                    })
            })
            .unwrap_or_else(|| JavaIdentifier::from_hint("value"));
        let name = if used.contains(&preferred) {
            used.claim_hint_variant(&preferred)
        } else {
            used.reserve(preferred)
        };
        names.insert(variable.identity(), name);
    }
}

#[cfg(test)]
#[path = "../../../tests/internal/semantic_name_candidates.rs"]
mod candidate_tests;
