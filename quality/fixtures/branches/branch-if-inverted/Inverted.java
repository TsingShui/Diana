package fixture;

/** Conditional branches with empty arms, calls, copies, exceptions and loops. */
public final class Inverted {
    public static native void observe(int value);
    public static native int probe();

    public static void invertedVoid(int x) {
        if (x != 0) observe(x);
    }
    public static void invertedNull(Object value, int x) {
        if (value != null) observe(x);
    }
    public static void invertedEmptyBlock(int x) {
        if (x != 0) observe(x);
    }
    public static void invertedProbe() {
        int value = probe();
        if (value != 0) observe(value);
    }
    public static void invertedFloat(float x, float y) {
        // The condition is false when either operand is NaN.
        if (x < y) observe(-1);
    }
    public static int plain(int x) {
        int result = 0;
        if (x != 0) { observe(x); result = 1; }
        return result;
    }
    public static int effectPredicate() {
        int result = 0;
        int value = probe();
        if (value != 0) { observe(value); result = 1; }
        return result;
    }
    public static int nullCheck(Object value) {
        return value == null ? 0 : 1;
    }
    public static int twoArms(int x) {
        int result = x == 0 ? 2 : 1;
        observe(result);
        return result;
    }
    public static int nested(int x, int y) {
        if (x != 0 && y != 0) { observe(y); return 1; }
        return 0;
    }
    public static int floatCmp(float x, float y) {
        return !Float.isNaN(x) && !Float.isNaN(y) && x < y ? 1 : 0;
    }
    public static int guardedCall(int x) {
        try {
            if (x != 0) { observe(x); return 1; }
            return 0;
        } catch (RuntimeException failure) { return -1; }
    }
    public static int loopBreak(int limit) {
        int value = 0;
        while (value < limit) {
            value++;
            if (value == limit) { observe(value); break; }
        }
        return value;
    }
}
