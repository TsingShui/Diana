use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::ir::{analysis::SsaVar, ArgType, PrimitiveType};

use super::{
    bounds::{BoundKind, TypeBound, TypeBounds},
    constraints::{ArrayConstraint, NormalizedTypeConstraints},
    hierarchy::{SubtypeRelation, TypeHierarchy},
};

impl TypeBounds {
    fn produced_type(&self, hierarchy: &dyn TypeHierarchy) -> Option<ArgType> {
        let exact = self.evidence(BoundKind::Exact);
        let evidence = if exact.clone().next().is_some() {
            exact
        } else {
            self.evidence(BoundKind::Lower)
        };
        TypeLattice::join_all(evidence.cloned(), hierarchy)
    }
}

pub(super) struct TypeLattice;

impl TypeLattice {
    pub(super) fn solve(
        mut constraints: NormalizedTypeConstraints,
        hierarchy: &dyn TypeHierarchy,
    ) -> super::SsaTypeEnvironment {
        let mut states = std::mem::take(&mut constraints.bounds);
        if !(constraints.flows.is_empty()
            && constraints.upper_flows.is_empty()
            && constraints.arrays.is_empty())
        {
            // Propagation may introduce an unobserved relation endpoint. Keep it
            // available to transitive constraints, then exclude it from the result.
            let observed = if let Some(members) = &constraints.members {
                for &root in members.values() {
                    states.entry(root).or_default();
                }
                None
            } else {
                Some(states.keys().copied().collect::<Vec<_>>())
            };
            Self::propagate(&constraints, &mut states, hierarchy);
            if let Some(observed) = observed {
                if states.len() != observed.len() {
                    states.retain(|value, _| observed.binary_search(value).is_ok());
                }
            }
        }

        let resolved = states.into_iter().filter_map(|(root, state)| {
            Self::resolve_state(&state, hierarchy).map(|ty| (root, ty))
        });
        // Singleton components already arrive in register/version order.
        let Some(members) = constraints.members else {
            return super::SsaTypeEnvironment::from_sorted(resolved.collect());
        };
        let mut types = resolved.collect::<BTreeMap<_, _>>();
        for (&value, &root) in &members {
            if value != root {
                if let Some(ty) = types.get(&root) {
                    types.insert(value, ty.clone());
                }
            }
        }
        super::SsaTypeEnvironment::from_sorted(
            types
                .into_iter()
                .filter(|(value, _)| members.contains_key(value))
                .collect(),
        )
    }

