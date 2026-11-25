mod support;

fn selected(bytes: &[u8], owner: &str, method: &str) -> String {
    selected_signature(bytes, owner, method, "(I)I")
}

fn selected_signature(bytes: &[u8], owner: &str, method: &str, proto: &str) -> String {
    let engine = support::engine(bytes);
    let class = support::class(bytes, &engine, owner);
    let facts = engine.class_facts(class).expect("fixture class");
    let method = facts
        .resolve_method(
            &method.encode_utf16().collect::<Vec<_>>(),
            &proto.encode_utf16().collect::<Vec<_>>(),
        )
        .expect("original method");
    let result = engine.analyze_method(class, method).expect("method source");
    assert!(result.failures.is_empty(), "{:?}", result.failures);
    result.rendered.text().to_owned()
}

// No close in these inputs is protected by a DEX handler. In particular,
// a broad try around the typed handler must never catch a close failure.
fn assert_no_cleanup_retry(source: &str) {
    let lines: Vec<_> = source.lines().map(str::trim).collect();
    let mut scopes = Vec::new();
    let read = lines
        .iter()
        .rposition(|line| line.contains(".read("))
        .expect("protected read");
    let log = read
        + lines[read..]
            .iter()
            .position(|line| line.contains(".log("))
            .expect("read handler log");
    let mut log_covered = false;
    for (i, line) in lines.iter().enumerate() {
        if line.starts_with('}') {
            let opening = scopes.pop().expect("matching scope");
            if line.starts_with("} catch (Throwable") && lines[opening] == "try {" {
                log_covered |= opening < log && log < i;
                assert!(
                    !lines[opening + 1..i].iter().any(|s| s.contains(".close(")),
                    "close lies inside the catch-all range:\n{source}"
                );
            }
            if line.starts_with("} finally {") {
                log_covered |= opening < log && log < i;
                assert!(
                    !lines[opening + 1..i].iter().any(|s| s.contains(".audit(")),
                    "audit lies inside the cleanup range:\n{source}"
                );
            }
        }
        if line.ends_with('{') {
            scopes.push(i);
        }
    }
    assert!(scopes.is_empty());
    assert!(
        log_covered,
        "read-handler log lost its cleanup range:\n{source}"
    );
    assert!(source.contains("catch (IOException") && source.contains(".log("));
}

#[test]
fn typed_handler_cleanup_fault_escapes_without_retry() {
    let text = selected(
        include_bytes!("../../../quality/fixtures/exceptions/cleanup-outlet-frontiers/classes.dex"),
        "Lfixture/CleanupFrontiers;",
        "gap",
    );
    assert_no_cleanup_retry(&text);
}

#[test]
fn shared_continuation_close_remains_outside_the_adapter_catch() {
    let text = selected(
        include_bytes!("../../../quality/fixtures/exceptions/shared-cleanup-contexts/classes.dex"),
        "Lfixture/CleanupContexts;",
        "split",
    );
    assert_no_cleanup_retry(&text);
}

#[test]
fn recovered_io_audits_before_outside_cleanup() {
    let text = selected(
        include_bytes!("../../../quality/fixtures/exceptions/handler-cleanup-boundary/classes.dex"),
        "Lfixture/HandlerCleanupBoundary;",
        "run",
    );
    assert_no_cleanup_retry(&text);
    let (_, after_log) = text.split_once(".log(").expect("typed handler log");
    let (before_audit, after_audit) = after_log.split_once(".audit(").expect("outside audit");
    assert!(!before_audit.contains("finally {"), "{text}");
    assert!(after_audit.contains(".close("), "{text}");
}

fn scope_boundaries(lines: &[&str]) -> Vec<(usize, usize)> {
    let mut scopes = Vec::new();
    let mut boundaries = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        if line.starts_with('}') {
            boundaries.push((scopes.pop().expect("opening brace"), i));
        }
        if line.ends_with('{') {
            scopes.push(i);
        }
    }
    assert!(scopes.is_empty());
    boundaries
}

#[test]
fn protected_division_precedes_each_normal_cleanup_once() {
    let cases: &[(&[u8], &str, &str)] = &[
        (
            include_bytes!("../../../quality/fixtures/composite/complex-exceptions/classes.dex"),
            "Lfixture/ComplexExceptions;",
            "nestedFinally",
        ),
        (
            include_bytes!(
                "../../../quality/fixtures/obfuscated/obf-exceptions-proguard/classes.dex"
            ),
            "La/a;",
            "a",
        ),
    ];
    for &(bytes, owner, method) in cases {
        let source = selected(bytes, owner, method);
        let lines: Vec<_> = source.lines().map(str::trim).collect();
        let boundaries = scope_boundaries(&lines);
        // Follow the body and its finally clauses, excluding all caught exits.
        let mut events = Vec::new();
        for (i, line) in lines.iter().enumerate() {
            let caught = boundaries
                .iter()
                .any(|&(start, end)| lines[start].starts_with("} catch (") && start < i && i < end);
            if !caught {
                events.extend(
                    ["\"inner\"", "\"outer\"", "\"catch\""]
                        .into_iter()
                        .filter(|event| line.contains(event)),
                );
            }
        }
        assert_eq!(events, ["\"inner\"", "\"outer\""], "{source}");

        let divisions: Vec<_> = lines
            .iter()
            .enumerate()
            .filter(|(_, line)| line.contains("10 / "))
            .map(|(i, _)| i)
            .collect();
        let [division] = divisions.as_slice() else {
            panic!("expected one division:\n{source}");
        };
        let guarded = boundaries.iter().any(|&(start, handler)| {
            lines[start] == "try {"
                && start < *division
                && *division < handler
                && (lines[handler].starts_with("} finally {")
                    || lines[handler].starts_with("} catch (Throwable"))
                && boundaries.iter().any(|&(opening, end)| {
                    opening == handler
                        && lines[opening + 1..end]
                            .iter()
                            .any(|line| line.contains("\"inner\""))
                })
        });
        assert!(guarded, "division escaped the inner cleanup:\n{source}");
    }
}

