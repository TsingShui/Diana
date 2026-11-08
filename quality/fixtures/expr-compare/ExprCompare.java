package fixture;

/** Atomic fixture: comparisons and boolean composition. */
public final class ExprCompare {
    public static boolean lessThan(int left, int right) {
        return left < right;
    }

    public static boolean atLeast(long left, long right) {
        return left >= right;
    }

    public static boolean same(Object left, Object right) {
        return left == right;
    }

    public static boolean different(int left, int right) {
        return left != right;
    }

    public static boolean both(boolean first, boolean second) {
        return first && second;
    }

    public static boolean either(boolean first, boolean second) {
        return first || second;
    }

    public static boolean negated(boolean value) {
        return !value;
    }
}