    fn propagate(
        constraints: &NormalizedTypeConstraints,
        states: &mut BTreeMap<SsaVar, TypeBounds>,
        hierarchy: &dyn TypeHierarchy,
    ) {
        let mut outgoing = BTreeMap::<SsaVar, BTreeSet<SsaVar>>::new();
        let mut incoming = BTreeMap::<SsaVar, BTreeSet<SsaVar>>::new();
        for (source, target) in &constraints.flows {
            outgoing.entry(*source).or_default().insert(*target);
            incoming.entry(*target).or_default().insert(*source);
        }
        for (source, target) in &constraints.upper_flows {
            incoming.entry(*target).or_default().insert(*source);
        }
        // Keep each component's constraints in the same order as the original
        // array relation. A self relation contributes its array side only.
        let mut arrays = BTreeMap::<SsaVar, Vec<(SsaVar, BoundKind)>>::new();
        for constraint in &constraints.arrays {
            let (array, value, bound) = match *constraint {
                ArrayConstraint::Read { array, result } => (array, result, BoundKind::Exact),
                ArrayConstraint::Write { array, value } => (array, value, BoundKind::Upper),
            };
            arrays.entry(array).or_default().push((value, bound));
            if value != array {
                arrays
                    .entry(value)
                    .or_default()
                    .push((array, BoundKind::Lower));
            }
        }

        let mut pending = states.keys().copied().collect::<VecDeque<_>>();
        let mut queued = states.keys().copied().collect::<foldhash::HashSet<_>>();
        let mut fallback = Vec::new();
        let mut upper = Vec::new();
        while let Some(root) = pending.pop_front() {
            queued.remove(&root);
            let outgoing = outgoing.get(&root);
            let incoming = incoming.get(&root);
            let arrays = arrays.get(&root);
            if outgoing.is_none() && incoming.is_none() && arrays.is_none() {
                continue;
            }
            let state = &states[&root];
            let produced = (outgoing.is_some() || arrays.is_some())
                .then(|| state.produced_type(hierarchy))
                .flatten();
            let array_element = arrays.and_then(|_| match &produced {
                Some(ArgType::Array(element)) => Some((**element).clone()),
                Some(_) => None,
                None => match Self::most_specific_upper(state, hierarchy)? {
                    ArgType::Array(element) => Some(std::sync::Arc::unwrap_or_clone(element)),
                    _ => None,
                },
            });
            fallback.clear();
            upper.clear();
            if outgoing.is_some() {
                fallback.extend(state.evidence(BoundKind::Fallback).cloned());
            }
            if incoming.is_some() {
                upper.extend(
                    state
                        .evidence(BoundKind::Upper)
                        .chain(state.evidence(BoundKind::Exact))
                        .cloned(),
                );
            }

            if let Some(produced) = &produced {
                for &target in outgoing.into_iter().flatten() {
                    Self::insert_bound(
                        states,
                        target,
                        BoundKind::Lower,
                        produced.clone(),
                        &mut queued,
                        &mut pending,
                    );
                }
            }
            for target in outgoing.into_iter().flatten() {
                let state = states.entry(*target).or_default();
                let mut changed = false;
                for ty in &fallback {
                    changed |= state.insert(TypeBound::new(BoundKind::Fallback, ty.clone()));
                }
                if changed && queued.insert(*target) {
                    pending.push_back(*target);
                }
            }
            for source in incoming.into_iter().flatten() {
                let state = states.entry(*source).or_default();
                let mut changed = false;
                for ty in &upper {
                    changed |= state.insert(TypeBound::new(BoundKind::Upper, ty.clone()));
                }
                if changed && queued.insert(*source) {
                    pending.push_back(*source);
                }
            }
            for &(target, kind) in arrays.into_iter().flatten() {
                let ty = match kind {
                    BoundKind::Lower => produced
                        .as_ref()
                        .filter(|ty| Self::is_concrete(ty))
                        .map(|ty| ArgType::array(ty.clone())),
                    _ => array_element.clone(),
                };
                if let Some(ty) = ty {
                    Self::insert_bound(states, target, kind, ty, &mut queued, &mut pending);
                }
            }
        }
    }

    fn insert_bound(
        states: &mut BTreeMap<SsaVar, TypeBounds>,
        value: SsaVar,
        kind: BoundKind,
        ty: ArgType,
        queued: &mut foldhash::HashSet<SsaVar>,
        pending: &mut VecDeque<SsaVar>,
    ) {
        let state = states.entry(value).or_default();
        let changed = state.insert(TypeBound::new(kind, ty));
        if changed && queued.insert(value) {
            pending.push_back(value);
        }
    }

    fn resolve_state(state: &TypeBounds, hierarchy: &dyn TypeHierarchy) -> Option<ArgType> {
        let strong = state.produced_type(hierarchy);
        if let Some(ty) = strong.as_ref().filter(|ty| Self::is_concrete(ty)) {
            if state
                .evidence(BoundKind::Upper)
                .all(|upper| Self::is_assignable(ty, upper, hierarchy))
            {
                return Some(ty.clone());
            }
        }

        let upper = Self::most_specific_upper(state, hierarchy);
        if let Some(candidate) = upper.as_ref() {
            let exact_satisfies = state
                .evidence(BoundKind::Exact)
                .all(|bound| Self::producer_bound_compatible(bound, candidate, hierarchy));
            let lower_satisfies = state
                .evidence(BoundKind::Lower)
                .all(|bound| Self::producer_bound_compatible(bound, candidate, hierarchy));
            if exact_satisfies && lower_satisfies && Self::domain_accepts(state, candidate) {
                return Some(candidate.clone());
            }
        }
        if let Some(ty) = strong.as_ref().filter(|ty| Self::is_concrete(ty)) {
            if state
                .evidence(BoundKind::Upper)
                .all(|upper| Self::producer_bound_compatible(ty, upper, hierarchy))
                && Self::domain_accepts(state, ty)
            {
                return Some(ty.clone());
            }
        }
        if let Some(candidate) =
            Self::join_all(state.evidence(BoundKind::Fallback).cloned(), hierarchy)
        {
            if Self::is_concrete(&candidate)
                && state
                    .evidence(BoundKind::Upper)
                    .all(|upper| Self::is_assignable(&candidate, upper, hierarchy))
                && Self::domain_accepts(state, &candidate)
            {
                return Some(candidate);
            }
        }
        let domain = state
            .evidence(BoundKind::Domain)
            .filter_map(Self::concrete_hint)
            .collect::<BTreeSet<_>>();
        let mut candidates = domain.into_iter();
        let candidate = candidates.next()?;
        if candidates.next().is_some() {
            return None;
        }
        state
            .evidence(BoundKind::Upper)
            .all(|upper| Self::is_assignable(&candidate, upper, hierarchy))
            .then_some(candidate)
    }

