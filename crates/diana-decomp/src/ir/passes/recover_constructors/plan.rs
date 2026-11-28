use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::ir::{
    analysis::{
        InsnPosition, InstructionEffects, ObjectInitialization, SourceEvaluation, SsaValueGraph,
        SsaVar,
    },
    BlockId, EdgeKind, InsnArg, InsnNode, InsnType, MemberReference, MethodReference, CFG,
};

/// A construction is emitted as one allocation followed by ordered arguments
/// and the selected initializer. Only the covered operations may be removed.
pub(super) struct ConstructionPlan {
    pub(super) arguments: Option<Vec<InsnArg>>,
    pub(super) folded: BTreeSet<InsnPosition>,
    pub(super) exceptional_edges: Vec<(BlockId, BlockId)>,
}

pub(super) struct ConstructorFolding<'a> {
    cfg: &'a CFG,
    values: &'a SsaValueGraph,
    keeps_operands: Option<&'a dyn Fn(&MethodReference) -> bool>,
    multi_site_aliases: BTreeSet<SsaVar>,
}

impl<'a> ConstructorFolding<'a> {
    pub(super) fn new(
        cfg: &'a CFG,
        values: &'a SsaValueGraph,
        facts: &[ObjectInitialization],
        keeps_operands: Option<&'a dyn Fn(&MethodReference) -> bool>,
    ) -> Self {
        Self {
            cfg,
            values,
            keeps_operands,
            multi_site_aliases: facts
                .iter()
                .filter(|entry| entry.constructors.len() > 1)
                .flat_map(|entry| entry.aliases.iter().copied())
                .collect(),
        }
    }

    pub(super) fn plan(&self, initialization: &ObjectInitialization) -> Option<ConstructionPlan> {
        let allocation = initialization.allocation;
        let corridor = Corridor::collect(self.cfg, allocation, &initialization.constructors)?;
        let mut throwing = BTreeSet::from([allocation]);
        for &constructor in &initialization.constructors {
            if !self.same_exception_state(allocation, constructor) {
                return None;
            }
        }
        let mut effects = Vec::new();
        for &position in &corridor.instructions {
            let instruction = self.instruction(position)?;
            let effects_of_instruction = InstructionEffects::of_tree(instruction);
            if matches!(
                instruction.insn_type,
                InsnType::Goto | InsnType::If | InsnType::Switch
            ) {
                if !effects_of_instruction.without_control().is_pure() {
                    return None;
                }
            } else if !effects_of_instruction.is_pure() || instruction.can_throw() {
                if instruction.insn_type == InsnType::NewInstance
                    || effects_of_instruction.controls()
                    || matches!(instruction.payload.reference.as_deref(), Some(MemberReference::Method(method)) if method.is_constructor())
                {
                    return None;
                }
                if instruction.can_throw() {
                    if !self.same_exception_state(allocation, position) {
                        return None;
                    }
                    throwing.insert(position);
                }
                effects.push(position);
            }
        }
        let (arguments, folded) = if effects.is_empty() {
            (None, BTreeSet::new())
        } else {
            let [constructor] = initialization.constructors.as_slice() else {
                return None;
            };
            if !corridor.linear {
                return None;
            }
            let (arguments, folded) =
                self.fold_arguments(*constructor, &corridor.instructions, &effects)?;
            (Some(arguments), folded)
        };
        let exceptional_edges = throwing
            .into_iter()
            .filter(|position| {
                !initialization
                    .constructors
                    .iter()
                    .any(|ctor| ctor.block == position.block)
            })
            .flat_map(|position| {
                exceptional_targets(self.cfg, position.block)
                    .map(move |target| (position.block, target))
            })
            .collect();
        Some(ConstructionPlan {
            arguments,
            folded,
            exceptional_edges,
        })
    }

