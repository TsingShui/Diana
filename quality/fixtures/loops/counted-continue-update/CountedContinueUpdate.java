package fixture;

public final class CountedContinueUpdate {
    public static int state;
    public static int events;
    public static int lastValue;

    public static int advanceThenStop(int[] values, int limit, ArithmeticException fault) {
        for (int index = 0; index < values.length; index++) {
            int value = values[index];
            if (value > limit) {
                state = index;
                break;
            }
            if (value != 0) {
                record(index, value, fault);
            }
        }
        return state;
    }

    public static int holdOnZero(int[] values) {
        int index = 0;
        while (index < values.length) {
            int value = values[index];
            if (value == 0) {
                values[index] = 1;
                continue;
            }
            if (value < 0) {
                state = index;
                break;
            }
            record(index, value, null);
            index++;
        }
        return state;
    }

    public static int observeAfterAdvance(int[] values, ArithmeticException fault) {
        int index = 0;
        while (index < values.length) {
            int value = values[index];
            if (value < 0) {
                state = index;
                break;
            }
            index++;
            if (value != 0) {
                record(index, value, fault);
            }
        }
        return state;
    }

    private static void record(int index, int value, ArithmeticException fault) {
        events = (events * 31 + index + 1) * 31 + value;
        lastValue = value;
        if (index == 1 && fault != null) {
            throw fault;
        }
    }
}
