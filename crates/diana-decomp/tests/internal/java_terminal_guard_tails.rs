use super::*;

fn name(value: &str) -> JavaExpr {
    JavaExpr::Name(JavaIdentifier::from_hint(value))
}
fn ret(value: i32) -> JavaStmt {
    JavaStmt::Return(Some(JavaExpr::Literal(JavaLiteral::Integer(value))))
}
fn guard(condition: JavaExpr, body: JavaStmt) -> JavaStmt {
    JavaStmt::If {
        condition,
        then_stmt: Box::new(body),
        else_stmt: None,
    }
}
fn not(name_value: &str) -> JavaExpr {
    JavaExpr::Unary {
        op: JavaUnaryOp::LogicalNot,
        operand: Box::new(name(name_value)),
    }
}
#[test]
fn many_guards_keep_their_order_with_one_shared_terminal_suffix() {
    let predicate = name("ready");
    let mut statements = (0..8192)
        .map(|i| guard(predicate.clone(), ret(i)))
        .collect::<Vec<_>>();
    statements.push(ret(-1));
    let before = statements.clone();
    assert!(!TerminalBranchLinearizer::apply(&mut statements));
    assert!(!TerminalBranchLinearizer::apply_required_void_exit(
        &mut statements
    ));
    assert_eq!(statements, before);
}
#[test]
fn a_rewrite_restarts_tail_facts_for_guards_moved_out_of_its_branch() {
    let b = guard(not("b"), ret(2));
    let c = guard(not("c"), ret(3));
    let branch = JavaStmt::Block(vec![b.clone(), c, ret(4)]);
    let mut statements = vec![guard(name("a"), branch), ret(9)];
    assert!(TerminalBranchLinearizer::apply(&mut statements));
    assert_eq!(
        statements,
        vec![
            guard(not("a"), JavaStmt::Block(vec![ret(9)])),
            b,
            guard(name("c"), JavaStmt::Block(vec![ret(4)])),
            ret(3)
        ]
    );
}
#[test]
fn an_unreachable_suffix_still_contributes_to_depth_and_to_the_moved_tail() {
    let tail = JavaStmt::Expression(name("afterReturn"));
    let b = guard(not("b"), ret(2));
    let c = guard(not("c"), ret(3));
    let mut statements = vec![b.clone(), c, ret(4), tail.clone()];
    assert!(TerminalBranchLinearizer::apply(&mut statements));
    assert_eq!(
        statements,
        vec![
            b,
            guard(name("c"), JavaStmt::Block(vec![ret(4), tail])),
            ret(3)
        ]
    );
}
#[test]
fn initializer_leaves_follow_the_first_guard_with_a_noncompleting_tail() {
    let skipped = guard(name("outer"), JavaStmt::Expression(name("step")));
    let terminal = JavaStmt::Return(None);
    let mut statements = vec![
        skipped.clone(),
        guard(name("ready"), terminal.clone()),
        ret(4),
    ];
    assert!(TerminalBranchLinearizer::apply_required_void_exit(
        &mut statements
    ));
    assert_eq!(
        statements,
        vec![
            skipped,
            guard(not("ready"), JavaStmt::Block(vec![ret(4)])),
            terminal
        ]
    );
}
