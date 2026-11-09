//! Selected ownership and refused joint forests use the ordinary Engine entry.
//! Inputs and independent scenarios: atoms/joint-expression-cover/README.md.

use diana_decomp::p7::{
    Arguments, BlockEnd, BodyAction, ControlId, EmissionLayout, EvaluationResult, ExprId, ExprKind,
    MemberDeclaration, MethodCode, ReferenceDeclaration, StatementKind, TerminalKind,
    TransferSpelling,
};
use diana_decomp::p9::{Origin, Rendered};

#[path = "support/inspection.rs"]
mod inspection;

fn ownership(code: &MethodCode, rendered: &Rendered) {
    for expression in code.expressions() {
        let Some(evaluation) = expression.evaluation else {
            continue;
        };
        let operation = &code.operations()[evaluation.operation.0 as usize];
        assert_eq!(operation.nested_at, Some(expression.id));
        assert!(
            expression.origin.is_some(),
            "retain the original exact read"
        );
        let statement = operation.statement.expect("original producer statement");
        let source_expression = match code.statements()[statement.0 as usize].kind {
            StatementKind::Move { expression, .. } | StatementKind::Evaluate { expression } => {
                expression
            }
            _ => panic!("fixture producers are ordinary value operations"),
        };
        assert_eq!(evaluation.expression, source_expression);
        for origin in [
            Origin::Operation {
                method: diana_decomp::MethodId(0),
                operation: operation.source,
                pc: operation.pc,
            },
            Origin::Statement {
                method: diana_decomp::MethodId(0),
                statement,
            },
        ] {
            // Match original member-local identities; method IDs are engine-wide.
            let spans = rendered
                .spans()
                .iter()
                .filter(|span| match (span.origin, origin) {
                    (
                        Origin::Operation {
                            operation: actual,
                            pc,
                            ..
                        },
                        Origin::Operation {
                            operation: expected,
                            pc: expected_pc,
                            ..
                        },
                    ) => actual == expected && pc == expected_pc,
                    (
                        Origin::Statement {
                            statement: actual, ..
                        },
                        Origin::Statement {
                            statement: expected,
                            ..
                        },
                    ) => actual == expected,
                    _ => false,
                })
                .collect::<Vec<_>>();
            assert_eq!(spans.len(), 1, "one nested producer anchor");
            assert!(spans[0].start.0 < spans[0].end.0);
        }
        if let Some(pc) = operation.result_pc {
            assert_eq!(rendered.spans().iter().filter(|span| {
                matches!(span.origin, Origin::ResultCommit { operation: actual, pc: actual_pc, .. }
                    if actual == operation.source && actual_pc == pc)
            }).count(), 1);
        }
    }
    for operation in code.operations() {
        if let Some(operand) = operation.nested_at {
            assert_eq!(
                code.expressions()[operand.0 as usize]
                    .evaluation
                    .expect("owner expression")
                    .operation,
                operation.source
            );
        }
    }
}

#[test]
fn a_condition_owns_each_effectful_argument_once() {
    inspection::inspect(
        "atoms/joint-expression-cover",
        "Lfixture/JointCover;",
        "ordered",
        "(I)I",
        |_, code, rendered| {
            ownership(code, rendered);
            assert!(rendered.text().contains(
                "p0 != 0 || fixture.JointCover.pair(fixture.JointCover.take(1), fixture.JointCover.take(2)) != 0"
            ), "{}", rendered.text());
            assert_eq!(rendered.text().matches("JointCover.take(").count(), 2);
            assert_eq!(rendered.text().matches("JointCover.pair(").count(), 1);
        },
    );
}

#[test]
fn refusing_the_condition_keeps_the_whole_effect_schedule() {
    inspection::inspect(
        "atoms/joint-expression-cover",
        "Lfixture/JointCover;",
        "blocked",
        "(I)I",
        |_, code, rendered| {
            ownership(code, rendered);
            assert!(!rendered.text().contains(" || "));
            let text = rendered.text();
            let first = text.find("JointCover.take(1)").unwrap();
            let second = text.find("JointCover.take(2)").unwrap();
            let pair = text.find("JointCover.pair(").unwrap();
            let mark = text.find("JointCover.mark(").unwrap();
            assert!(first < second && second < mark && pair < mark);
            let statement = code
                .statements()
                .iter()
                .find(|statement| {
                    matches!(statement.kind, StatementKind::Evaluate { expression }
                    if call_name(code, expression).as_deref() == Some("pair"))
                })
                .unwrap();
            let StatementKind::Evaluate { expression } = statement.kind else {
                unreachable!()
            };
            let mut replay = Replay::new(code, 0, None);
            assert_eq!(replay.value(expression).unwrap(), 1112);
            assert_eq!(
                replay.events,
                vec![Event::Take(1), Event::Take(2), Event::Pair(11, 12)]
            );
            assert_eq!(text.matches("JointCover.take(").count(), 2);
            assert_eq!(text.matches("JointCover.pair(").count(), 1);
            assert_eq!(text.matches("JointCover.mark(").count(), 1);
        },
    );
}

