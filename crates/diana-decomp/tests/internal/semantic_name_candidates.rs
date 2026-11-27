use super::*;
use crate::ir::{
    BlockId, InstructionId, LiteralArg, RegisterArg, SemanticBlock, SemanticExpression,
    SemanticStatement,
};

struct ConflictModel;
impl VariableNameModel for ConflictModel {
    fn candidates(
        &self,
        variable: &VariableNode,
        _: &VariableSemanticGraph,
        _: &VariableRoleScores,
        _: u16,
    ) -> SmallVec<[NameCandidate; 3]> {
        let choices: &[(&str, u16)] = match variable.identity() {
            0 => &[("z", 100), ("a", 80), ("a", 70)],
            1 => &[("a", 100), ("x", 79), ("a", 90)],
            2 => &[("a", 100), ("b", 80)],
            _ => &[("ignored", 34)],
        };
        choices
            .iter()
            .map(|(name, score)| NameCandidate {
                name: JavaIdentifier::from_hint(name),
                score: *score,
            })
            .collect()
    }
}

#[test]
fn reserved_score_ties_and_duplicate_hints_keep_the_name_and_variable_order() {
    let statements = (0..=3)
        .rev()
        .map(|identity| {
            let mut result = RegisterArg::new(identity, ArgType::INT);
            result.code_var = Some(identity);
            SemanticStatement::definition(
                InstructionId::new(identity as usize),
                result,
                SemanticExpression::Literal(LiteralArg::int(0)),
            )
        })
        .collect();
    let tree = SemanticNode::BasicBlock(SemanticBlock {
        id: BlockId::new(0),
        statements,
    });
    let graph = VariableSemanticGraph::analyze(&tree, &Default::default());
    let (names, fallback) = ConstrainedNameSolver::new(&ConflictModel, 35).solve(
        &graph,
        &VariableRoleScores::default(),
        &[JavaIdentifier::from_hint("z")],
        &BTreeSet::new(),
    );
    assert!(fallback);
    assert_eq!(
        names
            .into_iter()
            .map(|(id, name)| (id, name.as_str().to_owned()))
            .collect::<Vec<_>>(),
        [(0, "a".into()), (1, "a2".into()), (2, "a3".into())]
    );
}
