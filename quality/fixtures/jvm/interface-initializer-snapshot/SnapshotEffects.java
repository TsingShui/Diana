public final class SnapshotEffects {
    static final Object BEFORE = new Object();
    static final Object AFTER = new Object();
    static final IllegalStateException FAULT = new IllegalStateException("snapshot");
    public static volatile Object INSTANCE = BEFORE;
    public static boolean throwOnReplace;
    static int trace;

    public static void replace(Object saved, boolean fail) {
        trace = trace * 10 + (saved == BEFORE ? 1 : 8);
        INSTANCE = AFTER;
        if (fail) {
            throw FAULT;
        }
    }

    public static void record(Object saved, IllegalStateException fault) {
        trace = trace * 10 + (saved == BEFORE ? 2 : 9);
        trace = trace * 10 + (fault == FAULT ? 4 : 7);
    }
}

final class StableOwner {
    static final Object INSTANCE = initialize();

    private static Object initialize() {
        SnapshotEffects.trace = SnapshotEffects.trace * 10 + 3;
        return SnapshotEffects.BEFORE;
    }
}
