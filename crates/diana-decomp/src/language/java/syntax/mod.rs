//! Finite Java syntax recovery proven over Semantic IR.

mod conditions;
mod expression;
mod loops;
pub(super) mod primitives;
mod protection;

use crate::analysis::SemanticTransform;
use crate::ir::{
    analysis::TypeHierarchy, SemanticMethod, SemanticNode, SemanticVisitor, SourceSemantics,
    SourceSyntaxSemantics,
};

pub struct SourceSyntaxRecovery<'a> {
    expressions: expression::ExpressionSyntax,
    loops: loops::JavaLoopSyntax<'a>,
    conditions: conditions::PathConditionSyntax,
    protection: protection::ProtectionSyntax<'a>,
}

pub struct JavaValueSyntax<'a> {
    expressions: expression::ExpressionSyntax,
    loops: loops::JavaLoopSyntax<'a>,
}

impl<'a> JavaValueSyntax<'a> {
    pub fn new(hierarchy: &'a dyn TypeHierarchy) -> Self {
        Self {
            expressions: expression::ExpressionSyntax::default(),
            loops: loops::JavaLoopSyntax::new(hierarchy),
        }
    }

    pub fn apply(
        &mut self,
        method: &mut SemanticMethod<SourceSyntaxSemantics>,
    ) -> Result<bool, crate::ir::SemanticFoldError> {
        let (body, state) = method.parts_mut();
        let recovery = self.expressions.apply(body, state.types())?;
        let loops = SyntaxSites::collect(body).loop_rewrite && self.loops.apply(body)?;
        Ok(recovery.changed || loops)
    }
}

impl<'a> SourceSyntaxRecovery<'a> {
    pub fn new(hierarchy: &'a dyn TypeHierarchy) -> Self {
        Self {
            expressions: expression::ExpressionSyntax::default(),
            loops: loops::JavaLoopSyntax::new(hierarchy),
            conditions: conditions::PathConditionSyntax,
            protection: protection::ProtectionSyntax::new(hierarchy),
        }
    }
}

impl SemanticTransform<SourceSemantics> for SourceSyntaxRecovery<'_> {
    type Output = SourceSyntaxSemantics;
    type Error = crate::ir::SemanticFoldError;

    fn transform(
        &mut self,
        mut method: SemanticMethod<SourceSemantics>,
    ) -> Result<SemanticMethod<Self::Output>, Self::Error> {
        let (body, state) = method.parts_mut();
        let expressions = self.expressions.apply(body, state.types())?;
        state
            .types_mut()
            .refine_booleans(expressions.boolean_variables);
        let sites = SyntaxSites::collect(body);
        if sites.loop_rewrite {
            self.loops.apply(body)?;
        }
        if sites.path_rewrite {
            self.conditions.apply(body, state.types())?;
        }
        if sites.multi_catch {
            self.protection.apply(body)?;
        }
        Ok(method.into_source_syntax())
    }
}

/// Loop recovery also flattens noncanonical sequences. Path recovery can
/// propagate a boolean assignment from one basic block into a later block
/// even without any predicate. Neither operation has work in a canonical
/// sequence containing one block and unconditional leaves.
#[derive(Default)]
struct SyntaxSites {
    loop_rewrite: bool,
    path_rewrite: bool,
    seen_block: bool,
    multi_catch: bool,
}

impl SyntaxSites {
    fn collect(root: &SemanticNode) -> Self {
        let mut sites = Self::default();
        sites.visit_node(root);
        sites
    }
}

impl SemanticVisitor for SyntaxSites {
    fn visits_values(&self) -> bool {
        false
    }

    fn enter_node(&mut self, node: &SemanticNode) {
        match node {
            SemanticNode::Sequence(items) => {
                let needs_flattening = items.len() < 2
                    || items.iter().any(|child| {
                        matches!(child, SemanticNode::Empty | SemanticNode::Sequence(_))
                    });
                self.loop_rewrite |= needs_flattening;
                self.path_rewrite |= needs_flattening;
            }
            SemanticNode::Empty => {}
            SemanticNode::BasicBlock(_) => {
                self.path_rewrite |= self.seen_block;
                self.seen_block = true;
            }
            SemanticNode::Leave(leave) => self.path_rewrite |= leave.condition.is_some(),
            SemanticNode::Loop { .. } => {
                self.loop_rewrite = true;
                self.path_rewrite = true;
            }
            SemanticNode::Try { catches, .. } => {
                self.path_rewrite = true;
                self.multi_catch |= catches.iter().any(|catch| catch.exception_types.len() > 1);
            }
            _ => self.path_rewrite = true,
        }
    }
}