#[test]
fn ordinary_and_reversed_operands_keep_original_producer_ownership() {
    for name in ["ordinary", "reversed"] {
        inspection::inspect(
            "atoms/joint-expression-cover",
            "Lfixture/JointCover;",
            name,
            "(I)I",
            |_, code, rendered| {
                ownership(code, rendered);
                let text = rendered.text();
                assert_eq!(text.matches("JointCover.take(").count(), 2);
                assert_eq!(text.matches("JointCover.pair(").count(), 1);
                assert!(text.contains("p0 + 1"));
            },
        );
    }
}

#[test]
fn grouped_argument_evaluations_keep_the_original_operation_record() {
    inspection::inspect(
        "atoms/joint-expression-cover",
        "Lfixture/JointCover;",
        "constructed",
        "(I)Lfixture/CoverBox;",
        |_, code, rendered| {
            ownership(code, rendered);
            assert!(
                !code.constructions().is_empty(),
                "selected original construction"
            );
            assert_eq!(rendered.text().matches("JointCover.take(").count(), 2);
            assert!(rendered.text().contains("new "));
        },
    );
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Event {
    Take(i32),
    Pair(i32, i32),
    Mark(i32),
}

fn call_name(code: &MethodCode, expression: ExprId) -> Option<String> {
    let ExprKind::Call(call) = &code.expressions()[expression.0 as usize].kind else {
        return None;
    };
    let Ok(ReferenceDeclaration::Member(MemberDeclaration::Method { name, .. })) =
        &code.references()[call.target.0 as usize].declaration
    else {
        return None;
    };
    Some(String::from_utf16(&name.units).unwrap())
}

// Executes only these fixtures' delivered scalar expressions and straight
// Sequence/Block schedule. Unknown forms fail; no upstream graph is replayed.
struct Replay<'a> {
    code: &'a MethodCode,
    locals: Vec<Option<i32>>,
    events: Vec<Event>,
    fault: Option<usize>,
}
impl<'a> Replay<'a> {
    fn new(code: &'a MethodCode, seed: i32, fault: Option<usize>) -> Self {
        let mut locals = vec![None; code.locals().len()];
        for local in code.locals() {
            if matches!(
                local.original.kind(),
                diana_decomp::p5::LocalKind::Parameter {
                    role: diana_decomp::p3::EntryRole::Parameter(0),
                    ..
                }
            ) {
                locals[local.original.representative().0 as usize] = Some(seed);
            }
        }
        Self {
            code,
            locals,
            events: Vec::new(),
            fault,
        }
    }
    fn value(&mut self, id: ExprId) -> Result<i32, usize> {
        Ok(self.expression(id)?.expect("void is not a scalar operand"))
    }
    fn expression(&mut self, id: ExprId) -> Result<Option<i32>, usize> {
        let expression = &self.code.expressions()[id.0 as usize];
        if let Some(evaluation) = expression.evaluation {
            let value = self.value(evaluation.expression)?;
            if let EvaluationResult::Store(local) = evaluation.result {
                self.locals[local.0 as usize] = Some(value);
            }
            return Ok(Some(value));
        }
        let value = match &expression.kind {
            ExprKind::Read { storage, .. } => {
                self.locals[storage.0 as usize].expect("uncaptured local")
            }
            ExprKind::Literal { bits, .. } => {
                assert_eq!(bits.width, diana_decomp::SlotWidth::One);
                (bits.value as u32) as i32
            }
            ExprKind::Binary {
                op: diana_decomp::p2::BinaryOp::AddInt,
                left,
                right,
            } => {
                let left = self.value(*left)?;
                left.wrapping_add(self.value(*right)?)
            }
            ExprKind::Call(call) => {
                assert!(call.receiver.is_none());
                let Arguments::Known(inputs) = &call.arguments else {
                    panic!("unknown call operands")
                };
                let arguments = inputs
                    .iter()
                    .map(|&input| self.value(input))
                    .collect::<Result<Vec<_>, _>>()?;
                let (event, result) =
                    match (call_name(self.code, id).as_deref(), arguments.as_slice()) {
                        (Some("take"), &[x]) => (Event::Take(x), Some(x.wrapping_add(10))),
                        (Some("pair"), &[left, right]) => (
                            Event::Pair(left, right),
                            Some(left.wrapping_mul(100).wrapping_add(right)),
                        ),
                        (Some("mark"), &[x]) => (Event::Mark(x), None),
                        _ => panic!("unmodeled helper"),
                    };
                let index = self.events.len();
                self.events.push(event);
                if self.fault == Some(index) {
                    return Err(index);
                }
                return Ok(result);
            }
            _ => panic!("unsupported scalar expression"),
        };
        Ok(Some(value))
    }
    fn control(&mut self, id: ControlId) -> Result<Option<i32>, usize> {
        let control = &self.code.controls()[id.0 as usize];
        if self.code.control_omitted(id) {
            return Ok(None);
        }
        match &control.layout {
            EmissionLayout::Sequence => {
                for &child in control.children() {
                    if let Some(value) = self.control(child)? {
                        return Ok(Some(value));
                    }
                }
                Ok(None)
            }
            EmissionLayout::Block(node) => {
                let block = &self.code.emission().blocks()[node.0 as usize];
                for action in &block.actions {
                    let BodyAction::Statement {
                        operation,
                        statement,
                    } = *action
                    else {
                        panic!("unsupported body event")
                    };
                    if self.code.nested_operation(operation)
                        || self.code.origin_only_operation(operation)
                    {
                        continue;
                    }
                    let phases = &self.code.emission().operations()[operation.0 as usize];
                    assert!(
                        phases.before.is_empty()
                            && phases.snapshots.is_empty()
                            && phases.after.is_empty()
                    );
                    let statement = &self.code.statements()[statement.0 as usize];
                    let expression = match statement.kind {
                        StatementKind::Move { expression, .. }
                        | StatementKind::Evaluate { expression } => expression,
                        _ => panic!("unsupported statement"),
                    };
                    let value = self.expression(expression)?;
                    for output in &statement.outputs {
                        let diana_decomp::p5::Materialization::Store(local) =
                            output.materialization()
                        else {
                            panic!("unsupported output")
                        };
                        self.locals[local.0 as usize] = Some(value.expect("value store"));
                    }
                }
                match &block.end {
                    BlockEnd::Terminal(block) => {
                        let TerminalKind::Return(Some(value)) =
                            self.code.terminals()[block.0 as usize].kind
                        else {
                            panic!("unsupported terminal")
                        };
                        assert!(
                            self.code.emission().terminals()[block.0 as usize]
                                .before
                                .is_empty()
                        );
                        Ok(Some(self.value(value)?))
                    }
                    BlockEnd::Transfers(transfers) => {
                        for transfer in transfers {
                            let transfer = &self.code.emission().transfers()[transfer.0 as usize];
                            assert!(matches!(transfer.spelling, TransferSpelling::Fallthrough));
                            assert!(transfer.copies.is_empty());
                        }
                        Ok(None)
                    }
                    BlockEnd::None => Ok(None),
                }
            }
            _ => panic!("unsupported control layout"),
        }
    }
    fn run(&mut self) -> Result<i32, usize> {
        assert!(self.code.emission().initial().is_empty());
        Ok(self
            .control(self.code.root().unwrap())?
            .expect("delivered return"))
    }
}