    fn most_specific_upper(state: &TypeBounds, hierarchy: &dyn TypeHierarchy) -> Option<ArgType> {
        state
            .evidence(BoundKind::Upper)
            .filter_map(Self::concrete_hint)
            .filter(|candidate| {
                state
                    .evidence(BoundKind::Upper)
                    .all(|other| Self::is_assignable(candidate, other, hierarchy))
            })
            .min()
    }

    fn concrete_hint(ty: &ArgType) -> Option<ArgType> {
        match ty {
            ArgType::Unknown(categories) => {
                let [category] = categories.as_ref() else {
                    return None;
                };
                match category {
                    PrimitiveType::Object => Some(ArgType::object("java/lang/Object")),
                    PrimitiveType::Array => None,
                    primitive => Some(ArgType::Primitive(*primitive)),
                }
            }
            ty if Self::is_concrete(ty) => Some(ty.clone()),
            _ => None,
        }
    }

    fn is_concrete(ty: &ArgType) -> bool {
        match ty {
            ArgType::Primitive(PrimitiveType::Object | PrimitiveType::Array) => false,
            ArgType::Primitive(_) | ArgType::Object(_) => true,
            ArgType::Array(element) => Self::is_concrete(element),
            ArgType::Unknown(_) => false,
        }
    }

    fn is_assignable(value: &ArgType, expected: &ArgType, hierarchy: &dyn TypeHierarchy) -> bool {
        if value == expected {
            return true;
        }
        match (value, expected) {
            (_, ArgType::Unknown(categories)) => Self::unknown_accepts(categories, value),
            (ArgType::Unknown(categories), known) => Self::unknown_accepts(categories, known),
            (ArgType::Object(value), ArgType::Object(expected)) => {
                hierarchy.subtype_relation(value, expected) != SubtypeRelation::No
            }
            (ArgType::Array(value), ArgType::Array(expected)) => {
                match (value.as_ref(), expected.as_ref()) {
                    (ArgType::Primitive(value), ArgType::Primitive(expected)) => value == expected,
                    (ArgType::Primitive(_), _) | (_, ArgType::Primitive(_)) => false,
                    (value, expected) => Self::is_assignable(value, expected, hierarchy),
                }
            }
            (ArgType::Array(_), ArgType::Object(expected)) => {
                matches!(
                    expected.as_str(),
                    "java/lang/Object" | "java/lang/Cloneable" | "java/io/Serializable"
                )
            }
            (ArgType::Primitive(value), ArgType::Primitive(expected)) => {
                Self::primitive_assignable(*value, *expected)
            }
            _ => false,
        }
    }

    /// Reference producers can be broader than a consumer's required type:
    /// Java lowering preserves that physical type and inserts a cast or uses a
    /// generic source projection when the emitted expression is not directly
    /// assignable. Primitive values cannot use that recovery and must remain
    /// directly assignable.
    fn producer_bound_compatible(
        produced: &ArgType,
        expected: &ArgType,
        hierarchy: &dyn TypeHierarchy,
    ) -> bool {
        if produced.is_reference() && expected.is_reference() {
            return true;
        }
        Self::is_assignable(produced, expected, hierarchy)
    }

