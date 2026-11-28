public final class RuntimeFields {
    public static final String COMPILE_TEXT = "compiled";
    public static final int COMPILE_NUMBER = 42;
    public static final String RUNTIME_TEXT;
    public static final int RUNTIME_NUMBER;

    static {
        RUNTIME_TEXT = "runtime";
        RUNTIME_NUMBER = 7;
        InitEffects.begin(RUNTIME_TEXT, RUNTIME_NUMBER);
        InitEffects.complete();
    }
}
