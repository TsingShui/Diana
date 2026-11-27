mod support;

use std::collections::HashMap;

use diana_decomp::{MethodSelector, Request};

const DEX: &[u8] =
    include_bytes!("../../../quality/fixtures/branches/mutable-tail-guards/classes.dex");
const SCENARIOS: &str =
    include_str!("../../../quality/fixtures/branches/mutable-tail-guards/scenarios.tsv");

/// Executes the fixture's emitted local assignments, short-circuit conditions,
/// helper calls and returns. A fresh frame records effects up to the exact throw.
struct Frame<'a> {
    parameters: &'a str,
    seed: i32,
    fault: i32,
    trace: i32,
    calls: Vec<i32>,
    locals: HashMap<&'a str, i32>,
}

impl<'a> Frame<'a> {
    fn value(&mut self, text: &str) -> Result<i32, i32> {
        if let Some(args) = text.strip_prefix("GuardTrace.step(") {
            let (parameters, node) = args.strip_suffix(')').unwrap().rsplit_once(", ").unwrap();
            assert_eq!(parameters, self.parameters);
            let node: i32 = node.parse().unwrap();
            self.calls.push(node);
            self.trace = self.trace.wrapping_mul(31).wrapping_add(node);
            if node == self.fault {
                return Err(node);
            }
            return Ok((((self.seed as u32 >> (node % 27)) as i32) ^ self.trace ^ (node * 13)) & 1);
        }
        Ok(match text {
            "true" => 1,
            "false" => 0,
            _ => text.parse().unwrap_or_else(|_| self.locals[text]),
        })
    }

    fn condition(&mut self, text: &str) -> Result<bool, i32> {
        for arm in text.split(" || ") {
            let mut matched = true;
            for term in arm.split(" && ") {
                let value = if let Some(variable) = term.strip_prefix('!') {
                    self.value(variable)? == 0
                } else if let Some(value) = term.strip_suffix(" == 0") {
                    self.value(value)? == 0
                } else {
                    self.value(term.strip_suffix(" != 0").unwrap_or(term))? != 0
                };
                if !value {
                    matched = false;
                    break;
                }
            }
            if matched {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn execute(&mut self, body: &[&'a str]) -> Result<i32, i32> {
        let mut parents = Vec::new();
        let mut active = true;
        for &line in body {
            if let Some(condition) = line.strip_prefix("if (") {
                parents.push(active);
                active = active && self.condition(condition.strip_suffix(") {").unwrap())?;
            } else if line == "} else {" {
                active = parents.last().copied().unwrap() && !active;
            } else if line == "}" {
                active = parents.pop().expect("method must have returned");
            } else if active {
                let statement = line.strip_suffix(';').expect("supported fixture statement");
                if let Some(result) = statement.strip_prefix("return ") {
                    return self.value(result);
                }
                if let Some((target, expression)) = statement.split_once(" = ") {
                    let value = self.value(expression)?;
                    self.locals
                        .insert(target.rsplit(' ').next().unwrap(), value);
                } else {
                    self.value(statement)?;
                }
            }
        }
        panic!("method did not return")
    }
}

#[test]
fn mutable_guard_keeps_the_calls_and_faults_on_the_selected_route() {
    let engine = support::engine(DEX);
    let class = support::class(DEX, &engine, "Lfixture/MutableGuardRoutes;");
    for method in ["route", "extended"] {
        let source = engine
            .class_source(Request {
                class,
                method: Some(MethodSelector {
                    name: method.encode_utf16().collect(),
                    prototype: "(II)I".encode_utf16().collect(),
                }),
            })
            .unwrap();
        assert!(source.failures.is_empty(), "{:?}", source.failures);
        let mut lines = source.text.lines().map(str::trim);
        let signature = lines
            .find(|line| line.contains(&format!("int {method}(")))
            .unwrap();
        let (first, second) = signature
            .split_once('(')
            .unwrap()
            .1
            .split_once(')')
            .unwrap()
            .0
            .split_once(", ")
            .unwrap();
        let parameters = format!(
            "{}, {}",
            first.strip_prefix("int ").unwrap(),
            second.strip_prefix("int ").unwrap()
        );
        let body: Vec<_> = lines.collect();
        let mut checked = 0;
        for scenario in SCENARIOS.lines().filter(|line| !line.starts_with('#')) {
            let columns: Vec<_> = scenario.split('\t').collect();
            if columns[0] != method {
                continue;
            }
            let seed = columns[1].parse().unwrap();
            let initial_trace = columns[2].parse().unwrap();
            let terminal: i32 = columns[3].parse().unwrap();
            let expected: Vec<i32> = columns[4]
                .split(',')
                .map(|item| item.parse().unwrap())
                .collect();
            for fault in -1..=terminal {
                let mut frame = Frame {
                    parameters: &parameters,
                    seed,
                    fault,
                    trace: initial_trace,
                    calls: Vec::new(),
                    locals: HashMap::new(),
                };
                let hit = expected.iter().position(|&node| node == fault);
                let outcome = frame.execute(&body);
                assert_eq!(
                    frame.calls,
                    expected[..hit.map_or(expected.len(), |index| index + 1)],
                    "{method} seed {seed} fault {fault}"
                );
                assert_eq!(
                    outcome,
                    hit.map_or(Ok(terminal), |_| Err(fault)),
                    "{method} seed {seed} fault {fault}"
                );
                checked += 1;
            }
        }
        assert_eq!(checked, if method == "route" { 588 } else { 700 });
    }
}
