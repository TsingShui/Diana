public final class ReturnLanes {
    public static int trace;

    public static Object lookup(Object supplied, RuntimeException bodyFault,
                                RuntimeException cursorFault, RuntimeException ownerFault) {
        synchronized (ReturnLanes.class) {
            stamp(1);
            try {
                Object result;
                try {
                    result = supplied == null ? null : capture(supplied);
                    if (result != null) markThenThrow(3, bodyFault);
                } catch (Throwable primary) {
                    try {
                        markThenThrow(4, cursorFault);
                    } catch (Throwable cleanup) {
                        primary.addSuppressed(cleanup);
                    }
                    throw primary;
                }
                markThenThrow(4, cursorFault);
                return result;
            } catch (Exception failure) {
                return null;
            } finally {
                markThenThrow(5, ownerFault);
            }
        }
    }

    private static Object capture(Object supplied) {
        stamp(2);
        return supplied;
    }

    private static void markThenThrow(int event, RuntimeException failure) {
        stamp(event);
        if (failure != null) throw failure;
    }

    private static void stamp(int event) {
        trace = trace * 10 + event;
    }

    private static final Object ITEM = new Object();

    private static void check(String requested, String id, boolean empty, boolean body, boolean resource, boolean owner,
                              boolean same) {
        if (!id.equals(requested)) return;
        RuntimeException bodyFailure = body ? new IllegalStateException("body") : null;
        RuntimeException resourceFailure = resource
            ? (same ? bodyFailure : new ArithmeticException("resource")) : null;
        RuntimeException ownerFailure = owner ? new IllegalArgumentException("owner") : null;
        String outcome;
        ReturnLanes.trace = 0;
        try {
            Object returned = ReturnLanes.lookup(empty ? null : ITEM, bodyFailure, resourceFailure, ownerFailure);
            outcome = returned == ITEM ? "item" : returned == null ? "null" : "another-object";
        } catch (RuntimeException failure) {
            outcome = failure == ownerFailure ? "owner-fault" : failure == bodyFailure ? "body-fault"
                : failure == resourceFailure ? "resource-fault" : "different-fault";
        }
        Throwable[] suppressed = bodyFailure == null ? new Throwable[0] : bodyFailure.getSuppressed();
        String suppression = suppressed.length == 0 ? "none"
            : suppressed.length == 1 && suppressed[0] == resourceFailure ? "resource-fault" : "different";
        System.out.printf("%s,%d,%s,%s%n", outcome, ReturnLanes.trace, suppression,
            Thread.holdsLock(ReturnLanes.class) ? "held" : "released");
    }

    public static void main(String[] args) {
        check(args[0], "nonempty", false, false, false, false, false);
        check(args[0], "empty", true, false, false, false, false);
        check(args[0], "body-fault", false, true, false, false, false);
        check(args[0], "normal-close-fault", false, false, true, false, false);
        check(args[0], "body-and-resource-fault", false, true, true, false, false);
        check(args[0], "same-fault", false, true, true, false, true);
        check(args[0], "normal-owner-fault", false, false, false, true, false);
        check(args[0], "caught-owner-fault", false, true, false, true, false);
        check(args[0], "empty-resource-fault", true, true, true, false, false);
        check(args[0], "empty-owner-fault", true, true, false, true, false);
    }
}
