use super::*;

#[test]
fn binary_apply_preserves_the_truth_table_of_every_three_variable_function() {
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
    let entries = {
        let manager = context.manager.borrow();
        manager.binary_cache.len() + manager.complements
    };
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
