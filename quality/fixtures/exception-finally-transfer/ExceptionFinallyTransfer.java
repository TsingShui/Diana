package fixture;

/** Atomic fixture: finally with return/break/continue, and try inside catch. */
public final class ExceptionFinallyTransfer {
    private static int state;

    public static int returnInFinally(int value) {
        try {
            state = 1;
            return value;
        } finally {
            state = 2;
            if (value < 0) {
                return -1;
            }
        }
    }

    public static int breakInFinally(int[] values) {
        int total = 0;
        for (int index = 0; index < values.length; index++) {
            try {
                total += values[index];
            } finally {
                if (values[index] < 0) {
                    state = 3;
                    break;
                }
                state = 4;
            }
        }
        return total;
    }

    public static int continueInFinally(int[] values) {
        int total = 0;
        for (int index = 0; index < values.length; index++) {
            try {
                total += values[index];
                if (values[index] == 0) {
                    continue;
                }
                total++;
            } finally {
                state = 5;
            }
        }
        return total;
    }

    public static int tryInsideCatch(String text) {
        try {
            return Integer.parseInt(text);
        } catch (NumberFormatException error) {
            try {
                return Integer.parseInt(text.trim());
            } catch (NumberFormatException nested) {
                state = 6;
                return -2;
            }
        }
    }
}
