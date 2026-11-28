public final class NullableOverload<T> {
    public static String object(NullableOverload<Number> box) {
        return box.pick((Number) null);
    }

    public static String integer(NullableOverload<Number> box) {
        return box.pick((Integer) null);
    }

    public static String guarded(NullableOverload<Number[]> box, boolean fail) {
        try {
            return box.choose(NullEffects.leading(fail), (Number[]) null);
        } catch (IllegalStateException failure) {
            return NullEffects.recordLeading(failure);
        }
    }

    public static String bounded() {
        return NullEffects.bounded(null);
    }

    // BEGIN OVERLOAD DEFINITIONS
    public String pick(T supplied) {
        return NullEffects.result(supplied, "object", 1);
    }

    public String pick(Integer supplied) {
        return NullEffects.result(supplied, "integer", 2);
    }

    public String choose(int leading, T supplied) {
        if (leading != 42) throw new AssertionError("leading argument");
        return NullEffects.result(supplied, "guarded-object", 6);
    }

    public String choose(int leading, Integer... supplied) {
        if (leading != 42) throw new AssertionError("leading argument");
        return NullEffects.result(supplied, "guarded-array", 7);
    }
    // END OVERLOAD DEFINITIONS
}
