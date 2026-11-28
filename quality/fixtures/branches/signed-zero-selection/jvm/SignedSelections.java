public final class SignedSelections {
    public static final float FIELD_NEG_FLOAT = -0.0f;
    public static final float FIELD_POS_FLOAT = 0.0f;
    public static final double FIELD_NEG_DOUBLE = -0.0d;
    public static final double FIELD_POS_DOUBLE = 0.0d;
    public static float staticFloat;
    public static double staticDouble;
    public static int trace;

    public static float selectFloat(boolean negative) {
        return negative ? -0.0f : 0.0f;
    }

    public static double selectDouble(boolean positive) {
        return positive ? 0.0d : -0.0d;
    }

    public static int rawFloat(boolean negative) {
        return Float.floatToRawIntBits(negative ? -0.0f : 0.0f);
    }

    public static long rawDouble(boolean positive) {
        return Double.doubleToRawLongBits(positive ? 0.0d : -0.0d);
    }

    public static void store(boolean first) {
        staticFloat = first ? -0.0f : 0.0f;
        staticDouble = first ? 0.0d : -0.0d;
    }

    public static long nestedDouble(boolean outer, boolean inner) {
        double selected = outer ? (inner ? -0.0d : 0.0d) : -0.0d;
        return Double.doubleToRawLongBits(selected);
    }

    public static int guardedStore(float[] array, int index, boolean negative) {
        trace = 1;
        try {
            array[index] = negative ? -0.0f : 0.0f;
            trace = 12;
            return Float.floatToRawIntBits(array[index]);
        } catch (ArrayIndexOutOfBoundsException fault) {
            trace = 13;
            return 301;
        } catch (NullPointerException fault) {
            trace = 14;
            return 302;
        }
    }

    private static boolean choose(boolean value) {
        trace++;
        return value;
    }

    public static int effectful(boolean negative) {
        return Float.floatToRawIntBits(choose(negative) ? -0.0f : 0.0f);
    }

    public static double sameSign(boolean flag) {
        double value = flag ? -0.0d : -0.0d;
        return value;
    }
}