    fn primitive_assignable(value: PrimitiveType, expected: PrimitiveType) -> bool {
        use PrimitiveType::{Byte, Char, Double, Float, Int, Long, Short};
        matches!(
            (value, expected),
            (Byte, Short | Int | Long | Float | Double)
                | (Short | Char, Int | Long | Float | Double)
                | (Int, Long | Float | Double)
                | (Long, Float | Double)
                | (Float, Double)
        )
    }

    fn domain_accepts(state: &TypeBounds, candidate: &ArgType) -> bool {
        let mut domain = state.evidence(BoundKind::Domain).peekable();
        domain.peek().is_none()
            || domain.any(|observed| match observed {
                ArgType::Unknown(categories) => Self::unknown_accepts(categories, candidate),
                known => known == candidate || Self::same_register_category(known, candidate),
            })
    }

    fn same_register_category(left: &ArgType, right: &ArgType) -> bool {
        match (left, right) {
            (ArgType::Primitive(left), ArgType::Primitive(right)) => {
                left.is_wide() == right.is_wide()
            }
            (ArgType::Object(_) | ArgType::Array(_), ArgType::Object(_) | ArgType::Array(_)) => {
                true
            }
            _ => false,
        }
    }

    fn join_all(
        values: impl IntoIterator<Item = ArgType>,
        hierarchy: &dyn TypeHierarchy,
    ) -> Option<ArgType> {
        let mut values = values.into_iter().collect::<Vec<_>>();
        values.sort();
        let first = values.first()?.clone();
        Some(
            values
                .into_iter()
                .skip(1)
                .fold(first, |left, right| Self::join(left, right, hierarchy)),
        )
    }

    fn join(left: ArgType, right: ArgType, hierarchy: &dyn TypeHierarchy) -> ArgType {
        if left == right {
            return left;
        }
        match (left, right) {
            (ArgType::Unknown(left), ArgType::Unknown(right)) => {
                // The membership test does not reorder or deduplicate the left
                // operand. Reuse its representation when every member survives.
                let accepted = right.iter().fold(0u16, |mask, ty| mask | (1 << *ty as u8));
                let keep = |ty: &PrimitiveType| accepted & (1 << *ty as u8) != 0;
                if left.iter().all(keep) {
                    return if left.is_empty() {
                        ArgType::unknown()
                    } else {
                        ArgType::Unknown(left)
                    };
                }
                let intersection: crate::ir::TypeCandidates =
                    left.iter().copied().filter(keep).collect();
                if intersection.is_empty() {
                    ArgType::unknown()
                } else {
                    ArgType::Unknown(intersection)
                }
            }
            (known, ArgType::Unknown(candidates)) | (ArgType::Unknown(candidates), known) => {
                if Self::unknown_accepts(&candidates, &known) {
                    known
                } else {
                    Self::unknown_union(&known, &ArgType::Unknown(candidates))
                }
            }
            (ArgType::Object(left), ArgType::Object(right)) => hierarchy
                .least_common_supertype(&left, &right)
                .map(|name| ArgType::object(&name))
                .unwrap_or_else(|| ArgType::object("java/lang/Object")),
            (ArgType::Array(left), ArgType::Array(right)) => {
                match (left.as_ref(), right.as_ref()) {
                    (ArgType::Primitive(left), ArgType::Primitive(right)) if left != right => {
                        ArgType::object("java/lang/Object")
                    }
                    (ArgType::Primitive(_), _) | (_, ArgType::Primitive(_)) => {
                        ArgType::object("java/lang/Object")
                    }
                    _ => ArgType::array(Self::join(
                        std::sync::Arc::unwrap_or_clone(left),
                        std::sync::Arc::unwrap_or_clone(right),
                        hierarchy,
                    )),
                }
            }
            (ArgType::Array(_), ArgType::Object(object))
            | (ArgType::Object(object), ArgType::Array(_))
                if object == "java/lang/Object" =>
            {
                ArgType::object("java/lang/Object")
            }
            (ArgType::Primitive(left), ArgType::Primitive(right)) => {
                Self::join_primitive(left, right)
            }
            (left, right) => Self::unknown_union(&left, &right),
        }
    }

