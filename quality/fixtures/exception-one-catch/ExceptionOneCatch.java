package fixture;

/** Atomic fixture: one try, one catch, different catch bodies. */
public final class ExceptionOneCatch {
    private static int state;

    public static int catchReturns(int divisor) {
        try {
            return 10 / divisor;
        } catch (ArithmeticException error) {
            return -1;
        }
    }

    public static void catchRethrows(int divisor) {
        try {
            state = 10 / divisor;
        } catch (ArithmeticException error) {
            state = 2;
            throw error;
        }
    }

    public static int sideEffectsInTry(int value) {
        try {
            state = value;
            state = state + 1;
            return state;
        } catch (RuntimeException error) {
            state = -1;
            return state;
        }
    }
}
