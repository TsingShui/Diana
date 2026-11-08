package fixture;

/**
 * A finally that overwrites the storage used by a pending return and then records
 * the new value. Java evaluates the return operand before entering the finally, so
 * the saved value must be returned even though the local or field changes and the
 * finally observes the new value.
 */
public final class FinallyReturnCapture {
    private static int state;

    public static native void record(int value);

    public static int localReturn(int mode) throws Throwable {
        int result = 1;
        try {
            if (mode == 1) {
                result = 2;
            }
            if (mode == 2) {
                throw new IllegalStateException("body");
            }
            return result;
        } finally {
            result = 3;
            record(result);
        }
    }

    public static int fieldReturn(int mode) throws Throwable {
        state = 10;
        try {
            if (mode == 1) {
                throw new IllegalStateException("body");
            }
            return state;
        } finally {
            state = 99;
            record(state);
        }
    }
}
