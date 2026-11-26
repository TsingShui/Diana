//! Bind DEX `move-result` pseudo-definitions to their instruction-stream producer.

use std::collections::BTreeMap;

use crate::ir::analysis::InsnPosition;
use crate::ir::{ArgType, BlockId, InsnNode, InsnType, MemberReference, RegisterArg, CFG};

use super::{Pass, PassResult};

#[derive(Debug, Clone)]
struct ResultBinding {
    producer: InsnPosition,
    pseudo: InsnPosition,
    value: RegisterArg,
}

struct ResultBindingAnalysis;

#[derive(Debug)]
pub enum ResultBindingError {
    MissingInstruction(InsnPosition),
    MissingProducer(u32),
    InvalidProducer { offset: u32, instruction: InsnType },
    NonLinearFallthrough(u32),
    NonAdjacent(u32),
    MissingDestination(u32),
    MissingReference(u32),
    InvalidReferenceKind(u32),
    VoidResult(u32),
    MissingBlock(BlockId),
}

impl std::fmt::Display for ResultBindingError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingInstruction(position) => {
                write!(formatter, "missing instruction at {position:?}")
            }
            Self::MissingProducer(offset) => {
                write!(formatter, "move-result at {offset} has no producer")
            }
            Self::InvalidProducer {
                offset,
                instruction,
            } => write!(
                formatter,
                "move-result at {offset} follows {instruction:?}, not a result producer"
            ),
            Self::NonLinearFallthrough(offset) => write!(
                formatter,
                "move-result at {offset} is not on the producer fallthrough edge"
            ),
            Self::NonAdjacent(offset) => {
                write!(
                    formatter,
                    "move-result at {offset} is not adjacent to its producer"
                )
            }
            Self::MissingDestination(offset) => {
                write!(formatter, "move-result at {offset} has no destination")
            }
            Self::MissingReference(offset) => {
                write!(
                    formatter,
                    "result producer at {offset} has no method reference"
                )
            }
            Self::InvalidReferenceKind(offset) => {
                write!(
                    formatter,
                    "result producer at {offset} has a non-method reference"
                )
            }
            Self::VoidResult(offset) => {
                write!(
                    formatter,
                    "void producer at {offset} is followed by move-result"
                )
            }
            Self::MissingBlock(block) => write!(formatter, "missing result block {block}"),
        }
    }
}

impl std::error::Error for ResultBindingError {}

impl ResultBindingAnalysis {
    fn analyze(cfg: &CFG) -> Result<Vec<ResultBinding>, ResultBindingError> {
        let instructions = || {
            cfg.blocks.values().flat_map(|block| {
                block.insns.iter().enumerate().map(move |(index, insn)| {
                    (
                        InsnPosition {
                            block: block.id,
                            index,
                        },
                        insn,
                    )
                })
            })
        };
        let mut previous_offset = 0;
        let mut has_results = false;
        let ordered = cfg.blocks.iter().all(|(&id, block)| {
            id == block.id
                && block.insns.iter().all(|insn| {
                    let ordered = previous_offset <= insn.offset;
                    previous_offset = insn.offset;
                    has_results |= insn.insn_type == InsnType::MoveResult;
                    ordered
                })
        });
        if ordered {
            if !has_results {
                return Ok(Vec::new());
            }
            return Self::bind_stream(cfg, instructions().map(Ok));
        }

        // CFG transformations may reorder blocks with respect to DEX offsets.
        // Keep the position tie-break and checked lookups for these inputs.
        let mut order: Vec<_> = instructions()
            .map(|(pos, insn)| (insn.offset, pos))
            .collect();
        order.sort_unstable_by_key(|(offset, pos)| (*offset, pos.block, pos.index));
        Self::bind_stream(
            cfg,
            order
                .into_iter()
                .map(|(_, pos)| Ok((pos, Self::instruction(cfg, pos)?))),
        )
    }

