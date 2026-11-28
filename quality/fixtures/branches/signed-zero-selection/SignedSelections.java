package fixture;

public final class SignedSelections {
    public static float staticFloat;
    public static double staticDouble;
    public static int lastBits;
    public static ArithmeticException caught;

    public static int rawFloat(boolean negative) {
        return Float.floatToRawIntBits(negative ? -0.0f : 0.0f);
    }

    public static long rawDouble(boolean positive) {
        return Double.doubleToRawLongBits(positive ? 0.0d : -0.0d);
    }

    public static void storeFloat(boolean negative) {
        staticFloat = negative ? -0.0f : 0.0f;
    }

    public static void storeDouble(boolean positive) {
        staticDouble = positive ? 0.0d : -0.0d;
    }

    public static float selectFloat(boolean negative) {
        if (negative)
            return -0.0f;
        return 0.0f;
    }

    public static int inspectFloat(float value, ArithmeticException fault) {
        int bits = Float.floatToRawIntBits(value);
        lastBits = bits;
        if (fault != null)
            throw fault;
        return bits;
    }

    public static int protectedUse(boolean negative, ArithmeticException fault) {
        float value = negative ? -0.0f : 0.0f;
        try {
            return inspectFloat(value, fault);
        } catch (ArithmeticException exception) {
            caught = exception;
            return lastBits;
        }
    }
}
