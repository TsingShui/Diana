use super::*;

fn name(value: &str) -> JavaExpr {
    JavaExpr::Name(JavaIdentifier::from_dex(value))
}

fn binary(left: JavaExpr, op: JavaBinaryOp, right: JavaExpr) -> JavaExpr {
    JavaExpr::Binary {
        left: Box::new(left),
        op,
        right: Box::new(right),
    }
}

fn not(value: JavaExpr) -> JavaExpr {
    JavaExpr::Unary {
        op: JavaUnaryOp::LogicalNot,
        operand: Box::new(value),
    }
}

// Independent, allocating specification for the small test expressions.
// The rendering threshold and the printed-node cost have different meanings.
fn expected(value: &JavaExpr) -> (JavaExpr, u8) {
    use JavaBinaryOp::*;
    match value {
        JavaExpr::Binary {
            left,
            op: op @ (LogicalAnd | LogicalOr),
            right,
        } => {
            let (left, a) = expected(left);
            let (right, b) = expected(right);
            if a + b > 3 {
                return (not(value.clone()), 3);
            }
            let inverse = if *op == LogicalAnd {
                LogicalOr
            } else {
                LogicalAnd
            };
            (binary(left, inverse, right), a + b)
        }
        JavaExpr::Binary { left, op, right } => {
            let opposite = match op {
                Equal => NotEqual,
                NotEqual => Equal,
                IntegralLess => IntegralGreaterEqual,
                IntegralGreaterEqual => IntegralLess,
                IntegralGreater => IntegralLessEqual,
                IntegralLessEqual => IntegralGreater,
                _ => return (not(value.clone()), 1),
            };
            (binary(*left.clone(), opposite, *right.clone()), 0)
        }
        JavaExpr::Unary {
            op: JavaUnaryOp::LogicalNot,
            operand,
        } => (*operand.clone(), 0),
        JavaExpr::Literal(JavaLiteral::Boolean(b)) => {
            (JavaExpr::Literal(JavaLiteral::Boolean(!b)), 0)
        }
        _ => (not(value.clone()), 1),
    }
}

#[test]
fn complement_budget_ties_and_printed_cost_agree_with_the_constructed_expression() {
    use JavaBinaryOp::*;
    let mut leaves = vec![
        name("a"),
        not(name("b")),
        JavaExpr::Literal(JavaLiteral::Boolean(true)),
    ];
    for op in [
        Equal,
        NotEqual,
        Less,
        GreaterEqual,
        Greater,
        LessEqual,
        IntegralLess,
        IntegralGreaterEqual,
        IntegralGreater,
        IntegralLessEqual,
        BitOr,
    ] {
        leaves.push(binary(name("left"), op, name("right")));
    }
    let three = binary(
        binary(name("a"), LogicalAnd, name("b")),
        LogicalOr,
        name("c"),
    );
    let four = binary(three.clone(), LogicalAnd, name("d"));
    leaves.extend([three, four]);
    for a in &leaves {
        for b in &leaves {
            for c in &leaves {
                let input = binary(
                    a.clone(),
                    LogicalAnd,
                    binary(b.clone(), LogicalOr, c.clone()),
                );
                let (expected, _) = expected(&input);
                assert_eq!(
                    input.negation_reduces_cost(),
                    expected.cost() < input.cost()
                );
                assert_eq!(input.negated(), expected);
            }
        }
    }
}

#[test]
fn a_deep_junction_visits_each_logical_node_and_operand_once() {
    use JavaBinaryOp::*;
    const LEAVES: usize = 4096;
    let leaf = || binary(name("key"), Equal, name("match"));
    let mut chain = leaf();
    for _ in 1..LEAVES {
        chain = binary(chain, LogicalAnd, leaf());
    }
    let plan = NegationPlan::analyze(&chain);
    assert_eq!(plan.visits, 2 * LEAVES - 1);
    assert_eq!(plan.edits.len(), plan.visits);
    assert_eq!(plan.cost_delta, 0);
    let JavaExpr::Binary { right, .. } = &chain else {
        unreachable!()
    };
    let right_address = std::ptr::from_ref(right.as_ref()).addr();
    plan.apply(&mut chain);
    let JavaExpr::Binary { right, .. } = &chain else {
        unreachable!()
    };
    assert_eq!(std::ptr::from_ref(right.as_ref()).addr(), right_address);
    // Release a deliberately deep input iteratively, verifying each output arm.
    for _ in 1..LEAVES {
        let JavaExpr::Binary {
            left,
            op: LogicalOr,
            right,
        } = chain
        else {
            panic!("expected a distributed disjunction");
        };
        assert!(matches!(*right, JavaExpr::Binary { op: NotEqual, .. }));
        chain = *left;
    }
    assert!(matches!(chain, JavaExpr::Binary { op: NotEqual, .. }));
}
