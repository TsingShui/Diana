//! CFG structural invariant validation.

use std::collections::BTreeSet;

use crate::ir::{EdgeKind, InsnType, CFG};

use super::{Pass, PassResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CfgInvariantError {
    MissingEntry,
    InvalidMethodOwner(crate::ir::ArgType),
    InvalidInputRegisters {
        declared: u32,
        encoded: u32,
        registers: u32,
    },
    MissingSource(crate::ir::BlockId),
    MissingCoverage(crate::ir::BlockId),
    DuplicateEdges(crate::ir::BlockId),
    MissingTarget {
        source: crate::ir::BlockId,
        target: crate::ir::BlockId,
    },
    MalformedBranch(crate::ir::BlockId),
    MalformedGoto {
        block: crate::ir::BlockId,
        successors: usize,
    },
    TerminalSuccessor(crate::ir::BlockId),
    MissingSwitchDefault(crate::ir::BlockId),
    InconsistentSwitchDispatch(crate::ir::BlockId),
    LinearFanout {
        block: crate::ir::BlockId,
        successors: usize,
    },
    NonPrefixPhi(crate::ir::BlockId),
    InconsistentPhi(crate::ir::BlockId),
}

impl std::fmt::Display for CfgInvariantError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingEntry => formatter.write_str("entry block missing"),
            Self::InvalidMethodOwner(owner) => {
                write!(formatter, "method owner {owner} is not an object type")
            }
            Self::InvalidInputRegisters {
                declared,
                encoded,
                registers,
            } => write!(
                formatter,
                "method descriptor uses {declared} input words, DEX declares {encoded} in {registers} registers"
            ),
            Self::MissingSource(block) => write!(formatter, "edge source {block} has no block"),
            Self::MissingCoverage(block) => {
                write!(formatter, "block {block} has no exception coverage fact")
            }
            Self::DuplicateEdges(block) => write!(formatter, "block {block} has duplicate edges"),
            Self::MissingTarget { source, target } => {
                write!(formatter, "edge {source} -> {target} has no target block")
            }
            Self::MalformedBranch(block) => write!(
                formatter,
                "conditional block {block} does not have one true and one false edge"
            ),
            Self::MalformedGoto { block, successors } => write!(
                formatter,
                "goto block {block} has {successors} ordinary successors"
            ),
            Self::TerminalSuccessor(block) => {
                write!(
                    formatter,
                    "terminal block {block} has an ordinary successor"
                )
            }
            Self::MissingSwitchDefault(block) => {
                write!(formatter, "switch block {block} has no default target")
            }
            Self::InconsistentSwitchDispatch(block) => {
                write!(
                    formatter,
                    "switch block {block} edges disagree with its dispatch labels"
                )
            }
            Self::LinearFanout { block, successors } => write!(
                formatter,
                "linear block {block} has {successors} ordinary successors"
            ),
            Self::NonPrefixPhi(block) => write!(formatter, "block {block} has a non-prefix Phi"),
            Self::InconsistentPhi(block) => {
                write!(formatter, "block {block} has inconsistent Phi edges")
            }
        }
    }
}

impl std::error::Error for CfgInvariantError {}

#[derive(Debug, Default)]
pub struct ValidateCFG;

impl Pass for ValidateCFG {
    type Error = CfgInvariantError;

