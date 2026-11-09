//! Which labels and scopes still need a printed name.

use super::super::build::{bug, filled};
use super::super::{
    BlockEnd, ControlCompletion, ControlKind, Delivery, ExpressionError, LabelId, MethodCode,
    TerminalKind, TransferId, TransferSpelling,
};
use crate::p2::BlockId;

pub(super) struct Selection {
    labels: Vec<bool>,
    controls: Vec<bool>,
}

impl Selection {
    pub(super) fn new(code: &MethodCode) -> Result<Self, ExpressionError> {
        let labels = filled(code.labels.len(), false)?;
        let controls = filled(code.controls.len(), false)?;
        Ok(Self { labels, controls })
    }

    pub(super) fn exception_routes(
        &mut self,
        code: &MethodCode,
        chains: &mut [bool],
    ) -> Result<(), ExpressionError> {
        for site in &code.sites {
            if !code.emission.site_annotations[site.id.0 as usize] {
                continue;
            }
            *chains
                .get_mut(site.chain.0 as usize)
                .ok_or(bug("site names an unknown exception chain"))? = true;
            for route in &site.routes {
                if let Delivery::Catch(control) = route.delivery {
                    *self
                        .controls
                        .get_mut(control.0 as usize)
                        .ok_or(bug("exception route names an unknown control"))? = true;
                }
            }
        }
        Ok(())
    }

    pub(super) fn select(mut self, code: &mut MethodCode) -> Result<(), ExpressionError> {
        for transfer in &code.emission.transfers {
            match transfer.spelling {
                TransferSpelling::Goto(label) => mark_label(&mut self.labels, label)?,
                TransferSpelling::Break(control) | TransferSpelling::Continue(control) => {
                    *self
                        .controls
                        .get_mut(control.0 as usize)
                        .ok_or(bug("transfer names an unknown control scope"))? = true;
                }
                TransferSpelling::BreakHere(_) | TransferSpelling::ContinueHere(_) => {}
                TransferSpelling::Fallthrough | TransferSpelling::Selected => {}
            }
        }
        for control in &code.controls {
            if let ControlKind::Graph { entries, .. } = &control.kind {
                for &entry in entries {
                    mark_label(&mut self.labels, entry)?;
                }
                if let Some(entry) = control.entry {
                    mark_label(&mut self.labels, entry)?;
                }
            }
            if let ControlKind::Switch { arms, .. } = &control.kind {
                for &edge in arms {
                    mark_selected_target(code, &mut self.labels, edge)?;
                }
            }
        }
        for chain in &code.chains {
            // Only a printed chain line names its handler targets; otherwise the
            // catch clause that prints the route carries no label text.
            if !code.emission.show_chains[chain.id.0 as usize] {
                continue;
            }
            for clause in &chain.clauses {
                if let Some(target) = clause.target {
                    mark_label(&mut self.labels, target)?;
                }
            }
        }
        for control in &code.controls {
            match control.continuation {
                Some(ControlCompletion::Goto(label)) => mark_label(&mut self.labels, label)?,
                Some(ControlCompletion::Break(scope) | ControlCompletion::Continue(scope)) => {
                    *self
                        .controls
                        .get_mut(scope.0 as usize)
                        .ok_or(bug("completion names an unknown scope"))? = true;
                }
                _ => {}
            }
        }
        for block in &code.emission.blocks {
            match &block.end {
                BlockEnd::Terminal(block) => mark_terminal_targets(code, &mut self.labels, *block)?,
                BlockEnd::None | BlockEnd::Transfers(_) => {}
            }
        }
        for (index, block) in code.emission.blocks.iter_mut().enumerate() {
            block.show_label = *self.labels.get(index).ok_or(bug("block lost its label"))?;
        }
        for (index, control) in code.controls.iter_mut().enumerate() {
            control.label_required = self.controls[index];
        }
        Ok(())
    }
}

fn mark_label(labels: &mut [bool], label: LabelId) -> Result<(), ExpressionError> {
    *labels
        .get_mut(label.0 as usize)
        .ok_or(bug("emission names an unknown label"))? = true;
    Ok(())
}

fn mark_selected_target(
    code: &MethodCode,
    labels: &mut [bool],
    edge: TransferId,
) -> Result<(), ExpressionError> {
    let original = code
        .transfers
        .get(edge.0 as usize)
        .ok_or(bug("terminal names an unknown transfer"))?;
    let planned = code
        .emission
        .transfers
        .get(edge.0 as usize)
        .ok_or(bug("terminal transfer lost its emission"))?;
    if matches!(planned.spelling, TransferSpelling::Selected) {
        mark_label(labels, original.destination)?;
    }
    Ok(())
}

fn mark_terminal_targets(
    code: &MethodCode,
    labels: &mut [bool],
    id: BlockId,
) -> Result<(), ExpressionError> {
    let terminal = code
        .terminals
        .get(id.0 as usize)
        .ok_or(bug("block lost its terminal"))?;
    match &terminal.kind {
        TerminalKind::If {
            when_true,
            when_false,
            ..
        } => {
            mark_selected_target(code, labels, *when_true)?;
            mark_selected_target(code, labels, *when_false)?;
        }
        TerminalKind::Switch { arms, .. } => {
            for &arm in arms {
                mark_selected_target(code, labels, arm)?;
            }
        }
        _ => {}
    }
    Ok(())
}
