package fixture;

/** Atomic fixture: every constant form d8 can encode, nothing else. */
public final class ExprConstants {
    private static final String LABEL = "label";

    public static int intValue() {
        return 123456;
    }

    public static long longValue() {
        return 1234567890123L;
    }

    public static float floatValue() {
        return 1.5f;
    }

    public static double doubleValue() {
        return 2.25;
    }

    public static char charValue() {
        return 'x';
    }

    public static boolean booleanValue() {
        return false;
    }

    public static String stringValue() {
        return LABEL;
    }

    public static Object nullValue() {
        return null;
    }

    public static Class<?> classValue() {
        return ExprConstants.class;
    }
}
