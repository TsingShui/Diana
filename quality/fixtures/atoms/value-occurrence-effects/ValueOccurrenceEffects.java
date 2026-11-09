package fixture;

public final class ValueOccurrenceEffects {
    public static int visits;
    public static int tick(int x) {
        visits++;
        if (x == 13) throw new ArithmeticException();
        return x + 7;
    }
    public static int pair(int a, int b) { return a * 10 + b; }
    public static void sink(byte[] bytes) { observed = bytes; }
    public static byte[] observed;
    public static int combined(int x, int y) {
        int seed = tick(x);
        int adjusted = (seed + y) * 3;
        return tick(pair(adjusted, adjusted));
    }
    public static int protectedValue(int x) {
        int saved = tick(x);
        try { return tick(saved); }
        catch (ArithmeticException error) { return saved; }
    }
    public static byte[] twoArrays() {
        byte[] first = {1, 2, 3};
        sink(first);
        return new byte[] {4, 5, 6};
    }
}
