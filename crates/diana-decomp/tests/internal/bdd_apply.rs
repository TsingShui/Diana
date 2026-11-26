use super::*;

#[test]
fn boolean_operations_preserve_the_truth_table_of_every_three_variable_function() {
    let names = (0..3).map(BoolVariable::Atom).collect::<BTreeSet<_>>();
    let context = BddContext::new(&names);
    // This test deliberately queries many more pairs in one context than a
    // single method normally requests. Test resource failures separately.
    context.manager.borrow_mut().limits.cache_entries = 200_000;
    let vars = names
        .iter()
        .map(|v| context.compile(&BoolExpr::Symbol(v.clone())).unwrap())
        .collect::<Vec<_>>();
    let mut functions = Vec::new();
    for table in 0..256 {
        let mut function = context.falsity();
        for row in 0..8 {
            if table & (1 << row) == 0 {
                continue;
            }
            let mut cube = context.truth();
            for (index, &var) in vars.iter().enumerate() {
                let bit = if row & (1 << index) != 0 {
                    var
                } else {
                    context.not(var).unwrap()
                };
                cube = context.and(cube, bit).unwrap();
            }
            function = context.or(function, cube).unwrap();
        }
        for row in 0..8 {
            let assignments = names
                .iter()
                .enumerate()
                .map(|(i, v)| (v.clone(), row & (1 << i) != 0))
                .collect();
            assert_eq!(
                context.restrict(function, &assignments).unwrap().is_true(),
                table & (1 << row) != 0
            );
        }
        functions.push(function);
    }
    for (i, &a) in functions.iter().enumerate() {
        assert_eq!(context.not(a).unwrap(), functions[255 ^ i]);
        for (j, &b) in functions.iter().enumerate() {
            assert_eq!(context.and(a, b).unwrap(), functions[i & j]);
            assert_eq!(context.or(a, b).unwrap(), functions[i | j]);
            assert_eq!(context.implies_bdd(a, b).unwrap(), i & !j == 0);
        }
    }
}

#[test]
fn failed_operation_leaves_following_operations_and_cached_symbols_available() {
    let names = (0..4).map(BoolVariable::Atom).collect::<BTreeSet<_>>();
    let mut context = BddContext::new(&names);
    let expressions = names
        .iter()
        .cloned()
        .map(BoolExpr::Symbol)
        .collect::<Vec<_>>();
    let vars = expressions
        .iter()
        .map(|expr| context.compile(expr).unwrap())
        .collect::<Vec<_>>();
    let a = context.or(vars[0], vars[1]).unwrap();
    let b = context.or(vars[2], vars[3]).unwrap();
    context.manager.borrow_mut().limits.operations = 1;
    assert_eq!(
        context.and(a, b),
        Err(BddError::ResourceLimit {
            resource: BddResource::Operations,
            limit: 1
        })
    );
    context.manager.borrow_mut().limits.operations = 512;
    let both = context.and(a, b).unwrap();
    assert!(context.implies_bdd(both, a).unwrap());
    assert!(context.implies_bdd(both, b).unwrap());
    assert!(!context.implies_bdd(a, both).unwrap());

    context.limits.cache_entries = 4;
    let compound = BoolExpr::And(expressions.clone());
    assert_eq!(
        context.compile(&compound),
        Err(BddError::ResourceLimit {
            resource: BddResource::CacheEntries,
            limit: 4
        })
    );
    for (expr, value) in expressions.iter().zip(vars) {
        assert_eq!(context.compile(expr).unwrap(), value);
    }
    context.limits.cache_entries += 1;
    let compiled = context.compile(&compound).unwrap();
    assert!(context.implies_bdd(compiled, both).unwrap());
    assert_eq!(context.compile(&compound).unwrap(), compiled);
}

