public final class NullEffects {
    static final IllegalStateException LEADING = new IllegalStateException("leading");
    static int trace;

    static String result(Object supplied, String overload, int effect) {
        if (supplied != null) throw new AssertionError("expected null");
        trace = trace * 10 + effect;
        return overload;
    }

    public static int leading(boolean fail) {
        trace = trace * 10 + 5;
        if (fail) throw LEADING;
        return 42;
    }

    public static String recordLeading(IllegalStateException failure) {
        if (failure != LEADING) throw failure;
        trace = trace * 10 + 8;
        return "leading-fault";
    }

    public static <T extends Object & Runnable> String bounded(T supplied) {
        return result(supplied, "bounded", 9);
    }
}
