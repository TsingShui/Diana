package fixture;

/** Monitor release roles finalized against the selected iteration continuation. */
public final class MonitorIteration {
    public static int trace;
    public static void mark(int event) { trace = trace * 31 + event; }
    public static int read(int row, int mode) {
        mark(10 + row);
        if (mode == 1) throw new NumberFormatException("row");
        if (mode == 2) throw new NullPointerException("row");
        if (mode == 3) throw new IllegalStateException("row");
        if (mode == 4 || mode == 6) throw new AssertionError("row");
        return row + 1;
    }
    public static void cleanup(int row, int mode) {
        mark(30 + row);
        if (mode == 5 || mode == 6) throw new IllegalArgumentException("close");
    }

    public static int heldCleanup(Object lock, int rows, int mode) {
        int total = 0;
        for (int row = 0; row < rows; row++) {
            synchronized (lock) {
                int value;
                try {
                    try { value = read(row, mode); }
                    catch (NumberFormatException | NullPointerException error) {
                        mark(20);
                        value = 100;
                    } catch (RuntimeException error) {
                        mark(21);
                        value = -2;
                    }
                } finally { cleanup(row, mode); }
                if (value == 2) continue;
                total += value;
            }
        }
        return total;
    }

    public static int walk(Object lock, int rows, int mode) {
        int total = 0;
        for (int row = 0; row < rows; row++) {
            synchronized (lock) {
                int value;
                try {
                    try { value = read(row, mode); }
                    catch (NumberFormatException | NullPointerException error) {
                        mark(20);
                        value = 100;
                    } catch (RuntimeException error) {
                        mark(21);
                        value = -2;
                    }
                } finally { cleanup(row, mode); }
                if (value < 0) return value;
                if (value == 2) continue;
                total += value;
                if (total >= 200) break;
            }
        }
        return total;
    }

    /** One original backedge skips row++; a monitor does not authorize changing it. */
    public static int bypass(Object lock, int limit, int flag) {
        int row = 0;
        int seen = 0;
        while (row < limit) {
            synchronized (lock) {
                mark(50 + row);
                if (flag != 0) {
                    seen++;
                    if (seen == 2) return seen;
                    continue;
                }
            }
            row++;
        }
        return seen;
    }
}
