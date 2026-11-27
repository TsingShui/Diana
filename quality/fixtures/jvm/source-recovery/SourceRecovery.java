package fixture.jvm;

public final class SourceRecovery {
    private int value;
    private String last;

    public SourceRecovery(int initial) {
        value = initial;
    }

    public int calculate(int delta) {
        int combined = value + delta;
        return twice(combined) - 1;
    }

    private static int twice(int input) {
        return input * 2;
    }

    public int choose(int input) {
        if (input > 0) {
            return value + input;
        }
        return value - input;
    }

    public static int descendingSum(int remaining) {
        int total = 0;
        while (remaining > 0) {
            total += remaining;
            remaining--;
        }
        return total;
    }

    public static int divideOrDefault(int numerator, int divisor) {
        try {
            return numerator / divisor;
        } catch (ArithmeticException error) {
            return -7;
        }
    }

    public int update(int delta) {
        value += delta;
        last = "value=" + value;
        value -= 2;
        return value;
    }

    public String last() {
        return last;
    }

    public static int classify(int input) {
        switch (input) {
            case -7:
            case 40:
                return 61;
            case 6:
                return -4;
            default:
                return input + 200;
            case 100:
                return 99;
        }
    }
}