    fn join_primitive(left: PrimitiveType, right: PrimitiveType) -> ArgType {
        use PrimitiveType::{Boolean, Byte, Char, Double, Float, Int, Long, Short};
        match (left, right) {
            (Boolean, Int) | (Int, Boolean) => ArgType::Unknown([Boolean, Int].into()),
            (Byte | Short | Char | Int, Byte | Short | Char | Int) => ArgType::INT,
            (Long, Byte | Short | Char | Int) | (Byte | Short | Char | Int, Long) => ArgType::LONG,
            (Float, Byte | Short | Char | Int) | (Byte | Short | Char | Int, Float) => {
                ArgType::FLOAT
            }
            (Double, Float | Long | Byte | Short | Char | Int)
            | (Float | Long | Byte | Short | Char | Int, Double) => ArgType::DOUBLE,
            (left, right) => ArgType::Unknown(vec![left, right].into()),
        }
    }

    fn unknown_accepts(categories: &[PrimitiveType], known: &ArgType) -> bool {
        match known {
            ArgType::Primitive(primitive) => categories.contains(primitive),
            ArgType::Object(_) => categories.contains(&PrimitiveType::Object),
            ArgType::Array(_) => categories.contains(&PrimitiveType::Array),
            ArgType::Unknown(other) => other.iter().all(|candidate| categories.contains(candidate)),
        }
    }

    fn unknown_union(left: &ArgType, right: &ArgType) -> ArgType {
        let mut categories = Self::categories(left);
        categories.extend(Self::categories(right));
        ArgType::Unknown(categories.into_iter().collect())
    }