    fn same_exception_state(&self, left: InsnPosition, right: InsnPosition) -> bool {
        let (Some(left_insn), Some(right_insn)) = (self.instruction(left), self.instruction(right))
        else {
            return false;
        };
        let clauses = |offset| {
            self.cfg
                .handlers
                .iter()
                .filter(move |clause| clause.covers(offset))
                .map(|clause| (&clause.catch_type, clause.handler))
        };
        if !clauses(left_insn.offset).eq(clauses(right_insn.offset))
            || !exceptional_targets(self.cfg, left.block)
                .eq(exceptional_targets(self.cfg, right.block))
        {
            return false;
        }
        if left.block == right.block {
            return exceptional_targets(self.cfg, left.block).next().is_none();
        }
        for target in exceptional_targets(self.cfg, left.block) {
            let Some(block) = self.cfg.block(target) else {
                return false;
            };
            for phi in block
                .insns
                .iter()
                .take_while(|insn| insn.insn_type == InsnType::Phi)
            {
                let operand = |source| {
                    phi.payload
                        .phi_edges
                        .iter()
                        .zip(&phi.args)
                        .find_map(|(&(pred, kind), arg)| {
                            (pred == source && kind.is_exception()).then_some(arg)
                        })
                        .and_then(InsnArg::as_register)
                        .and_then(SsaVar::from_reg)
                };
                let (Some(left), Some(right)) = (operand(left.block), operand(right.block)) else {
                    return false;
                };
                if left != right {
                    return false;
                }
            }
        }
        true
    }

    fn instruction(&self, position: InsnPosition) -> Option<&InsnNode> {
        self.cfg.block(position.block)?.insns.get(position.index)
    }

    fn fold_arguments(
        &self,
        constructor: InsnPosition,
        instructions: &[InsnPosition],
        effects: &[InsnPosition],
    ) -> Option<(Vec<InsnArg>, BTreeSet<InsnPosition>)> {
        let definitions = instructions.iter().copied().collect::<BTreeSet<_>>();
        let constructor = self.instruction(constructor)?;
        let MemberReference::Method(reference) = constructor.payload.reference.as_deref()? else {
            return None;
        };
        if self
            .keeps_operands
            .is_some_and(|keeps_order| !keeps_order(reference))
        {
            return None;
        }
        let (receiver, arguments) = constructor.args.split_first()?;
        let mut tasks = arguments
            .iter()
            .rev()
            .map(|argument| Fold::Input(argument, 0))
            .collect::<Vec<_>>();
        let mut output = vec![receiver.clone()];
        let mut folded = BTreeSet::new();
        let mut evaluation = Vec::new();
        while let Some(task) = tasks.pop() {
            match task {
                Fold::Input(argument, depth) => {
                    let Some(register) = argument.as_register().and_then(SsaVar::from_reg) else {
                        // Decoded SSA arguments are registers or literals. A
                        // separate proof is needed for pre-existing trees.
                        if matches!(argument, InsnArg::Wrapped(_)) {
                            return None;
                        }
                        output.push(argument.clone());
                        continue;
                    };
                    if self.multi_site_aliases.contains(&register) {
                        return None;
                    }
                    let Some(position) = self
                        .values
                        .value(register)
                        .filter(|value| value.uses.len() == 1)
                        .and_then(|value| value.definition)
                        .filter(|position| definitions.contains(position))
                    else {
                        output.push(argument.clone());
                        continue;
                    };
                    let instruction = self.instruction(position)?;
                    if matches!(
                        instruction.insn_type,
                        InsnType::Phi | InsnType::MoveException | InsnType::Cmp
                    ) || instruction.payload.compound_target.is_some()
                        || !folded.insert(position)
                    {
                        return None;
                    }
                    // The recursive instruction representation is also read
                    // by renderers. Keep deeper inputs in explicit statements.
                    if depth == 64 {
                        return None;
                    }
                    let operands = SourceEvaluation::arguments(instruction).ok()?;
                    if !operands.iter().copied().eq(instruction.args.iter()) {
                        return None;
                    }
                    tasks.push(Fold::Instruction(position));
                    tasks.extend(
                        operands
                            .into_iter()
                            .rev()
                            .map(|arg| Fold::Input(arg, depth + 1)),
                    );
                }
                Fold::Instruction(position) => {
                    let mut instruction = self.instruction(position)?.clone();
                    if !InstructionEffects::of_tree(&instruction).is_pure()
                        || instruction.can_throw()
                    {
                        evaluation.push(position);
                    }
                    let start = output.len().checked_sub(instruction.args.len())?;
                    instruction.args = output.split_off(start);
                    if matches!(
                        instruction.insn_type,
                        InsnType::FilledNewArray | InsnType::StringConcat
                    ) && instruction.args.iter().any(|arg| {
                        matches!(arg, InsnArg::Wrapped(child)
                            if !InstructionEffects::of_tree(child).is_pure() || child.can_throw())
                    }) {
                        return None;
                    }
                    // The result now exists only inside this expression. Keep its
                    // type without advertising an unavailable SSA definition.
                    let result = instruction.result.as_mut()?;
                    if !result.ty.is_known() {
                        return None;
                    }
                    result.ssa_version = None;
                    result.code_var = None;
                    output.push(InsnArg::wrap(instruction));
                }
            }
        }
        (evaluation == effects).then_some((output, folded))
    }
}