#[test]
fn negation_can_resume_after_its_first_branch_used_the_last_cache_slots() {
    let names = (0..4).map(BoolVariable::Atom).collect::<BTreeSet<_>>();
    let context = BddContext::new(&names);
    let vars: Vec<_> = names
        .iter()
        .map(|v| context.compile(&BoolExpr::Symbol(v.clone())).unwrap())
        .collect();
    let first = context.and(vars[0], vars[1]).unwrap();
    let second = context.and(vars[2], vars[3]).unwrap();
    let formula = context.or(first, second).unwrap();
    let entries = context.manager.borrow().cache_entries;
    context.manager.borrow_mut().limits.cache_entries = entries + 2;
    assert_eq!(
        context.not(formula),
        Err(BddError::ResourceLimit {
            resource: BddResource::CacheEntries,
            limit: entries + 2,
        })
    );
    context.manager.borrow_mut().limits.cache_entries += 32;
    let inverse = context.not(formula).unwrap();
    assert_eq!(context.not(inverse).unwrap(), formula);
    for bits in 0..16 {
        let assignment = names
            .iter()
            .enumerate()
            .map(|(i, v)| (v.clone(), bits & (1 << i) != 0))
            .collect();
        let expected = (bits & 3 != 3) && (bits & 12 != 12);
        assert_eq!(
            context.restrict(inverse, &assignment).unwrap().is_true(),
            expected
        );
    }
}

#[test]
fn ordered_junctions_flatten_only_the_supplied_level_and_keep_duplicate_tests() {
    let a = BoolExpr::source(1);
    let b = BoolExpr::source(2);
    for (combine, raw, neutral, absorbing) in [
        (
            BoolExpr::and as fn(Vec<BoolExpr>) -> BoolExpr,
            BoolExpr::And as fn(Vec<BoolExpr>) -> BoolExpr,
            BoolExpr::True,
            BoolExpr::False,
        ),
        (BoolExpr::or, BoolExpr::Or, BoolExpr::False, BoolExpr::True),
    ] {
        let nested = raw(vec![neutral.clone(), a.clone()]);
        let prefix = raw(vec![b.clone(), nested.clone(), b.clone()]);
        assert_eq!(
            combine(vec![neutral.clone(), prefix.clone(), a.clone()]),
            raw(vec![b.clone(), nested, b.clone(), a.clone()])
        );
        assert_eq!(
            combine(vec![prefix, absorbing.clone(), a.clone()]),
            absorbing
        );
        assert_eq!(combine(vec![neutral.clone(), neutral.clone()]), neutral);
        assert_eq!(combine(vec![neutral.clone(), b.clone()]), b);
    }
}

#[test]
fn opposite_literal_shares_the_compound_budget_and_retains_its_completed_child() {
    let first = BoolVariable::Atom(1);
    let second = BoolVariable::Atom(2);
    let mut context = BddContext::ordered([first.clone(), second.clone()]);
    context.limits.cache_entries = 1;
    let signed = BoolExpr::Not(Box::new(BoolExpr::Symbol(first.clone())));
    let error = Err(BddError::ResourceLimit {
        resource: BddResource::CacheEntries,
        limit: 1,
    });
    assert_eq!(context.compile(&signed), error);
    let positive = context.compile_literal(&first, true).unwrap();
    assert_eq!(context.compile_literal(&second, true), error);
    context.limits.cache_entries = 2;
    let negative = context.compile(&signed).unwrap();
    assert_eq!(context.compile_literal(&first, false), Ok(negative));
    assert!(context.and(positive, negative).unwrap().is_false());
    assert_eq!(
        context.compile_literal(&second, true),
        Err(BddError::ResourceLimit {
            resource: BddResource::CacheEntries,
            limit: 2
        })
    );
}

#[test]
fn arena_growth_retains_canonical_ids_and_looks_up_existing_nodes_at_its_limit() {
    let mut manager = Manager::new(
        42,
        BddLimits {
            nodes: 65_536,
            cache_entries: 1,
            operations: 1,
        },
    );
    for variable in 0..65_534 {
        assert_eq!(
            manager.intern(variable, 0, 1).unwrap(),
            (variable + 2) as u16
        );
    }
    for variable in (0..65_534).rev() {
        assert_eq!(
            manager.intern(variable, 0, 1).unwrap(),
            (variable + 2) as u16
        );
    }
    assert_eq!(
        manager.intern(0, 1, 0),
        Err(BddError::ResourceLimit {
            resource: BddResource::Nodes,
            limit: 65_536
        })
    );
    assert_eq!(manager.intern(0, 1, 1), Ok(1));
}

