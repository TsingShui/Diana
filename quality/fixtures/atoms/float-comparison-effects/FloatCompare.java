package fixture;

public final class FloatCompare {
    public static int trace;

    public static float left() {
        trace = trace * 10 + 1;
        return 2.0f;
    }

    public static float right() {
        trace = trace * 10 + 2;
        return 1.0f;
    }

    public static float leftNaN() {
        trace = trace * 10 + 1;
        return Float.NaN;
    }

    public static float rightMaybe(RuntimeException fault) {
        trace = trace * 10 + 2;
        if (fault != null) {
            throw fault;
        }
        return 1.0f;
    }

    public static double doubleLeft(boolean nan) {
        trace = trace * 10 + 1;
        return nan ? Double.NaN : 2.0;
    }

    public static double doubleRightMaybe(RuntimeException fault) {
        trace = trace * 10 + 2;
        if (fault != null) {
            throw fault;
        }
        return 1.0;
    }

    public static boolean one() {
        float a = left();
        float b = right();
        return compare(a, b, 1) == 1;
    }

    public static boolean zero() {
        float a = left();
        float b = right();
        return compare(a, b, 1) > 0;
    }

    public static int value() {
        float a = left();
        float b = right();
        return compare(a, b, 1);
    }

    public static boolean nanSkips(RuntimeException fault) {
        float a = leftNaN();
        float b = rightMaybe(fault);
        return compare(a, b, 1) == 1;
    }

    public static boolean lowFloatZero(RuntimeException fault) {
        float a = leftNaN();
        float b = rightMaybe(fault);
        return compare(a, b, -1) < 0;
    }

    public static boolean doubleFinite() {
        double a = doubleLeft(false);
        double b = doubleRightMaybe(null);
        return compare(a, b, 1) == 1;
    }

    public static int doubleHigh(RuntimeException fault) {
        double a = doubleLeft(true);
        double b = doubleRightMaybe(fault);
        return compare(a, b, 1);
    }

    public static boolean doubleLow(RuntimeException fault) {
        double a = doubleLeft(true);
        double b = doubleRightMaybe(fault);
        return compare(a, b, -1) == -1;
    }

    private static int compare(double a, double b, int unordered) {
        if (a > b) {
            return 1;
        }
        if (a < b) {
            return -1;
        }
        if (a == b) {
            return 0;
        }
        return unordered;
    }
}