fn exceptional_targets(cfg: &CFG, block: BlockId) -> impl Iterator<Item = BlockId> + '_ {
    cfg.successors_with_kind(block)
        .iter()
        .filter_map(|&(target, kind)| (kind == EdgeKind::Exception).then_some(target))
}

enum Fold<'a> {
    Input(&'a InsnArg, usize),
    Instruction(InsnPosition),
}

struct Corridor {
    instructions: Vec<InsnPosition>,
    linear: bool,
}

impl Corridor {
    fn collect(cfg: &CFG, allocation: InsnPosition, constructors: &[InsnPosition]) -> Option<Self> {
        let ends = constructors
            .iter()
            .map(|position| (position.block, position.index))
            .collect::<BTreeMap<_, _>>();
        if ends.len() != constructors.len() || ends.is_empty() {
            return None;
        }
        let mut work = VecDeque::from([allocation.block]);
        let mut successors = BTreeMap::<BlockId, Vec<BlockId>>::new();
        let mut instructions = Vec::new();
        let mut linear = true;
        while let Some(block) = work.pop_front() {
            if successors.contains_key(&block) {
                continue;
            }
            let body = cfg.block(block)?;
            let start = if block == allocation.block {
                allocation.index.checked_add(1)?
            } else {
                0
            };
            let end = ends.get(&block).copied().unwrap_or(body.insns.len());
            if start > end || end > body.insns.len() {
                return None;
            }
            instructions.extend((start..end).map(|index| InsnPosition { block, index }));
            let targets = if ends.contains_key(&block) {
                Vec::new()
            } else {
                let targets = cfg.normal_successors(block).collect::<Vec<_>>();
                if targets.is_empty() {
                    return None;
                }
                linear &= targets.len() == 1;
                targets
            };
            work.extend(targets.iter().copied());
            successors.insert(block, targets);
        }
        if !ends.keys().all(|end| successors.contains_key(end)) {
            return None;
        }
        // Every interior ingress belongs to this allocation. A DAG with no
        // other sinks reaches exactly one initializer on each normal path.
        let mut incoming = successors
            .keys()
            .map(|&block| (block, 0usize))
            .collect::<BTreeMap<_, _>>();
        for (&block, targets) in &successors {
            if block != allocation.block
                && cfg
                    .predecessors_with_kind(block)
                    .iter()
                    .any(|&(source, kind)| {
                        kind.is_exception()
                            || !successors
                                .get(&source)
                                .is_some_and(|targets| targets.contains(&block))
                    })
            {
                return None;
            }
            for target in targets {
                *incoming.get_mut(target)? += 1;
            }
        }
        let mut roots = VecDeque::from([allocation.block]);
        if incoming[&allocation.block] != 0 {
            return None;
        }
        let mut visited = 0;
        while let Some(block) = roots.pop_front() {
            visited += 1;
            for target in &successors[&block] {
                let count = incoming.get_mut(target)?;
                *count -= 1;
                if *count == 0 {
                    roots.push_back(*target);
                }
            }
        }
        (visited == successors.len()).then_some(Self {
            instructions,
            linear,
        })
    }
}
