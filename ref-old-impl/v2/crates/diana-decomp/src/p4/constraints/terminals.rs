//! Terminal read requirements and pair-equality compatibility in original order.

use super::*;

pub(super) fn collect(
    ssa: &SsaMethod<'_>,
    f: &mut TypeFacts,
    work: &mut Work,
    returned: TypeNameId,
) -> Result<(), TypeError> {
    let s = ssa.facts();
    let flow = ssa.execution();
    for block in flow.blocks() {
        let Some(uses) = s.terminal_uses(block.id()) else {
            continue;
        };
        for (offset, _) in uses.iter().enumerate() {
            let index = s
                .terminal_site(block.id())
                .ok_or(TypeError::InternalInvariant("terminal view missing"))?
                .use_start() as usize
                + offset;
            let families = match block.terminal() {
                Some(Terminal::Branch {
                    op: BranchOp::Eq | BranchOp::Ne | BranchOp::Eqz | BranchOp::Nez,
                    ..
                }) => Families::INT_REF,
                Some(Terminal::Branch { .. } | Terminal::Switch { .. }) => mask(Family::Integral),
                Some(Terminal::Throw { .. }) => mask(Family::Reference),
                Some(Terminal::Return { .. }) => {
                    name_family(f, returned).map(mask).unwrap_or(Families(0))
                }
                _ => f.uses[index].requirement.families,
            };
            if families == Families::INT_REF {
                f.uses[index].requirement.meaning = ReadMeaning::Equality;
            }
            f.uses[index].requirement.families =
                f.uses[index].requirement.families.intersect(families);
            if matches!(block.terminal(), Some(Terminal::Return { .. })) {
                push(&mut f.uses[index].requirement.types, returned)?;
            }
        }
        if matches!(
            block.terminal(),
            Some(Terminal::Branch {
                op: BranchOp::Eq | BranchOp::Ne,
                ..
            })
        ) && uses.len() == 2
        {
            let start = s
                .terminal_site(block.id())
                .ok_or(TypeError::InternalInvariant("terminal view missing"))?
                .use_start() as usize;
            push(&mut work.compatible, (start, start + 1))?;
        }
    }
    Ok(())
}