#[test]
fn implication_pairs_are_charged_once_per_traversal_across_spills_and_restarts() {
    let mut work = ImplicationWork::default();
    for count in [4, 8, 300, 2, 9, 8, 400] {
        work.begin();
        for id in 1..=count {
            let pair = id << 16 | (id + 1);
            assert!(work.visit(pair));
            assert!(!work.visit(pair));
            assert!(!work.visit(1 << 16 | 2));
        }
        for id in (1..=count).rev() {
            assert!(!work.visit(id << 16 | (id + 1)));
        }
    }
}

#[test]
fn implication_checks_the_high_proof_before_reporting_a_false_low_cofactor() {
    let context = BddContext::ordered((0..3).map(BoolVariable::Atom));
    let variables: Vec<_> = (0..3)
        .map(|i| {
            context
                .compile_literal(&BoolVariable::Atom(i), true)
                .unwrap()
        })
        .collect();
    let (x, y, z) = (variables[0], variables[1], variables[2]);
    // x = false disproves this implication immediately. For x = true,
    // proving (y && z) => y needs a second nonterminal visit.
    let yz = context.and(y, z).unwrap();
    let premise = context.or(context.not(x).unwrap(), yz).unwrap();
    let consequence = context.and(x, y).unwrap();
    context.manager.borrow_mut().limits.operations = 1;
    assert_eq!(
        context.implies_bdd(premise, consequence),
        Err(BddError::ResourceLimit {
            resource: BddResource::Operations,
            limit: 1,
        })
    );
    context.manager.borrow_mut().limits.operations = 2;
    assert_eq!(context.implies_bdd(premise, consequence), Ok(false));
    context.manager.borrow_mut().limits.operations = 0;
    assert_eq!(context.implies_bdd(premise, consequence), Ok(false));
}

#[test]
fn fork_rebinds_cached_compounds_and_preserves_the_remaining_cache_budget() {
    let names = (0..3).map(BoolVariable::Atom).collect::<BTreeSet<_>>();
    let bdd = BddContext::new(&names);
    let expression = BoolExpr::And(vec![
        BoolExpr::Symbol(BoolVariable::Atom(0)),
        BoolExpr::Or(vec![
            BoolExpr::Symbol(BoolVariable::Atom(1)),
            BoolExpr::Symbol(BoolVariable::Atom(2)),
        ]),
    ]);
    let value = bdd.compile(&expression).unwrap();
    let limit = {
        let mut manager = bdd.manager.borrow_mut();
        let used = manager.cache_entries;
        manager.limits.cache_entries = used;
        used
    };
    let (fork, roots) = bdd.fork([Some(value), None]).unwrap();
    let copy = roots[0].unwrap();
    assert_eq!(roots[1], None);
    assert_ne!(copy, value);
    assert_eq!(fork.compile(&expression).unwrap(), copy);
    assert_eq!(fork.and(copy, value), Err(BddError::ForeignHandle));
    assert!(matches!(
        bdd.fork([Some(copy)]),
        Err(BddError::ForeignHandle)
    ));
    for (arena, root) in [(&bdd, value), (&fork, copy)] {
        assert_eq!(
            arena.implies_bdd(root, root),
            Err(BddError::ResourceLimit {
                resource: BddResource::CacheEntries,
                limit
            })
        );
    }
    bdd.manager.borrow_mut().limits.cache_entries += 1;
    assert!(bdd.implies_bdd(value, value).unwrap());
    assert!(fork
        .implies_bdd(copy, copy)
        .unwrap_err()
        .is_resource_limit());
}