#[test]
fn response_closes_before_rethrowing_an_interruption() {
    let source = selected(
        include_bytes!("../../../quality/fixtures/exceptions/response-close-snapshots/classes.dex"),
        "Lfixture/ResponseClose;",
        "response",
    );
    assert_no_cleanup_retry(&source);
    let lines: Vec<_> = source.lines().map(str::trim).collect();
    let interrupted = lines
        .iter()
        .position(|line| line.starts_with("} catch (InterruptedIOException "))
        .expect("priority of the original typed handler");
    let boundaries = scope_boundaries(&lines);
    let end = |opening| {
        boundaries
            .iter()
            .find_map(|&(start, end)| (start == opening).then_some(end))
            .expect("closing brace")
    };
    let captured = lines[interrupted]
        .strip_suffix(") {")
        .unwrap()
        .split_whitespace()
        .last()
        .expect("captured interruption");
    let expected_throw = format!("throw {captured};");
    let rethrow = (interrupted + 1..end(interrupted))
        .find(|&i| lines[i] == expected_throw)
        .expect("same interruption must propagate");
    let closes = |start: usize, stop: usize| {
        lines[start + 1..stop]
            .iter()
            .any(|line| line.contains(".close()"))
    };
    let covered = boundaries.iter().any(|&(start, handler)| {
        let catch_all = lines[handler].starts_with("} catch (Throwable") && lines[start] == "try {";
        (catch_all || lines[handler].starts_with("} finally {"))
            && start < rethrow
            && rethrow < handler
            && closes(handler, end(handler))
    });
    assert!(
        covered || closes(interrupted, rethrow),
        "interruption escaped before closing:\n{source}"
    );
}

#[test]
fn all_resource_reads_precede_the_innermost_cleanup() {
    let source = selected(
        include_bytes!(
            "../../../quality/fixtures/exceptions/three-resources-late-failure/classes.dex"
        ),
        "Lfixture/ThreeResourceLateFailure;",
        "openThree",
    );
    let lines: Vec<_> = source.lines().map(str::trim).collect();
    let handles: Vec<_> = (0..3)
        .map(|id| {
            let acquisition = format!(".open({id})");
            let line = lines
                .iter()
                .find(|line| line.contains(&acquisition))
                .expect("resource acquisition");
            line.split_once(" = ")
                .unwrap()
                .0
                .split_whitespace()
                .last()
                .unwrap()
        })
        .collect();
    let reads: Vec<_> = source
        .split(".read(")
        .skip(1)
        .map(|tail| tail.split_once(')').unwrap().0)
        .collect();
    assert_eq!(reads, handles, "{source}");

    let boundaries = scope_boundaries(&lines);
    let inner_close = format!(".close({})", handles[2]);
    let protected = boundaries.iter().any(|&(start, handler)| {
        lines[start] == "try {"
            && lines[handler].starts_with("} catch (Throwable")
            && lines[start + 1..handler]
                .iter()
                .map(|line| line.matches(".read(").count())
                .sum::<usize>()
                == 3
            && boundaries.iter().any(|&(opening, end)| {
                opening == handler
                    && lines[opening + 1..end]
                        .iter()
                        .any(|line| line.contains(&inner_close))
                    && lines[opening + 1..end]
                        .iter()
                        .any(|line| line.contains(".addSuppressed("))
            })
    });
    assert!(
        protected,
        "a resource read escaped its cleanup handler:\n{source}"
    );
}

#[test]
fn ordinary_exit_effects_escape_earlier_typed_handlers() {
    let cases: &[(&[u8], &str, &str, &str, &str)] = &[
        (
            include_bytes!(
                "../../../quality/fixtures/exceptions/scalar-views-protected-joins/classes.dex"
            ),
            "Lfixture/ProtectedViews;",
            "route",
            "([CIZ)I",
            ".mark(",
        ),
        (
            include_bytes!("../../../quality/fixtures/exceptions/nested-catch-display/classes.dex"),
            "Lfixture/LayeredCatch;",
            "layered",
            "(I)I",
            ".finish(",
        ),
    ];
    for &(bytes, owner, name, proto, call) in cases {
        let source = selected_signature(bytes, owner, name, proto);
        let lines: Vec<_> = source.lines().map(str::trim).collect();
        let is_exit_effect =
            |line: &&str| line.contains(call) && (call != ".mark(" || line.contains("70"));
        assert!(lines.iter().any(is_exit_effect), "{source}");
        let mut typed = scope_boundaries(&lines)
            .into_iter()
            .filter(|&(start, end)| {
                lines[start] == "try {" && lines[end].starts_with("} catch (IllegalStateException")
            })
            .peekable();
        assert!(typed.peek().is_some(), "{source}");
        for (start, end) in typed {
            assert!(
                !lines[start + 1..end].iter().any(is_exit_effect),
                "ordinary exit effect is inside the typed catch range:\n{source}"
            );
        }
    }
}
