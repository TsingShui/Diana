package fixture;

/** Adjacent carriers keep checks/effects at their original positions. */
public final class RefinedCarriers {
    public static int trace;
    public static void mark(int event) { trace = trace * 31 + event; }
    public static Object identity(Object value) { mark(10); return value; }
    public static void tick(int event, int mode) {
        mark(event);
        if (mode == 1) throw new IllegalStateException("tick");
    }
    public static int declared(Object input) {
        String value = (String) identity(input);
        return value.length();
    }
    public static int checkedEarly(Object input, int mode) {
        String value = (String) input;
        tick(1, mode);
        if (mode < 0) return 0;
        return value.length();
    }
    public static byte[] bytes() { return new byte[]{1, 2, 3}; }
    public static byte[] guarded(Object input) {
        byte[] values = new byte[3];
        String checked = (String) input; // Explains the DEX check, which writes no register.
        values[0] = 1; values[1] = 2; values[2] = 3;
        return values;
    }
}
