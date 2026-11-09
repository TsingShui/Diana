package fixture;

// This explanation is not the assembly input.
public final class ValueOccurrences {
    public static int visits;
    public static int tick(int x) { visits++; return x + 7; }
    public static int pair(int a, int b) { return a * 10 + b; }
    public static Object identity(Object x) { return x; }
    public static int chain(int a, int b) { return (a + b) * 3; }
    public static int effects(int x) { return pair(tick(x), 2); }
    public static int shared(int x) { int saved = tick(x); return pair(saved, saved); }
    public static int repeated(int x) { return tick(tick(x)); }
    public static byte[] bytes() { return new byte[] {1, 2, 3}; }
    public static int checked(Object x) { return ((String) identity(x)).length(); }
}