#[test]
fn variable_ordinals_keep_the_first_occurrence_in_any_input_order() {
    let a = BoolVariable::Instruction(crate::ir::InstructionId::new(17));
    let b = BoolVariable::Source(7);
    let c = BoolVariable::Named("flag".to_owned());
    for order in [
        [a.clone(), b.clone(), c.clone()],
        [c.clone(), a.clone(), b.clone()],
    ] {
        let [x, y, z] = order.clone();
        let inputs = [x.clone(), x.clone(), y.clone(), z.clone(), z.clone()];
        let context = BddContext::ordered(inputs);
        assert_eq!(context.variable_order(), &order);
        let first = context.compile_literal(&x, false).unwrap();
        let (fork, roots) = context.fork([Some(first)]).unwrap();
        assert_eq!(fork.compile_index(0, false).unwrap(), roots[0].unwrap());
        for (index, name) in order.iter().enumerate() {
            assert_eq!(context.variable_index(name), Some(index));
            let symbol = context.compile_index(index, true).unwrap();
            assert_eq!(
                context.expression(symbol, 1).unwrap(),
                Some((BoolExpr::Symbol(name.clone()), 1))
            );
        }
        let reordered =
            BddContext::ordered([z.clone(), y.clone(), z.clone(), x.clone(), y.clone()]);
        assert_eq!(reordered.variable_order(), &[z, y, x]);
        assert_eq!(context.variable_index(&BoolVariable::Atom(17)), None);
    }
}

#[test]
fn a_literal_prefix_charges_its_pair_even_when_the_result_already_exists() {
    for and in [false, true] {
        for positive in [false, true] {
            let mut manager = Manager::new(
                7,
                BddLimits {
                    nodes: 4,
                    cache_entries: 2,
                    operations: 0,
                },
            );
            let (literal_low, literal_high) = if positive { (0, 1) } else { (1, 0) };
            let literal = manager.intern(0, literal_low, literal_high).unwrap();
            let later = manager.variable(1).unwrap();
            let blocking = u16::from(!and);
            let (low, high) = if literal_low == blocking {
                (blocking, later.node)
            } else {
                (later.node, blocking)
            };
            let literal = manager.handle(literal);
            let apply = |manager: &mut Manager| {
                manager.begin_operation();
                if and {
                    manager.and(literal, later)
                } else {
                    manager.or(later, literal)
                }
            };
            assert_eq!(
                apply(&mut manager),
                Err(BddError::ResourceLimit {
                    resource: BddResource::Operations,
                    limit: 0,
                })
            );
            manager.limits.operations = 1;
            assert_eq!(
                apply(&mut manager),
                Err(BddError::ResourceLimit {
                    resource: BddResource::Nodes,
                    limit: 4,
                })
            );
            manager.limits.nodes = 5;
            // Another operation could have interned this node first.
            let result = manager.intern(0, low, high).unwrap();
            assert_eq!(
                apply(&mut manager),
                Err(BddError::ResourceLimit {
                    resource: BddResource::CacheEntries,
                    limit: 2,
                })
            );
            assert_eq!(manager.operations_used, 1);
            manager.limits.cache_entries = 3;
            assert_eq!(apply(&mut manager).unwrap().node, result);
            assert_eq!(manager.operations_used, 1);
            manager.limits.operations = 0;
            assert_eq!(apply(&mut manager).unwrap().node, result);
            assert_eq!(manager.operations_used, 0);
        }
    }
}

#[test]
fn pending_memo_entries_survive_bucket_collisions_and_recursive_table_growth() {
    for grow in [false, true] {
        let mut memo = BinaryMemo::new();
        let mask = memo.slots.len() - 1;
        let mut buckets: [Vec<u32>; 4] = std::array::from_fn(|_| Vec::new());
        for id in 3..131 {
            let key = (2 << 16) | id;
            buckets[memo.hasher.hash_one(key) as usize & mask].push(key);
        }
        let keys = buckets.into_iter().max_by_key(Vec::len).unwrap();
        let MemoLookup::Vacant(parent) = memo.lookup(keys[0]) else {
            panic!("new pair")
        };
        let descendants = if grow { &keys[1..24] } else { &keys[1..2] };
        for &key in descendants {
            let MemoLookup::Vacant(child) = memo.lookup(key) else {
                panic!("new child pair")
            };
            memo.insert(key, key as u16, child);
        }
        assert_eq!(memo.slots.len() > 4, grow);
        memo.insert(keys[0], 0, parent);
        assert!(matches!(memo.lookup(keys[0]), MemoLookup::Cached(0)));
        for &key in descendants {
            assert!(matches!(memo.lookup(key), MemoLookup::Cached(result) if result == key as u16));
        }
    }
}
