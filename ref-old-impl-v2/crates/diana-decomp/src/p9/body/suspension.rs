//! Presentation modes over delivered original facts and selected transfers.
use super::super::expressions::Expressions;
use super::super::{Origin, RenderError};
use crate::p7::{SourceMode, TransferId};

impl Expressions<'_, '_> {
    pub(super) fn source_mode(&mut self, mode: SourceMode) {
        match mode {
            SourceMode::Code => {}
            SourceMode::Abi => self.out.abi_comment = true,
            SourceMode::Origins => self.out.origin_only = true,
        }
    }
    pub(super) fn annotated_terminal(
        &mut self,
        terminal: crate::p2::BlockId,
        mode: SourceMode,
        normal: TransferId,
        suspended: TransferId,
    ) -> Result<(), RenderError> {
        let previous = (self.out.abi_comment, self.out.origin_only);
        self.source_mode(mode);
        let rendered = (|| {
            let source = &self.code.terminals()[terminal.0 as usize];
            let crate::p7::TerminalKind::If {
                condition,
                when_true,
                when_false,
            } = &source.kind
            else {
                return Err(RenderError::Invariant(
                    "annotated terminal lost its original branch",
                ));
            };
            let phases = &self.code.emission().terminals()[terminal.0 as usize];
            self.out.indent()?;
            let start = self.out.position();
            self.copies(&phases.before)?;
            self.out.put("if (")?;
            self.condition(condition, false)?;
            self.phases(&phases.snapshots, "after_operands_before_effect")?;
            self.out.put(") goto ")?;
            self.label_name(self.code.transfers()[when_true.0 as usize].destination)?;
            self.out.put("; else goto ")?;
            self.label_name(self.code.transfers()[when_false.0 as usize].destination)?;
            self.out.put(";\n")?;
            self.out.span(
                start,
                Origin::Terminal {
                    method: self.method,
                    block: terminal,
                },
            )?;
            self.out.span(
                start,
                Origin::Transfer {
                    method: self.method,
                    transfer: suspended,
                },
            )?;
            Ok(())
        })();
        self.out.abi_comment = previous.0;
        self.out.origin_only = previous.1;
        rendered?;
        self.transfer(normal)
    }
    pub(super) fn abi_operation(&mut self, operation: crate::p2::OpId) -> Result<(), RenderError> {
        let previous = std::mem::replace(&mut self.out.abi_comment, true);
        let rendered = self.statement(operation, None);
        self.out.abi_comment = previous;
        rendered
    }
}
