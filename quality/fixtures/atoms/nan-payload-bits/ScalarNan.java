package fixture;

public final class ScalarNan {
    public static double stored;
    public static long lastBits;
    public static ArithmeticException caught;

    public static float floatValue() {
        return Float.intBitsToFloat(0x7fc01234);
    }

    public static double doubleValue() {
        return Double.longBitsToDouble(0xfff8000000005678L);
    }

    public static int bits(boolean positive) {
        float selected = Float.intBitsToFloat(positive ? 0x7fc01234 : 0xffc04321);
        return Float.floatToRawIntBits(selected);
    }

    public static void store() {
        stored = Double.longBitsToDouble(0xfff8000000005678L);
    }

    public static float canonicalFloat() {
        return Float.NaN;
    }

    public static long inspectDouble(double value, ArithmeticException fault) {
        long bits = Double.doubleToRawLongBits(value);
        lastBits = bits;
        if (fault != null)
            throw fault;
        return bits;
    }

    public static long protectedBits(boolean positive, ArithmeticException fault) {
        double selected = Double.longBitsToDouble(positive ? 0x7ff8000000009abcL : 0xfff8000000005678L);
        try {
            return inspectDouble(selected, fault);
        } catch (ArithmeticException exception) {
            caught = exception;
            return lastBits;
        }
    }
}
