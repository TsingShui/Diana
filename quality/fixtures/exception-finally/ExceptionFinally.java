package fixture;

/** Atomic fixture: finally on the normal path and on the throwing path. */
public final class ExceptionFinally {
    private static int state;

    public static int normalPath(int value) {
        int acc = 0;
        try {
            acc = value;
        } finally {
            state = 1;
            acc++;
        }
        return acc;
    }

    public static int throwingPath(int divisor) {
        int acc = 0;
        try {
            acc = 10 / divisor;
            return acc;
        } finally {
            state = 2;
        }
    }
}
