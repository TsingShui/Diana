package fixture;

import java.util.ArrayList;
import java.util.List;
import java.util.function.IntSupplier;

final class ScopeHooks {
    static final List<String> events = new ArrayList<>();
    static String first, second;
    static RuntimeException problem, replacement;

    static void record(String event) {
        events.add(event);
        if (event.equals(first)) throw problem;
        if (event.equals(second)) throw replacement;
    }

    static int select(int row, int mode) {
        record("select(" + row + "," + mode + ")");
        return (row + mode) & 3;
    }
    static void skip(int value) { record("skip(" + value + ")"); }
    static int start(int value) { record("start(" + value + ")"); return 1 + (value & 1); }
    static void step(int value) { record("step(" + value + ")"); }
    static int read(int value) { record("read(" + value + ")"); return 10 + value; }
    static int recover(int value) { record("recover(" + value + ")"); return 100 + value; }
    static void close(int mode) { record("close(" + mode + ")"); }
}

public final class Check {
    private static int count;
    private static final RuntimeException TYPED = new NumberFormatException("typed");
    private static final RuntimeException OTHER = new IllegalStateException("other");
    private static final RuntimeException CLEANUP = new IllegalArgumentException("cleanup");

    private static void check(
            IntSupplier method, String first, RuntimeException firstFault, String second,
            Integer result, RuntimeException failure, String events) {
        ScopeHooks.events.clear();
        ScopeHooks.first = first;
        ScopeHooks.problem = firstFault;
        ScopeHooks.second = second;
        ScopeHooks.replacement = CLEANUP;
        Integer actual = null;
        RuntimeException thrown = null;
        try { actual = method.getAsInt(); } catch (RuntimeException caught) { thrown = caught; }
        if (!java.util.Objects.equals(actual, result) || failure != thrown
                || !String.join(" ", ScopeHooks.events).equals(events)) {
            throw new AssertionError("scenario " + count + " returned " + actual
                    + " or threw " + thrown + " after " + ScopeHooks.events);
        }
        count++;
    }

    public static void main(String[] ignored) {
        check(() -> SiblingExit.run(4, 0), null, null, null, 23, null,
                "select(0,0) skip(0) select(1,0) start(1) step(2) step(1) read(1) "
                + "select(2,0) start(2) step(1) read(2) select(3,0) close(0)");
        check(() -> SiblingExit.run(1, 1), "read(0)", TYPED, null, 100, null,
                "select(0,1) start(0) step(1) read(0) recover(0) close(1)");
        check(() -> SiblingExit.run(1, 1), "step(1)", TYPED, null, null, TYPED,
                "select(0,1) start(0) step(1) close(1)");
        check(() -> SiblingExit.run(1, 1), "read(0)", OTHER, "close(1)", null, CLEANUP,
                "select(0,1) start(0) step(1) read(0) close(1)");
        check(() -> SiblingExit.run(2, 3), null, null, null, 0, null,
                "select(0,3) close(3)");
        check(() -> SiblingExit.sweep(3, 1, 0), "read(1)", TYPED, null, 123, null,
                "start(1) step(2) step(1) read(0) read(1) recover(1) read(2) step(3) close(1)");
        check(() -> SiblingExit.sweep(3, 1, 0), "step(2)", OTHER, null, null, OTHER,
                "start(1) step(2)");
        check(() -> SiblingExit.sweep(3, 1, 1), "read(1)", OTHER, null, null, OTHER,
                "start(1) step(2) step(1) read(0) read(1) step(3) close(1)");
        check(() -> SiblingExit.sweep(3, 1, 1), "read(1)", OTHER, "step(3)", null, CLEANUP,
                "start(1) step(2) step(1) read(0) read(1) step(3)");
        check(() -> SiblingExit.sweep(2, 0, 2), "read(0)", TYPED, "recover(0)", null, CLEANUP,
                "skip(0) read(0) recover(0) skip(2)");
        check(() -> SiblingExit.sweep(2, 0, 2), "read(1)", TYPED, null, 111, null,
                "skip(0) read(0) read(1) recover(1) skip(2)");
        check(() -> SiblingExit.sweep(0, 5, 9), null, null, "close(5)", null, CLEANUP,
                "step(0) close(5)");
        System.out.println(count + " exit and cleanup scenarios passed");
    }
}
