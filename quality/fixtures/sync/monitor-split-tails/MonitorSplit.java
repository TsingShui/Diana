package fixture;

/** Three unlocked inner tails remain held by the independently captured outer lock. */
public final class MonitorSplit {
    public static int trace;
    public static void mark(int event) { trace = trace * 31 + event; }
    public static int read(int row, int mode) {
        mark(10 + row);
        if (mode == 1) throw new NumberFormatException("row");
        if (mode == 2) throw new NullPointerException("row");
        if (mode == 3) throw new IllegalStateException("row");
        if (mode == 4 || mode == 6) throw new AssertionError("row");
        return 5;
    }
    public static void cleanup(int row, int mode) {
        mark(30 + row);
        if (mode == 5 || mode == 6) throw new IllegalArgumentException("close");
    }
    public static int finish(int route, int value, int mode) {
        mark(40 + route);
        if (mode == 7) throw new IllegalStateException("tail");
        if (route == 0) return value + 1;
        if (route == 1) return value * 2;
        return value - 3;
    }

    public static int transfer(Object lock, int flag, int value) {
        int selected;
        synchronized (lock) {
            mark(5);
            if (flag == 0) selected = value;
            else selected = value + 1;
        }
        return selected + value;
    }

    public static int split(Object outer, Object inner, int route, int mode) {
        synchronized (outer) {
            int value;
            synchronized (inner) {
                try {
                    try { value = read(0, mode); }
                    catch (NumberFormatException | NullPointerException error) {
                        mark(20);
                        value = 100;
                    }
                } finally { cleanup(0, mode); }
            }
            // Smali selects one of three release/tail outlets while inner is held.
            // These pure tests of an unchanged parameter have no observable effects.
            if (route == 0) return finish(0, value, mode);
            if (route == 1) return finish(1, value, mode);
            return finish(2, value, mode);
        }
    }
}
