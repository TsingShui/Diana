use diana_decomp::language::java::{
    JavaBinaryOp, JavaExpr, JavaIdentifier, JavaMethodBody, JavaPrinter, JavaStmt, JavaUnaryOp,
};

fn name(value: &str) -> JavaExpr {
    JavaExpr::Name(JavaIdentifier::from_hint(value))
}

fn binary(left: JavaExpr, op: JavaBinaryOp, right: JavaExpr) -> JavaExpr {
    JavaExpr::Binary {
        left: Box::new(left),
        op,
        right: Box::new(right),
    }
}

#[test]
fn right_operands_preserve_order_and_precedence_at_depth() {
    let mut expr = name("end");
    let depth = 2048;
    for _ in 0..depth {
        expr = binary(name("n"), JavaBinaryOp::Subtract, expr);
    }
    let body = JavaMethodBody {
        root: JavaStmt::Return(Some(expr)),
    };
    let text = JavaPrinter::default().print_method_body(&body).unwrap();
    let expected = format!(
        "return {}n - end{};\n",
        "n - (".repeat(depth - 1),
        ")".repeat(depth - 1)
    );
    assert_eq!(text, expected);

    let JavaStmt::Return(Some(mut expr)) = body.root else {
        unreachable!()
    };
    while let JavaExpr::Binary { right, .. } = expr {
        expr = *right;
    }
}

#[test]
fn right_boolean_runs_and_nested_unary_operators_keep_their_grouping() {
    let expr = binary(
        binary(name("a"), JavaBinaryOp::LogicalOr, name("b")),
        JavaBinaryOp::LogicalAnd,
        binary(
            name("c"),
            JavaBinaryOp::LogicalAnd,
            JavaExpr::Unary {
                op: JavaUnaryOp::LogicalNot,
                operand: Box::new(binary(name("d"), JavaBinaryOp::Equal, name("e"))),
            },
        ),
    );
    let body = JavaMethodBody {
        root: JavaStmt::Return(Some(expr)),
    };
    assert_eq!(
        JavaPrinter::default().print_method_body(&body).unwrap(),
        "return (a || b) && c && !(d == e);\n"
    );
}