    fn name(&self) -> &'static str {
        "validate_cfg"
    }

    fn run(&mut self, cfg: &mut CFG) -> Result<PassResult, Self::Error> {
        cfg.capture_exception_coverage();
        if cfg.method().owner().as_object().is_none() {
            return Err(CfgInvariantError::InvalidMethodOwner(
                cfg.method().owner().clone(),
            ));
        }
        let declared_inputs = u32::from(!cfg.method().is_static())
            + cfg
                .method()
                .descriptor()
                .parameters
                .iter()
                .map(|ty| if ty.is_wide() { 2 } else { 1 })
                .sum::<u32>();
        if declared_inputs != cfg.ins || cfg.ins > cfg.registers {
            return Err(CfgInvariantError::InvalidInputRegisters {
                declared: declared_inputs,
                encoded: cfg.ins,
                registers: cfg.registers,
            });
        }
        if cfg.entry_block().is_none() {
            return Err(CfgInvariantError::MissingEntry);
        }
        for source in cfg.graph_nodes() {
            let Some(block) = cfg.block(source) else {
                return Err(CfgInvariantError::MissingSource(source));
            };
            if !cfg.has_exception_coverage(source) {
                return Err(CfgInvariantError::MissingCoverage(source));
            }
            let edges = cfg.successors_with_kind(source);
            let duplicate = match edges {
                [] | [_] => false,
                [first, second] => first == second,
                _ => edges.iter().copied().collect::<BTreeSet<_>>().len() != edges.len(),
            };
            if duplicate {
                return Err(CfgInvariantError::DuplicateEdges(source));
            }
            for (target, _) in edges {
                if cfg.block(*target).is_none() {
                    return Err(CfgInvariantError::MissingTarget {
                        source,
                        target: *target,
                    });
                }
            }

            let ordinary = edges
                .iter()
                .filter(|(_, kind)| *kind != EdgeKind::Exception);
            let ordinary_count = ordinary.clone().count();
            match block.terminator().map(|instruction| instruction.insn_type) {
                Some(InsnType::If) => {
                    if ordinary_count != 2
                        || !ordinary.clone().any(|(_, kind)| *kind == EdgeKind::True)
                        || !ordinary.clone().any(|(_, kind)| *kind == EdgeKind::False)
                    {
                        return Err(CfgInvariantError::MalformedBranch(source));
                    }
                }
                Some(InsnType::Goto) if ordinary_count != 1 => {
                    return Err(CfgInvariantError::MalformedGoto {
                        block: source,
                        successors: ordinary_count,
                    });
                }
                Some(InsnType::Return | InsnType::Throw) if ordinary_count != 0 => {
                    return Err(CfgInvariantError::TerminalSuccessor(source));
                }
                Some(InsnType::Switch) => {
                    let terminator = block
                        .terminator()
                        .ok_or(CfgInvariantError::MissingSwitchDefault(source))?;
                    if terminator.payload.switch_default.is_none() {
                        return Err(CfgInvariantError::MissingSwitchDefault(source));
                    }
                    let encoded = terminator
                        .get_switch_cases()
                        .map(Vec::as_slice)
                        .unwrap_or_default();
                    let encoded_values = encoded
                        .iter()
                        .map(|(value, _)| *value)
                        .collect::<BTreeSet<_>>();
                    let edge_values = ordinary
                        .clone()
                        .filter_map(|(_, kind)| match kind {
                            EdgeKind::SwitchCase(value) => Some(*value),
                            _ => None,
                        })
                        .collect::<BTreeSet<_>>();
                    let defaults = ordinary
                        .filter(|(_, kind)| *kind == EdgeKind::SwitchDefault)
                        .count();
                    if encoded.len() != encoded_values.len()
                        || encoded_values != edge_values
                        || defaults != 1
                        || ordinary_count != encoded.len() + 1
                    {
                        return Err(CfgInvariantError::InconsistentSwitchDispatch(source));
                    }
                }
                Some(InsnType::Goto | InsnType::Return | InsnType::Throw) => {}
                _ if ordinary_count > 1 => {
                    return Err(CfgInvariantError::LinearFanout {
                        block: source,
                        successors: ordinary_count,
                    });
                }
                _ => {}
            }

            let mut non_phi_seen = false;
            for instruction in &block.insns {
                if instruction.insn_type == InsnType::Phi {
                    if non_phi_seen {
                        return Err(CfgInvariantError::NonPrefixPhi(source));
                    }
                    let expected = cfg.predecessors_with_kind(source);
                    if instruction.payload.phi_edges.as_slice() != expected
                        || instruction.args.len() != expected.len()
                    {
                        return Err(CfgInvariantError::InconsistentPhi(source));
                    }
                } else {
                    non_phi_seen = true;
                }
            }
        }
        Ok(PassResult::Unchanged)
    }
}
