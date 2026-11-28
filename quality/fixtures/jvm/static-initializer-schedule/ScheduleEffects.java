public final class ScheduleEffects {
    static final Object CAPTURED = new Object();
    static final Object OTHER = new Object();
    static final IllegalStateException CAPTURE_FAULT = new IllegalStateException("capture");
    static final IllegalStateException EARLIER_FAULT = new IllegalStateException("earlier");
    static final IllegalStateException AFTER_FAULT = new IllegalStateException("after");
    static int mode;
    static int trace;

    public static Object capture() {
        trace = trace * 10 + 1;
        if ((mode & 1) != 0) throw CAPTURE_FAULT;
        return CAPTURED;
    }

    public static boolean captureBoolean() {
        capture();
        return true;
    }

    public static Object earlier() {
        trace = trace * 10 + 2;
        if ((mode & 2) != 0) throw EARLIER_FAULT;
        return OTHER;
    }

    public static Object join(Object first, Object saved) {
        trace = trace * 10 + 3;
        if (first != OTHER || saved != CAPTURED) throw new AssertionError("argument identity");
        return saved;
    }

    public static Object joinFirst(Object saved, Object second) {
        trace = trace * 10 + 3;
        if (saved != CAPTURED || second != OTHER) throw new AssertionError("argument identity");
        return saved;
    }

    public static boolean flag() {
        trace = trace * 10 + 2;
        return (mode & 8) != 0;
    }

    public static Object fallback() {
        trace = trace * 10 + 3;
        return OTHER;
    }

    public static void after(Object saved) {
        if (saved != CAPTURED) throw new AssertionError("published identity");
        trace = trace * 10 + 4;
        if ((mode & 4) != 0) throw AFTER_FAULT;
    }

    public static void record(IllegalStateException failure) {
        trace = trace * 10 + (failure == AFTER_FAULT ? 5 : 9);
    }
}
