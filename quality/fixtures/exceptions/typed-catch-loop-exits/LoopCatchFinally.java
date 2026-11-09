package fixture;

/** Shared catch entries and outer cleanup repeated inside a multi-exit loop. */
public final class LoopCatchFinally {
    public static int trace;

    public static void mark(int event) {
        trace = trace * 31 + event;
    }

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

    public static int scan(int rows, int mode) {
        int total = 0;
        for (int row = 0; row < rows; row++) {
            int value;
            try {
                try {
                    value = read(row, mode);
                } catch (NumberFormatException | NullPointerException error) {
                    mark(20);
                    value = 100;
                } catch (RuntimeException error) {
                    mark(21);
                    value = -2;
                }
            } finally {
                cleanup(row, mode);
            }
            if (value < 0) return value;
            if (value == 2) continue;
            total += value;
            if (total >= 200) break;
        }
        return total;
    }

    /**
     * The Smali also retains a redundant NPE clause after RuntimeException,
     * targeting the first handler. Its order must not be changed by grouping.
     */
    public static int priority(int mode) {
        try {
            read(0, mode);
            return 1;
        } catch (NumberFormatException error) {
            return 100;
        } catch (RuntimeException error) {
            return -2;
        }
    }
}
