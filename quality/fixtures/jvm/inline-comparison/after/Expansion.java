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

    public static int run() {
        begin();
        int result = effect(2) * 2 + effect(3);
        end(result);
        return result;
    }

    public long forwarded(int start, long base) {
        begin();
        long result = effect(start) + base * 2L;
        end((int) result);
        return result;
    }

    public static int changedArguments(int first, int second) {
        begin();
        int result = effect(second) + first * 2;
        end(result);
        return result;
    }

    public static int protectedCall() {
        begin();
        try {
            int result = effect(4) * 3 + effect(6);
            end(result);
            return result;
        } catch (ArithmeticException ignored) {
            return -1;
        }
    }

    public static int unrelated() {
        return effect(2) * 2 + effect(3);
    }
}
