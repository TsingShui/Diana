public final class GenericOperand {
    public static String RESULT;

    static {
        Object saved = GenericEffects.capture();
        RESULT = GenericEffects.select(saved);
    }
}
