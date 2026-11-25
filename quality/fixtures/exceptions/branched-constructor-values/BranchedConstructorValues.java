package fixture;

public final class BranchedConstructorValues {
    private final int value;

    private BranchedConstructorValues(int base) {
        record(base);
        value = base;
    }

    private BranchedConstructorValues(int base, int adjustment) {
        int sum = base + adjustment;
        record(sum);
        value = sum;
    }

    private static native void record(int value);
    private static native int consume(int first, int second);
    private static native int recover(int first);

    public static int run(boolean firstShort, boolean secondShort) {
        BranchedConstructorValues first = firstShort
                ? new BranchedConstructorValues(3) : new BranchedConstructorValues(3, 4);
        try {
            BranchedConstructorValues second = secondShort
                    ? new BranchedConstructorValues(5) : new BranchedConstructorValues(5, 6);
            return consume(first.value, second.value);
        } catch (IllegalStateException failure) {
            return recover(first.value);
        }
    }
}