    fn bind_stream<'a>(
        cfg: &CFG,
        instructions: impl Iterator<Item = Result<(InsnPosition, &'a InsnNode), ResultBindingError>>,
    ) -> Result<Vec<ResultBinding>, ResultBindingError> {
        let mut bindings = Vec::new();
        let mut previous = None;
        for current in instructions {
            let (pseudo, instruction) = current?;
            let producer = previous.replace((pseudo, instruction));
            if instruction.insn_type != InsnType::MoveResult {
                continue;
            }
            let (producer, producer_instruction) =
                producer.ok_or(ResultBindingError::MissingProducer(instruction.offset))?;
            if !matches!(
                producer_instruction.insn_type,
                InsnType::Invoke | InsnType::FilledNewArray
            ) {
                return Err(ResultBindingError::InvalidProducer {
                    offset: instruction.offset,
                    instruction: producer_instruction.insn_type,
                });
            }
            if producer.block != pseudo.block {
                let linear =
                    cfg.successors_with_kind(producer.block)
                        .iter()
                        .any(|(target, kind)| {
                            *target == pseudo.block && *kind == crate::ir::EdgeKind::Normal
                        });
                if !linear {
                    return Err(ResultBindingError::NonLinearFallthrough(instruction.offset));
                }
            } else if producer.index + 1 != pseudo.index {
                return Err(ResultBindingError::NonAdjacent(instruction.offset));
            }
            let value = instruction
                .result
                .clone()
                .ok_or(ResultBindingError::MissingDestination(instruction.offset))?;
            bindings.push(ResultBinding {
                producer,
                pseudo,
                value,
            });
        }
        Ok(bindings)
    }

    fn instruction(cfg: &CFG, position: InsnPosition) -> Result<&InsnNode, ResultBindingError> {
        cfg.block(position.block)
            .and_then(|block| block.insns.get(position.index))
            .ok_or(ResultBindingError::MissingInstruction(position))
    }

    fn result_type(producer: &InsnNode) -> Result<ArgType, ResultBindingError> {
        match producer.insn_type {
            InsnType::Invoke => {
                let method = producer
                    .payload
                    .reference
                    .as_deref()
                    .ok_or(ResultBindingError::MissingReference(producer.offset))?;
                let MemberReference::Method(method) = method else {
                    return Err(ResultBindingError::InvalidReferenceKind(producer.offset));
                };
                if method.descriptor.return_type == ArgType::VOID {
                    return Err(ResultBindingError::VoidResult(producer.offset));
                }
                Ok(method.descriptor.return_type.clone())
            }
            InsnType::FilledNewArray => producer
                .payload
                .class_type
                .as_deref()
                .cloned()
                .ok_or(ResultBindingError::MissingReference(producer.offset)),
            _ => Err(ResultBindingError::InvalidProducer {
                offset: producer.offset,
                instruction: producer.insn_type,
            }),
        }
    }
}

#[derive(Debug, Default)]
pub struct BindResults;

impl Pass for BindResults {
    type Error = ResultBindingError;

    fn name(&self) -> &'static str {
        "bind_results"
    }

    fn run(&mut self, cfg: &mut CFG) -> Result<PassResult, Self::Error> {
        let bindings = ResultBindingAnalysis::analyze(cfg)?;
        if bindings.is_empty() {
            return Ok(PassResult::Unchanged);
        }

        let mut removals = BTreeMap::<BlockId, Vec<usize>>::new();
        for binding in bindings {
            let producer = cfg
                .block_mut(binding.producer.block)
                .and_then(|block| block.insns.get_mut(binding.producer.index))
                .ok_or(ResultBindingError::MissingInstruction(binding.producer))?;
            let mut result = binding.value;
            if !result.ty.is_known() {
                result.ty = ResultBindingAnalysis::result_type(producer)?;
            }
            producer.result = Some(result);
            removals
                .entry(binding.pseudo.block)
                .or_default()
                .push(binding.pseudo.index);
        }
        for (block, mut indices) in removals {
            indices.sort_unstable();
            let instructions = &mut cfg
                .block_mut(block)
                .ok_or(ResultBindingError::MissingBlock(block))?
                .insns;
            // Valid positions are unique. Move every surviving instruction at
            // most once even in a block containing thousands of invoke results.
            if indices.len() > 1
                && indices
                    .last()
                    .is_some_and(|last| *last < instructions.len())
                && indices.windows(2).all(|pair| pair[0] != pair[1])
            {
                let mut removed = indices.into_iter().peekable();
                let mut index = 0;
                instructions.retain(|_| {
                    let keep = removed.next_if_eq(&index).is_none();
                    index += 1;
                    keep
                });
                continue;
            }
            for index in indices.into_iter().rev() {
                if index >= instructions.len() {
                    return Err(ResultBindingError::MissingInstruction(InsnPosition {
                        block,
                        index,
                    }));
                }
                instructions.remove(index);
            }
        }
        Ok(PassResult::Changed)
    }
}
