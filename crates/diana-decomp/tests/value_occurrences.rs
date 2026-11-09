//! Value identity survives storage reuse. The independent executor below covers
//! only linear integer final plans and the two authored numeric helper bodies.
//! Array contents, cast paths and protected control receive separate product and
//! source reviews; this is not a general JVM or final-plan behavioral suite.
use diana_decomp::p2::{BinaryOp, OpKind};
use diana_decomp::p3::EntryRole;
use diana_decomp::p5::{LocalKind, Materialization};
use diana_decomp::p7::*;
use diana_decomp::p9::{Origin, Rendered};

#[path = "support/inspection.rs"]
mod inspection;
use inspection::inspect;

const BASIC: &str = "atoms/value-occurrences";
const OWNER: &str = "Lfixture/ValueOccurrences;";
const CHALLENGE: &str = "atoms/value-occurrence-effects";
const EFFECTS: &str = "Lfixture/ValueOccurrenceEffects;";

#[derive(Debug, PartialEq, Eq)]
enum Fault {
    Arithmetic,
    Unchecked(&'static str),
}
#[derive(Debug, PartialEq, Eq)]
struct Observation {
    result: Result<i32, Fault>,
    calls: Vec<(String, Vec<i32>)>,
    visits: usize,
}
struct Execute<'a> {
    code: &'a MethodCode,
    locals: Vec<Option<i32>>,
    calls: Vec<(String, Vec<i32>)>,
    visits: usize,
    throwing: bool,
    duplicate_tick: bool,
    fuel: usize,
}
impl Execute<'_> {
    fn expression(&mut self, id: ExprId) -> Result<i32, Fault> {
        self.fuel = self
            .fuel
            .checked_sub(1)
            .ok_or(Fault::Unchecked("test budget"))?;
        if let Some(composed) = self.code.emission().composed_use(id) {
            let value = self.expression(composed.expression)?;
            if let ComposedResult::Store(local) = composed.result {
                self.locals[local.0 as usize] = Some(value);
            }
            return Ok(value);
        }
        match &self.code.expressions()[id.0 as usize].kind {
            ExprKind::Read { storage, .. } => self.locals[storage.0 as usize]
                .ok_or(Fault::Unchecked("read without a materialization")),
            ExprKind::Literal { bits, .. } if bits.width == diana_decomp::SlotWidth::One => {
                Ok(bits.value as u32 as i32)
            }
            ExprKind::Binary { op, left, right } => {
                let left = self.expression(*left)?;
                let right = self.expression(*right)?;
                match op {
                    BinaryOp::AddInt => Ok(left.wrapping_add(right)),
                    BinaryOp::MulInt => Ok(left.wrapping_mul(right)),
                    _ => Err(Fault::Unchecked("numeric operation")),
                }
            }
            ExprKind::Call(call) => {
                if call.receiver.is_some() {
                    return Err(Fault::Unchecked("instance call"));
                }
                let ReferenceDeclaration::Member(MemberDeclaration::Method {
                    owner,
                    name,
                    prototype,
                }) = &self.code.references()[call.target.0 as usize]
                    .declaration
                    .as_ref()
                    .map_err(|_| Fault::Unchecked("call declaration"))?
                else {
                    return Err(Fault::Unchecked("call declaration"));
                };
                let owner = &self.code.types()[owner.0 as usize].units;
                if owner != &OWNER.encode_utf16().collect::<Vec<_>>()
                    && owner != &EFFECTS.encode_utf16().collect::<Vec<_>>()
                {
                    return Err(Fault::Unchecked("helper owner"));
                }
                if self.code.types()[prototype.returns.0 as usize].units != [u16::from(b'I')]
                    || prototype
                        .parameters
                        .iter()
                        .any(|ty| self.code.types()[ty.0 as usize].units != [u16::from(b'I')])
                {
                    return Err(Fault::Unchecked("helper prototype"));
                }
                let name = String::from_utf16(&name.units).unwrap();
                let Arguments::Known(arguments) = &call.arguments else {
                    return Err(Fault::Unchecked("arguments"));
                };
                let arguments = arguments
                    .iter()
                    .map(|&id| self.expression(id))
                    .collect::<Result<Vec<_>, _>>()?;
                self.calls.push((name.clone(), arguments.clone()));
                match (name.as_str(), arguments.as_slice()) {
                    ("tick", [value]) => {
                        self.visits += 1;
                        if self.duplicate_tick {
                            self.calls.push((name, arguments.clone()));
                            self.visits += 1;
                        }
                        if self.throwing && *value == 13 {
                            Err(Fault::Arithmetic)
                        } else {
                            Ok(value.wrapping_add(7))
                        }
                    }
                    ("pair", [a, b]) => Ok(a.wrapping_mul(10).wrapping_add(*b)),
                    _ => Err(Fault::Unchecked("helper body")),
                }
            }
            _ => Err(Fault::Unchecked("expression")),
        }
    }
    fn control(&mut self, id: ControlId) -> Result<Option<i32>, Fault> {
        let emission = &self.code.emission().controls()[id.0 as usize];
        if emission.omitted {
            return Ok(None);
        }
        if emission.continuation.is_some() || !emission.source_only.is_empty() {
            return Err(Fault::Unchecked("scope completion"));
        }
        match emission.layout {
            EmissionLayout::Sequence => {
                for &child in emission.children(&self.code.controls()[id.0 as usize]) {
                    if let Some(value) = self.control(child)? {
                        return Ok(Some(value));
                    }
                }
            }
            EmissionLayout::Block(node) => {
                let block = &self.code.emission().blocks()[node.0 as usize];
                for action in &block.actions {
                    let BodyAction::Statement {
                        operation,
                        statement,
                    } = *action
                    else {
                        return Err(Fault::Unchecked("action"));
                    };
                    if self.code.emission().origin_only_operation(operation)
                        || self.code.emission().composed_operation(operation)
                        || self.code.emission().omitted_literal(statement)
                        || self.code.emission().omitted_nop(statement)
                    {
                        continue;
                    }
                    let phases = &self.code.emission().operations()[operation.0 as usize];
                    if !phases.before.is_empty()
                        || !phases.snapshots.is_empty()
                        || !phases.after.is_empty()
                    {
                        return Err(Fault::Unchecked("operation phases"));
                    }
                    let statement = &self.code.statements()[statement.0 as usize];
                    let (StatementKind::Move { expression, .. }
                    | StatementKind::Evaluate { expression }) = statement.kind
                    else {
                        return Err(Fault::Unchecked("statement"));
                    };
                    let value = self.expression(expression)?;
                    for output in &statement.outputs {
                        if let Materialization::Store(local) = output.materialization() {
                            self.locals[local.0 as usize] = Some(value);
                        } else {
                            return Err(Fault::Unchecked("output"));
                        }
                    }
                }
                match &block.end {
                    BlockEnd::None => {}
                    BlockEnd::Transfers(transfers) => {
                        for &transfer in transfers {
                            let transfer = &self.code.emission().transfers()[transfer.0 as usize];
                            if !transfer.copies.is_empty()
                                || !matches!(transfer.spelling, TransferSpelling::Fallthrough)
                            {
                                return Err(Fault::Unchecked("transfer"));
                            }
                        }
                    }
                    BlockEnd::Terminal(terminal) => {
                        let phases = &self.code.emission().terminals()[terminal.0 as usize];
                        if !phases.before.is_empty()
                            || !phases.snapshots.is_empty()
                            || !phases.after.is_empty()
                        {
                            return Err(Fault::Unchecked("terminal phases"));
                        }
                        if self.code.emission().origin_only_terminal(*terminal) {
                            return Ok(None);
                        }
                        match self.code.terminals()[terminal.0 as usize].kind {
                            TerminalKind::Return(Some(value)) => {
                                return self.expression(value).map(Some);
                            }
                            _ => return Err(Fault::Unchecked("terminal")),
                        }
                    }
                }
            }
            _ => return Err(Fault::Unchecked("control")),
        }
        Ok(None)
    }
}
fn execute(
    code: &MethodCode,
    arguments: &[i32],
    throwing: bool,
    duplicate_tick: bool,
) -> Observation {
    let mut run = Execute {
        code,
        locals: vec![None; code.locals().len()],
        calls: Vec::new(),
        visits: 0,
        throwing,
        duplicate_tick,
        fuel: 4096,
    };
    for local in code.locals() {
        if let LocalKind::Parameter {
            role: EntryRole::Parameter(index),
            ..
        } = local.original.kind()
        {
            run.locals[local.id.0 as usize] = Some(arguments[index as usize]);
        }
    }
    let result = if code.emission().initial().is_empty() {
        run.control(code.root().unwrap())
            .and_then(|v| v.ok_or(Fault::Unchecked("missing return")))
    } else {
        Err(Fault::Unchecked("initial copies"))
    };
    Observation {
        result,
        calls: run.calls,
        visits: run.visits,
    }
}
fn call(name: &str, values: &[i32]) -> (String, Vec<i32>) {
    (name.into(), values.into())
}
fn anchors(code: &MethodCode, rendered: &Rendered) {
    for operation in code
        .operations()
        .iter()
        .filter(|op| !matches!(op.emission, Emission::Literal))
    {
        assert_eq!(
            rendered
                .spans()
                .iter()
                .filter(|span| matches!(span.origin,
            Origin::Operation { operation: id, .. } if id == operation.source))
                .count(),
            1,
            "original operation: {operation:?}\n{}",
            rendered.text()
        );
    }
}

