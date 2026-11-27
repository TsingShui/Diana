use super::*;
use crate::language::java::{JavaCatch, JavaIdentifier, JavaLiteral, JavaSwitchCase};

fn literal(i: i32) -> JavaExpr {
    JavaExpr::Literal(JavaLiteral::Integer(i))
}
fn tree() -> JavaStmt {
    JavaStmt::Switch {
        label: None,
        selector: literal(1),
        cases: vec![JavaSwitchCase {
            labels: vec![literal(2)],
            is_default: false,
            body: vec![
                JavaStmt::Try {
                    body: Box::new(JavaStmt::Return(Some(literal(3)))),
                    catches: vec![JavaCatch {
                        types: Vec::new(),
                        variable: JavaIdentifier::from_hint("e"),
                        body: JavaStmt::Return(Some(literal(4))),
                    }],
                    finally: Some(Box::new(JavaStmt::Return(Some(literal(5))))),
                },
                JavaStmt::DoWhile {
                    label: None,
                    body: Box::new(JavaStmt::Throw(literal(6))),
                    condition: literal(7),
                },
            ],
        }],
    }
}

#[derive(Default)]
struct Edits(Vec<i32>);
impl JavaAstMutator for Edits {
    fn after_expression(&mut self, expression: &mut JavaExpr) {
        if let JavaExpr::Literal(JavaLiteral::Integer(number)) = expression {
            self.0.push(*number);
            *number += 10;
        }
    }
}
#[derive(Default)]
struct OwningEdits {
    numbers: Vec<i32>,
    lists: Vec<usize>,
}
impl JavaAstRewriter for OwningEdits {
    fn rewrite_expression(&mut self, expression: JavaExpr) -> JavaExpr {
        if let JavaExpr::Literal(JavaLiteral::Integer(number)) = expression {
            self.numbers.push(number);
            return literal(number + 10);
        }
        expression
    }
    fn rewrite_expressions(&mut self, values: Vec<JavaExpr>) -> Vec<JavaExpr> {
        self.lists.push(values.len());
        values
            .into_iter()
            .map(|e| self.rewrite_expression(e))
            .collect()
    }
}
#[test]
fn mutable_and_owning_walks_preserve_switch_try_and_loop_callback_order() {
    let mut edits = Edits::default();
    let mut owning = OwningEdits::default();
    let mutable = edits.rewrite_statement(tree());
    let rewritten = owning.rewrite_statement(tree());
    assert_eq!(edits.0, [2, 4, 3, 5, 6, 7, 1]);
    assert_eq!(owning.numbers, [2, 4, 3, 5, 6, 7, 1]);
    assert_eq!(owning.lists, [1]);
    assert_eq!(rewritten, mutable);
}
