package fixture;

/** Atomic fixture: one try, one multi-catch clause, one caught rethrow. */
public final class ExceptionMultiCatch {
    private static int state;

    public static int parse(Object input) {
        try {
            return Integer.parseInt(input.toString());
        } catch (NumberFormatException | NullPointerException error) {
            state = 1;
            return -1;
        }
    }

    public static int rethrow(Object input) {
        try {
            return Integer.parseInt(input.toString());
        } catch (NumberFormatException | IllegalStateException error) {
            state = 2;
            throw new IllegalStateException(error);
        }
    }
}