    fn categories(ty: &ArgType) -> BTreeSet<PrimitiveType> {
        match ty {
            ArgType::Primitive(primitive) => BTreeSet::from([*primitive]),
            ArgType::Object(_) => BTreeSet::from([PrimitiveType::Object]),
            ArgType::Array(_) => BTreeSet::from([PrimitiveType::Array]),
            ArgType::Unknown(categories) => categories.iter().copied().collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::analysis::ClassHierarchyIndex;

    #[test]
    fn exact_consumer_type_constrains_flow_sources() {
        let source = SsaVar::new(2, 5);
        let target = SsaVar::new(2, 7);
        let constraints = NormalizedTypeConstraints {
            members: Some(BTreeMap::from([(source, source), (target, target)])),
            bounds: BTreeMap::from([
                (
                    source,
                    TypeBounds::from([
                        TypeBound::new(
                            BoundKind::Domain,
                            ArgType::Unknown([PrimitiveType::Int, PrimitiveType::Boolean].into()),
                        ),
                        TypeBound::new(BoundKind::Fallback, ArgType::INT),
                    ]),
                ),
                (
                    target,
                    TypeBounds::from([TypeBound::new(BoundKind::Exact, ArgType::BOOLEAN)]),
                ),
            ]),
            flows: BTreeSet::from([(source, target)]),
            upper_flows: BTreeSet::new(),
            arrays: BTreeSet::new(),
        };

        let resolved = TypeLattice::solve(constraints, &ClassHierarchyIndex::default());

        assert_eq!(resolved.value_type(source), Some(&ArgType::BOOLEAN));
        assert_eq!(resolved.value_type(target), Some(&ArgType::BOOLEAN));
    }

    #[test]
    fn array_write_propagates_the_element_type_to_an_unknown_array() {
        let array = SsaVar::new(0, 0);
        let value = SsaVar::new(1, 0);
        let value_type = ArgType::object("android/content/pm/ActivityInfo");
        let constraints = NormalizedTypeConstraints {
            members: Some(BTreeMap::from([(array, array), (value, value)])),
            bounds: BTreeMap::from([
                (
                    array,
                    TypeBounds::from([TypeBound::new(
                        BoundKind::Domain,
                        ArgType::unknown_object(),
                    )]),
                ),
                (
                    value,
                    TypeBounds::from([TypeBound::new(BoundKind::Exact, value_type.clone())]),
                ),
            ]),
            flows: BTreeSet::new(),
            upper_flows: BTreeSet::new(),
            arrays: BTreeSet::from([ArrayConstraint::Write { array, value }]),
        };

        let resolved = TypeLattice::solve(constraints, &ClassHierarchyIndex::default());

        assert_eq!(
            resolved.value_type(array),
            Some(&ArgType::array(value_type))
        );
    }

    #[test]
    fn array_copy_uses_the_narrow_destination_element_type() {
        let source = SsaVar::new(0, 0);
        let destination = SsaVar::new(1, 0);
        let value = SsaVar::new(2, 0);
        let element = ArgType::object("androidx/compose/ui/node/LayoutNode");
        let constraints = NormalizedTypeConstraints {
            members: Some(BTreeMap::from([
                (source, source),
                (destination, destination),
                (value, value),
            ])),
            bounds: BTreeMap::from([
                (
                    source,
                    TypeBounds::from([TypeBound::new(
                        BoundKind::Exact,
                        ArgType::array(ArgType::object("java/lang/Object")),
                    )]),
                ),
                (
                    destination,
                    TypeBounds::from([
                        TypeBound::new(BoundKind::Lower, ArgType::array(element.clone())),
                        TypeBound::new(BoundKind::Upper, ArgType::array(element.clone())),
                    ]),
                ),
                (
                    value,
                    TypeBounds::from([TypeBound::new(
                        BoundKind::Domain,
                        ArgType::unknown_object(),
                    )]),
                ),
            ]),
            flows: BTreeSet::new(),
            upper_flows: BTreeSet::new(),
            arrays: BTreeSet::from([
                ArrayConstraint::Read {
                    array: source,
                    result: value,
                },
                ArrayConstraint::Write {
                    array: destination,
                    value,
                },
            ]),
        };

        let mut hierarchy = ClassHierarchyIndex::default();
        hierarchy.add("java/lang/Object", Vec::new());
        hierarchy.add(
            "androidx/compose/ui/node/LayoutNode",
            vec!["java/lang/Object".to_string()],
        );
        let resolved = TypeLattice::solve(constraints, &hierarchy);

        assert_eq!(resolved.value_type(value), Some(&element));
    }

    #[test]
    fn incompatible_primitive_producer_does_not_use_the_consumer_type() {
        let value = SsaVar::new(0, 0);
        let constraints = NormalizedTypeConstraints {
            members: Some(BTreeMap::from([(value, value)])),
            bounds: BTreeMap::from([(
                value,
                TypeBounds::from([
                    TypeBound::new(BoundKind::Exact, ArgType::BOOLEAN),
                    TypeBound::new(BoundKind::Upper, ArgType::INT),
                ]),
            )]),
            flows: BTreeSet::new(),
            upper_flows: BTreeSet::new(),
            arrays: BTreeSet::new(),
        };

        let resolved = TypeLattice::solve(constraints, &ClassHierarchyIndex::default());

        assert_eq!(resolved.value_type(value), None);
    }

    #[test]
    fn reference_producer_survives_incomparable_consumer_types() {
        let value = SsaVar::new(4, 11);
        let view = ArgType::object("android/view/View");
        let callback = ArgType::object("com/example/SelectionCallback");
        let constraints = NormalizedTypeConstraints {
            members: Some(BTreeMap::from([(value, value)])),
            bounds: BTreeMap::from([(
                value,
                TypeBounds::from([
                    TypeBound::new(BoundKind::Lower, view.clone()),
                    TypeBound::new(BoundKind::Upper, view.clone()),
                    TypeBound::new(BoundKind::Upper, callback),
                    TypeBound::new(BoundKind::Domain, ArgType::unknown_object()),
                ]),
            )]),
            flows: BTreeSet::new(),
            upper_flows: BTreeSet::new(),
            arrays: BTreeSet::new(),
        };

        let mut hierarchy = ClassHierarchyIndex::default();
        hierarchy.add("java/lang/Object", Vec::new());
        hierarchy.add("android/view/View", vec!["java/lang/Object".to_string()]);
        hierarchy.add(
            "com/example/SelectionCallback",
            vec!["java/lang/Object".to_string()],
        );
        let resolved = TypeLattice::solve(constraints, &hierarchy);

        assert_eq!(resolved.value_type(value), Some(&view));
    }
}

#[cfg(test)]
#[path = "../../../../tests/internal/type_domains.rs"]
mod type_domains;

#[cfg(test)]
#[path = "../../../../tests/internal/singleton_type_roots.rs"]
mod singleton_type_roots;
