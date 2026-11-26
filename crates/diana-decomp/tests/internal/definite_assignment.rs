use super::*;

#[test]
fn joins_and_short_circuits_preserve_definite_assignment_across_word_boundaries() {
    let names: Vec<_> = (0..195)
        .map(|index| JavaIdentifier::from_hint(&format!("local{index}")))
        .collect();
    let name = |index: usize| JavaExpr::Name(names[index].clone());
    let condition = || JavaExpr::Name(JavaIdentifier::from_hint("condition"));
    let store = |index| JavaExpr::Assignment {
        target: Box::new(name(index)),
        op: JavaAssignOp::Assign,
        value: Box::new(JavaExpr::Literal(JavaLiteral::Boolean(true))),
    };
    let mut statements: Vec<_> = names
        .iter()
        .map(|name| JavaStmt::Variable {
            ty: JavaType::boolean(),
            name: name.clone(),
            value: None,
        })
        .collect();
    let branch = |omitted: usize| {
        Box::new(JavaStmt::Block(
            (0..names.len())
                .filter(|index| index % 3 != omitted)
                .map(|index| JavaStmt::Expression(store(index)))
                .collect(),
        ))
    };
    statements.push(JavaStmt::If {
        condition: condition(),
        then_stmt: branch(1),
        else_stmt: Some(branch(2)),
    });
    // Both selected operands write local127. Short-circuit evaluation can
    // skip the write to local64, even when it occurs inside a call argument.
    let arguments = vec![
        JavaExpr::Conditional {
            condition: Box::new(condition()),
            when_true: Box::new(store(127)),
            when_false: Box::new(store(127)),
        },
        JavaExpr::Binary {
            left: Box::new(condition()),
            op: JavaBinaryOp::LogicalOr,
            right: Box::new(store(64)),
        },
    ];
    statements.push(JavaStmt::Expression(JavaExpr::Call {
        receiver: None,
        owner: None,
        type_arguments: Vec::new(),
        method: JavaIdentifier::from_hint("observe"),
        args: arguments,
    }));
    statements.push(JavaStmt::Return(Some(JavaExpr::NewArray {
        element_type: JavaType::boolean(),
        dimensions: Vec::new(),
        initializer: (0..names.len()).map(name).collect(),
    })));
    let mut body = JavaMethodBody {
        root: JavaStmt::Block(statements),
    };
    assert!(DefiniteAssignment.apply(&mut body).unwrap());
    let JavaStmt::Block(statements) = &body.root else {
        panic!("method block must be preserved");
    };
    for (index, statement) in statements[..names.len()].iter().enumerate() {
        let JavaStmt::Variable { name, value, .. } = statement else {
            panic!("missing declaration for local{index}");
        };
        assert_eq!(name, &names[index]);
        let definitely_assigned = index % 3 == 0 || index == 127;
        let initializer =
            (!definitely_assigned).then_some(JavaExpr::Literal(JavaLiteral::Boolean(false)));
        assert_eq!(*value, initializer, "local{index}");
    }
}
