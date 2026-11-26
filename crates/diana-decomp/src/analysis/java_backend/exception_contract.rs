use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::frontend::{ClassNode, MethodNode};
use crate::ir::analysis::{SubtypeRelation, TypeHierarchy};
use crate::ir::generic_types::{JvmTypeSignature, MethodSignature, TypeArgument};
use crate::ir::{ArgType, InsnType, MemberReference, MethodDescriptor, MethodReference, CFG};

use super::JavaSourceAbi;

/// A local candidate can acquire effects between two sites in declaration
/// order. Its ABI seeds apply only until its first sweep completes.
enum InvokeExceptions {
    Fixed(Vec<ArgType>),
    Local(Option<Vec<ArgType>>),
}

/// Recovers checked-exception contracts absent from DEX method metadata.
///
/// The analysis is a forward may-effect fixed point over the class-local call
/// graph. Declared source and classpath contracts are seeds; protected invokes
/// whose exception is accepted by a handler do not escape the method. This is
/// required for Java legalization because DEX itself does not enforce checked
/// exceptions, regardless of which frontend produced the bytecode.
pub(super) struct DeclaredExceptionAnalysis<'a> {
    abi: &'a JavaSourceAbi,
    hierarchy: &'a dyn TypeHierarchy,
}

impl<'a> DeclaredExceptionAnalysis<'a> {
    pub(super) fn new(abi: &'a JavaSourceAbi, hierarchy: &'a dyn TypeHierarchy) -> Self {
        Self { abi, hierarchy }
    }

