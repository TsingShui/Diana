package fixture;

/** Atomic fixture: type tests and casts. */
public final class CastInstanceof {
    public static boolean isString(Object value) {
        return value instanceof String;
    }

    public static int castToNumber(Object value) {
        return ((Number) value).intValue();
    }

    public static String castAfterTest(Object value) {
        if (value instanceof CharSequence) {
            return value.toString();
        }
        return null;
    }

    public static Object arrayCast(Object value) {
        return (int[]) value;
    }
}
