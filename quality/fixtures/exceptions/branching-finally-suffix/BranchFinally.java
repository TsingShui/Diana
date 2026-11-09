package fixture;

/** Loop/catch rejoin under a branching finally; all helper effects are concrete. */
public final class BranchFinally {
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

    public static void finish(int code, int mode) {
        mark(30 + code);
        if (mode == 5 || mode == 6) throw new IllegalArgumentException("close");
    }

    public static int run(int rows, int mode, int flag) {
        int row = 0;
        int total = 0;
        try {
            while (row < rows) {
                int value;
                try {
                    value = read(row, mode);
                } catch (NumberFormatException | NullPointerException error) {
                    mark(20);
                    value = 100;
                }
                total += value;
                row++;
            }
        } finally {
            if (flag == 0) {
                mark(40);
            } else {
                mark(41);
                finish(rows, mode);
            }
        }
        return total;
    }

    /** Deliberately different cleanup arguments: this is not one finally. */
    public static int different(int mode) {
        int value;
        try {
            value = read(0, mode);
        } catch (Throwable error) {
            finish(2, mode);
            throw error;
        }
        finish(1, mode);
        return value;
    }

    /** The same outside catch receives a prefix fault and the pending rethrow. */
    public static int guardedRethrow(int mode, int prefixMode) {
        try {
            read(99, prefixMode);
        } catch (Throwable error) {
            return 300;
        }
        int value;
        try {
            value = read(0, mode);
        } catch (Throwable error) {
            mark(0);
            try {
                throw error;
            } catch (Throwable outside) {
                return 300;
            }
        }
        mark(0);
        return value;
    }
}
