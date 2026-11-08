package fixture;

/** Atomic fixture: wide and floating-point arithmetic, comparisons and casts. */
public final class ExprWide {
    public static long shifts(long value, int count) {
        return (value << 3) ^ (value >> count);
    }

    public static long arithmetic(long left, long right) {
        return (left / right) + (left % right) * (left - right);
    }

    public static double floating(double left, int right) {
        return left * right + left / 3.0 - right;
    }

    public static float single(float value) {
        return value * 1.5f + 0.25f;
    }

    public static boolean nanComparison(double value) {
        return value != value || value < 0.0 || value > 1.0;
    }

    public static int narrowing(long value) {
        byte small = (byte) value;
        short medium = (short) (value >> 8);
        return small + medium;
    }

    public static long roundTrip(double value) {
        return Math.round(value) + (long) value;
    }
}
