public final class GenericArrayOperand {
    public static String object(int mode) {
        int first = GenericArrayEffects.leading();
        Object saved = GenericArrayEffects.capture(GenericArrayEffects.nested(mode));
        return GenericArrayEffects.choose(first, saved);
    }

    public static String array(int mode) {
        int first = GenericArrayEffects.leading();
        String[] saved = GenericArrayEffects.capture(GenericArrayEffects.nested(mode));
        return GenericArrayEffects.choose(first, saved);
    }
}
