public final class GenericArrayEffects {
    static final IllegalStateException NESTED = new IllegalStateException("nested");
    static final IllegalStateException CAPTURE = new IllegalStateException("capture");
    static final IllegalStateException LEADING = new IllegalStateException("leading");
    static final String[] VALUES = {"value"};
    static final String SCALAR = "scalar";
    static int trace;
    static boolean failLeading;

    public static int leading() {
        trace = trace * 10 + 1;
        if (failLeading) throw LEADING;
        return 42;
    }

    public static int nested(int mode) {
        trace = trace * 10 + 2;
        if (mode == 2) throw NESTED;
        return mode;
    }

    @SuppressWarnings("unchecked")
    public static <T> T capture(int mode) {
        trace = trace * 10 + 3;
        if (mode == 3) throw CAPTURE;
        return mode == 0 ? null : (T) (mode == 6 ? SCALAR : VALUES);
    }

    public static String choose(int first, Object saved) {
        return outcome(first, saved, 4, "object");
    }

    public static String choose(int first, String[] saved) {
        return outcome(first, saved, 5, "array");
    }

    private static String outcome(int first, Object saved, int event, String overload) {
        if (first != 42 || (saved != null && saved != VALUES && saved != SCALAR)) {
            throw new AssertionError("arguments");
        }
        trace = trace * 10 + event;
        if (saved == null) return overload + "-null";
        return overload + (saved == VALUES ? "-value" : "-scalar");
    }
}