    pub(super) fn solve<'b>(
        &self,
        class: &ClassNode,
        methods: impl IntoIterator<Item = (&'b MethodNode, &'b CFG)>,
    ) -> BTreeMap<MethodReference, Vec<ArgType>> {
        let function_object = super::FunctionObjectClass::analyze(class);
        let candidates = methods
            .into_iter()
            .filter(|(method, _)| {
                !method.is_class_init() && Self::declared(function_object, method).is_empty()
            })
            .map(|(method, cfg)| (Self::reference(class, method), cfg))
            .collect::<Vec<_>>();
        if candidates.is_empty() {
            return BTreeMap::new();
        }
        let mut contracts = class
            .methods()
            .iter()
            .filter_map(|method| {
                let exceptions = Self::declared(function_object, method);
                (!exceptions.is_empty()).then(|| {
                    (
                        Self::reference(class, method),
                        exceptions.iter().cloned().collect::<BTreeSet<_>>(),
                    )
                })
            })
            .collect::<BTreeMap<_, _>>();
        let candidate_refs = candidates
            .iter()
            .map(|(reference, _)| reference)
            .collect::<BTreeSet<_>>();
        let mut callers = BTreeMap::<&MethodReference, Vec<(usize, u32)>>::new();
        let mut invoked = foldhash::HashMap::default();
        let mut checked = BTreeMap::new();

        // Preserve the first sweep's declaration order. Before a local method
        // has contributed its first contract, its ABI is still a possible seed.
        for (caller, (reference, cfg)) in candidates.iter().enumerate() {
            let mut escapes = BTreeSet::new();
            for instruction in cfg.blocks.values().flat_map(|block| &block.insns) {
                if instruction.insn_type != InsnType::Invoke {
                    continue;
                }
                let Some(MemberReference::Method(method)) =
                    instruction.payload.reference.as_deref()
                else {
                    continue;
                };
                let add_exception = |exception: &ArgType| {
                    let is_checked = checked.get(exception).copied().unwrap_or_else(|| {
                        let result = self.is_checked(exception);
                        checked.insert(exception.clone(), result);
                        result
                    });
                    if is_checked && !self.handled(cfg, instruction.offset, exception) {
                        escapes.insert(exception.clone());
                    }
                };
                let contract = invoked.entry(method).or_insert_with(|| {
                    if let Some(exceptions) = self.generic_exceptions(method) {
                        return InvokeExceptions::Fixed(exceptions);
                    }
                    if &method.owner == class.class_type() {
                        if candidate_refs.contains(method) {
                            return InvokeExceptions::Local(None);
                        }
                        if let Some(declared) = contracts.get(method) {
                            return InvokeExceptions::Fixed(declared.iter().cloned().collect());
                        }
                    }
                    InvokeExceptions::Fixed(self.abi.method_exceptions(method))
                });
                match contract {
                    InvokeExceptions::Fixed(exceptions) => {
                        exceptions.iter().for_each(add_exception)
                    }
                    InvokeExceptions::Local(initial) => {
                        callers
                            .entry(method)
                            .or_default()
                            .push((caller, instruction.offset));
                        if let Some(local) = contracts.get(method) {
                            local.iter().for_each(add_exception);
                        } else {
                            initial
                                .get_or_insert_with(|| self.abi.method_exceptions(method))
                                .iter()
                                .for_each(add_exception);
                        }
                    }
                }
            }
            contracts
                .entry(reference.clone())
                .or_default()
                .extend(escapes);
        }

        // Only newly discovered effects can change a caller. Keeping each
        // invoke offset on the edge preserves its own ordered handler ranges.
        let mut pending = VecDeque::new();
        for (reference, exceptions) in &contracts {
            if let Some(sites) = callers.get(reference) {
                for exception in exceptions {
                    if *checked
                        .entry(exception.clone())
                        .or_insert_with(|| self.is_checked(exception))
                    {
                        pending.push_back((sites, exception.clone()));
                    }
                }
            }
        }
        while let Some((sites, exception)) = pending.pop_front() {
            for &(caller, offset) in sites {
                let (reference, cfg) = &candidates[caller];
                if !self.handled(cfg, offset, &exception)
                    && contracts
                        .get_mut(reference)
                        .expect("candidate contract initialized")
                        .insert(exception.clone())
                {
                    if let Some(sites) = callers.get(reference) {
                        pending.push_back((sites, exception.clone()));
                    }
                }
            }
        }

        contracts
            .into_iter()
            .filter(|(reference, exceptions)| {
                !exceptions.is_empty() && candidate_refs.contains(reference)
            })
            .map(|(reference, exceptions)| (reference, exceptions.into_iter().collect()))
            .collect()
    }

    fn generic_exceptions(&self, method: &MethodReference) -> Option<Vec<ArgType>> {
        let contract = self.abi.generic_method(method)?;
        let signature = &contract.signature;
        (!signature.throws.is_empty()).then(|| {
            signature
                .throws
                .iter()
                .filter(|exception| !Self::is_unconstrained_throws_variable(signature, exception))
                .map(JvmTypeSignature::erased)
                .collect()
        })
    }

    /// JLS 18.4 instantiates an inference variable used only in a throws
    /// bound as `RuntimeException` when no proper lower bound constrains it.
    /// Such a variable therefore contributes no checked effect at the call
    /// site even though its declaration erases to `Throwable`.
    fn is_unconstrained_throws_variable(
        signature: &MethodSignature,
        exception: &JvmTypeSignature,
    ) -> bool {
        let JvmTypeSignature::TypeVariable(name) = exception else {
            return false;
        };
        signature
            .type_parameters
            .iter()
            .any(|parameter| parameter.name == *name)
            && !signature
                .parameter_types
                .iter()
                .any(|parameter| Self::references_variable(parameter, name))
            && !Self::references_variable(&signature.return_type, name)
    }

    fn references_variable(ty: &JvmTypeSignature, name: &str) -> bool {
        match ty {
            JvmTypeSignature::TypeVariable(variable) => variable == name,
            JvmTypeSignature::Array(element) => Self::references_variable(element, name),
            JvmTypeSignature::ClassType(class) => class
                .type_arguments
                .iter()
                .chain(
                    class
                        .inner_segments
                        .iter()
                        .flat_map(|segment| &segment.type_arguments),
                )
                .any(|argument| match argument {
                    TypeArgument::Unbounded => false,
                    TypeArgument::Exact(ty)
                    | TypeArgument::Extends(ty)
                    | TypeArgument::Super(ty) => Self::references_variable(ty, name),
                }),
            JvmTypeSignature::BaseType(_) => false,
        }
    }

    fn handled(&self, cfg: &CFG, offset: u32, exception: &ArgType) -> bool {
        cfg.handlers.iter().any(|handler| {
            handler.covers(offset)
                && handler
                    .catch_type
                    .as_ref()
                    .is_none_or(|caught| self.is_subtype(exception, caught))
        })
    }

    fn is_checked(&self, exception: &ArgType) -> bool {
        self.is_subtype(exception, &ArgType::object("java/lang/Throwable"))
            && !self.is_subtype(exception, &ArgType::object("java/lang/RuntimeException"))
            && !self.is_subtype(exception, &ArgType::object("java/lang/Error"))
    }

    fn is_subtype(&self, value: &ArgType, expected: &ArgType) -> bool {
        match (value.as_object(), expected.as_object()) {
            (Some(value), Some(expected)) => {
                self.hierarchy.subtype_relation(value, expected) == SubtypeRelation::Yes
            }
            _ => false,
        }
    }

    fn declared(function_object: bool, method: &MethodNode) -> &[ArgType] {
        if !method.throws().is_empty() {
            return method.throws();
        }
        if method.access_flags.is_synthetic() || function_object {
            if let Some(semantics) = &method.override_semantics {
                return &semantics.inherited_throws;
            }
        }
        &[]
    }

    fn reference(class: &ClassNode, method: &MethodNode) -> MethodReference {
        MethodReference {
            owner: class.class_type().clone(),
            name: method.name().to_string(),
            descriptor: MethodDescriptor {
                parameters: method.param_types().to_vec(),
                return_type: method.return_type().clone(),
            },
        }
    }
}