#[test]
fn joint_forests_preserve_results_effects_and_every_reachable_call_fault() {
    for name in ["ordinary", "reversed", "tree", "barrier"] {
        inspection::inspect(
            "atoms/joint-expression-cover",
            "Lfixture/JointCover;",
            name,
            "(I)I",
            |_, code, rendered| {
                ownership(code, rendered);
                for seed in [0_i32, 3, -10, i32::MAX] {
                    let a = seed.wrapping_add(10);
                    let b = seed.wrapping_add(11);
                    let left = a.wrapping_mul(100).wrapping_add(b);
                    let (events, result) = match name {
                        "ordinary" => (
                            vec![
                                Event::Take(seed),
                                Event::Take(seed.wrapping_add(1)),
                                Event::Pair(a, b),
                            ],
                            left,
                        ),
                        "reversed" => (
                            vec![
                                Event::Take(seed),
                                Event::Take(seed.wrapping_add(1)),
                                Event::Pair(b, a),
                            ],
                            b.wrapping_mul(100).wrapping_add(a),
                        ),
                        "barrier" => (
                            vec![
                                Event::Take(seed),
                                Event::Mark(a),
                                Event::Take(seed.wrapping_add(1)),
                                Event::Pair(a, b),
                            ],
                            left,
                        ),
                        "tree" => {
                            let c = seed.wrapping_add(12);
                            let d = seed.wrapping_add(13);
                            let right = c.wrapping_mul(100).wrapping_add(d);
                            (
                                vec![
                                    Event::Take(seed),
                                    Event::Take(seed.wrapping_add(1)),
                                    Event::Pair(a, b),
                                    Event::Take(seed.wrapping_add(2)),
                                    Event::Take(seed.wrapping_add(3)),
                                    Event::Pair(c, d),
                                    Event::Pair(left, right),
                                ],
                                left.wrapping_mul(100).wrapping_add(right),
                            )
                        }
                        _ => unreachable!(),
                    };
                    let mut replay = Replay::new(code, seed, None);
                    assert_eq!(replay.run(), Ok(result), "{name}({seed})");
                    assert_eq!(replay.events, events);
                    for fault in 0..events.len() {
                        let mut replay = Replay::new(code, seed, Some(fault));
                        assert_eq!(replay.run(), Err(fault), "{name}({seed}) fault {fault}");
                        assert_eq!(replay.events, events[..=fault]);
                    }
                }
                if name == "ordinary" || name == "tree" {
                    assert!(
                        !rendered.text().contains("int v"),
                        "complete joint return forest"
                    );
                }
            },
        );
    }
}
