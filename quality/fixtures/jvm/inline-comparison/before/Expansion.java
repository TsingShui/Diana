package fixture.inline;

public class Expansion {
    public static int trace;
    public static boolean fail;

    private static void begin() { trace = 1; }
    private static void end(int value) { trace = trace * 10 + value; }

    private static int effect(int digit) {
        trace = trace * 10 + digit;
        if (fail && digit == 4) {
            throw new ArithmeticException("fixture fault");
        }
        return digit;
    }

    private static int noArguments() {
        return effect(2) * 2 + effect(3);
    }

    private static long arguments(int start, long base) {
        return effect(start) + base * 2L;
    }

    private static int originalOrder(int first, int second) {
        return effect(first) + second * 2;
    }

    private static int protectedHelper() {
        return effect(4) * 3 + effect(6);
    }

    public static int run() {
        begin();
        int result = noArguments();
        end(result);
        return result;
    }

    public long forwarded(int start, long base) {
        begin();
        long result = arguments(start, base);
        end((int) result);
        return result;
    }

    public static int changedArguments(int first, int second) {
        begin();
        int result = originalOrder(first, second);
        end(result);
        return result;
    }

    public static int protectedCall() {
        begin();
        try {
            int result = protectedHelper();
            end(result);
            return result;
        } catch (ArithmeticException ignored) {
            return -1;
        }
    }

    public static int unrelated() {
        return 12;
    }
}
