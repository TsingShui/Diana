package fixture;

/** Shared ordinary iteration tails, nested cleanup and two deliberate barriers. */
public final class IterationTail {
    public static int trace;
    public static void mark(int event) { trace = trace * 31 + event; }
    public static int read(int code, int mode) {
        mark(10 + code);
        if (mode == 1) throw new NumberFormatException("row");
        if (mode == 2) throw new NullPointerException("row");
        if (mode == 3) throw new IllegalStateException("row");
        if (mode == 4 || mode == 6) throw new AssertionError("row");
        return code + 1;
    }
    public static void cleanup(int code, int mode) {
        mark(30 + code);
        if (mode == 5 || mode == 6) throw new IllegalArgumentException("close");
    }

    public static int nested(int rows, int mode, int flag) {
        int total = 0;
        for (int row = 0; row < rows; row++) {
            if (row == 1 && flag != 0) continue;
            for (int column = 0; column < 3; column++) {
                int code = row * 3 + column;
                int value;
                try {
                    try { value = read(code, mode); }
                    catch (NumberFormatException | NullPointerException error) {
                        mark(20);
                        value = 100;
                    } catch (RuntimeException error) {
                        mark(21);
                        value = -2;
                    }
                } finally { cleanup(code, mode); }
                if (value < 0) return value;
                if (value == 2) continue;
                total += value;
                if (value == 5) break;
            }
        }
        return total;
    }

    /** A shared update reads the original nonconstant step at that point. */
    public static int stride(int limit, int step) {
        int row = 0;
        int total = 0;
        while (row < limit) {
            mark(row);
            if (row != 0) total += row;
            row += step;
        }
        return total;
    }

    /** The direct header continuation deliberately skips the row update. */
    public static int skipUpdate(int limit, int flag) {
        int row = 0;
        int total = 0;
        while (row < limit) {
            mark(50 + row);
            if (flag != 0) {
                total++;
                if (total == 2) return total;
                continue;
            }
            row++;
        }
        return total;
    }

    /** Specialized monitor exit routing remains explicit, not a hidden update. */
    public static int locked(Object lock, int limit) {
        int row;
        for (row = 0; row < limit; row++) {
            synchronized (lock) { mark(row); }
        }
        return row;
    }
}
