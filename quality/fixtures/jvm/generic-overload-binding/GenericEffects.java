public final class GenericEffects {
    static int trace;
    static final String TEXT = new String("captured");

    @SuppressWarnings("unchecked")
    public static <T> T capture() {
        trace = trace * 10 + 1;
        return (T) TEXT;
    }

    public static String select(Object saved) {
        return record(saved, 2, "object");
    }

    public static String select(String saved) {
        return record(saved, 3, "string");
    }

    private static String record(Object saved, int operation, String result) {
        if (saved != TEXT) throw new AssertionError("argument identity");
        trace = trace * 10 + operation;
        return result;
    }
}