#[test]
fn reused_storages_do_not_block_exact_numeric_definitions() {
    inspect(BASIC, OWNER, "chain", "(II)I", |_, code, rendered| {
        assert!(code.locals().iter().any(|local| local.original.is_reused()));
        assert!(rendered.text().contains("return ((p0 + p1) * 3);"));
        assert!(!rendered.text().contains("int v"));
        for (arguments, expected) in [([1, 2], 9), ([-4, 2], -6), ([i32::MAX, 1], i32::MIN)] {
            assert_eq!(
                execute(code, &arguments, false, false),
                Observation {
                    result: Ok(expected),
                    calls: vec![],
                    visits: 0
                }
            );
        }
        anchors(code, rendered);
    });
}
#[test]
fn a_composed_call_is_not_erased_by_an_identity_move() {
    inspect(BASIC, OWNER, "effects", "(I)I", |_, code, rendered| {
        assert!(
            rendered
                .text()
                .contains("return fixture.ValueOccurrences.pair("),
            "{}",
            rendered.text()
        );
        assert_eq!(
            execute(code, &[3], false, false),
            Observation {
                result: Ok(102),
                calls: vec![call("tick", &[3]), call("pair", &[10, 2])],
                visits: 1
            }
        );
        anchors(code, rendered);
    });
}
#[test]
fn nested_calls_preserve_original_argument_and_effect_order() {
    inspect(BASIC, OWNER, "repeated", "(I)I", |_, code, rendered| {
        assert!(
            rendered.text().contains(
                "return fixture.ValueOccurrences.tick(fixture.ValueOccurrences.tick(p0));"
            )
        );
        assert_eq!(
            execute(code, &[3], false, false),
            Observation {
                result: Ok(17),
                calls: vec![call("tick", &[3]), call("tick", &[10])],
                visits: 2
            }
        );
        anchors(code, rendered);
    });
}
#[test]
fn multiple_reads_retain_one_committed_value_and_its_declaration() {
    inspect(BASIC, OWNER, "shared", "(I)I", |_, code, rendered| {
        assert_eq!(rendered.text().matches("ValueOccurrences.tick(").count(), 1);
        assert!(
            rendered
                .text()
                .contains(" = fixture.ValueOccurrences.tick(p0);"),
            "{}",
            rendered.text()
        );
        assert_eq!(
            execute(code, &[3], false, false),
            Observation {
                result: Ok(110),
                calls: vec![call("tick", &[3]), call("pair", &[10, 10])],
                visits: 1
            }
        );
        anchors(code, rendered);
    });
}
#[test]
fn an_exact_array_fill_names_its_allocation_not_its_representative() {
    inspect(BASIC, OWNER, "bytes", "()[B", |_, code, rendered| {
        assert!(rendered.text().contains("new byte[]{1, 2, 3}"));
        assert!(!rendered.text().contains("fill_array"));
        anchors(code, rendered);
    });
}
#[test]
fn checked_reads_keep_the_original_cast_and_call_order() {
    inspect(
        BASIC,
        OWNER,
        "checked",
        "(Ljava/lang/Object;)I",
        |_, code, rendered| {
            assert_eq!(code.emission().inline_checks().len(), 1);
            assert!(rendered.text().contains("String) "));
            assert!(
                rendered.text().find("identity(p0)").unwrap()
                    < rendered.text().find(".length()").unwrap()
            );
            anchors(code, rendered);
        },
    );
}
#[test]
fn shared_arithmetic_and_three_helper_calls_compose_without_reexecution() {
    inspect(
        CHALLENGE,
        EFFECTS,
        "combined",
        "(II)I",
        |_, code, rendered| {
            assert!(rendered.text().contains(
                "return fixture.ValueOccurrenceEffects.tick(fixture.ValueOccurrenceEffects.pair("
            ));
            for (arguments, expected, a, b) in [([3, 2], 403, 36, 396), ([-8, 2], 40, 3, 33)] {
                assert_eq!(
                    execute(code, &arguments, true, false),
                    Observation {
                        result: Ok(expected),
                        calls: vec![
                            call("tick", &[arguments[0]]),
                            call("pair", &[a, a]),
                            call("tick", &[b])
                        ],
                        visits: 2
                    }
                );
            }
            assert_eq!(
                execute(code, &[13, 2], true, false),
                Observation {
                    result: Err(Fault::Arithmetic),
                    calls: vec![call("tick", &[13])],
                    visits: 1
                }
            );
            assert_eq!(
                execute(code, &[650_752_614, 0], true, false),
                Observation {
                    result: Err(Fault::Arithmetic),
                    calls: vec![
                        call("tick", &[650_752_614]),
                        call("pair", &[1_952_257_863, 1_952_257_863]),
                        call("tick", &[13])
                    ],
                    visits: 2,
                }
            );
            anchors(code, rendered);
        },
    );
}
#[test]
fn the_protected_result_keeps_a_declaration_covering_its_outer_read() {
    inspect(
        CHALLENGE,
        EFFECTS,
        "protectedValue",
        "(I)I",
        |_, code, rendered| {
            let calls = code
                .operations()
                .iter()
                .filter(|op| matches!(op.original, OpKind::Invoke { .. }))
                .collect::<Vec<_>>();
            assert_eq!(calls.len(), 2);
            let output = &code.statements()[calls[1].statement.unwrap().0 as usize].outputs[0];
            let Materialization::Store(local) = output.materialization() else {
                panic!("original result storage")
            };
            assert!(
                code.emission()
                    .local_initializer(StorageId(local.0))
                    .is_none(),
                "the return is outside the try"
            );
            assert!(
                rendered
                    .text()
                    .find("ValueOccurrenceEffects.tick(p0)")
                    .unwrap()
                    < rendered.text().find("try {").unwrap()
            );
            assert!(
                rendered
                    .text()
                    .contains("catch (java.lang.ArithmeticException")
            );
            anchors(code, rendered);
        },
    );
}
#[test]
fn two_exact_payloads_remain_distinct_across_publication() {
    inspect(
        CHALLENGE,
        EFFECTS,
        "twoArrays",
        "()[B",
        |_, code, rendered| {
            let text = rendered.text();
            assert!(text.find("new byte[]{1, 2, 3}").unwrap() < text.find(".sink(").unwrap());
            assert!(text.find(".sink(").unwrap() < text.find("new byte[]{4, 5, 6}").unwrap());
            assert!(!text.contains("fill_array"));
            anchors(code, rendered);
        },
    );
}
#[test]
fn grouped_argument_definitions_do_not_reintroduce_unused_storage() {
    inspect(
        "atoms/constructor-argument-chains",
        "Lfixture/ConstructorArguments;",
        "point",
        "()Lfixture/Point;",
        |_, code, _| {
            let group = &code.constructions()[0];
            assert_eq!(group.prepared_arguments.len(), 6);
            for argument in &group.prepared_arguments {
                assert!(code.emission().hidden_local(argument.local));
            }
        },
    );
}

#[test]
fn the_independent_observation_detects_an_extra_helper_effect() {
    inspect(BASIC, OWNER, "shared", "(I)I", |_, code, _| {
        let actual = execute(code, &[3], false, true);
        assert_eq!(actual.result, Ok(110));
        assert_ne!(
            actual,
            Observation {
                result: Ok(110),
                calls: vec![call("tick", &[3]), call("pair", &[10, 10])],
                visits: 1
            }
        );
    });
}
