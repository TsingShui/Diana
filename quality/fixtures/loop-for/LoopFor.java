package fixture;

/** Atomic fixture: classic for, with break and continue in the body. */
public final class LoopFor {
    private static int state;

    public static int sumTo(int limit) {
        int sum = 0;
        for (int index = 0; index < limit; index++) {
            sum += index;
        }
        return sum;
    }

    public static int skipNegative(int[] values) {
        int sum = 0;
        for (int index = 0; index < values.length; index++) {
            if (values[index] < 0) {
                continue;
            }
            sum += values[index];
        }
        return sum;
    }

    public static int stopAt(int[] values, int limit) {
        for (int index = 0; index < values.length; index++) {
            if (values[index] > limit) {
                state = index;
                break;
            }
        }
        return state;
    }
}
