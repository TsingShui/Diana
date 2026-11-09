package fixture;

/** The update reads a body-mutated local after cleanup, not a frozen stride. */
public final class MutableStep {
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

    public static int walk(int limit, int mode, int flag) {
        int total = 0;
        int step = 1;
        for (int row = 0; row < limit; row += step) {
            int value;
            try {
                try { value = read(row, mode); }
                catch (NumberFormatException | NullPointerException error) {
                    mark(20);
                    value = 100;
                }
                step = row == 0 ? 2 : 1;
            } finally { cleanup(row, mode); }
            if (flag != 0 && row == 0) continue;
            if (row == 3) break;
            if (flag == 2 && row == 2) return total;
            total += value;
        }
        return total;
    }

    /** Nonlocal condition outlets must not become the inner for's fallthrough. */
    public static int nested(int rows, int step, int flag) {
        int total = 0;
        outer: for (int row = 0; row < rows; row++) {
            for (int column = 0; column < 3; column += step) {
                mark(row * 10 + column);
                if (flag == 1 && column == 1) break outer;
                if (flag == 2 && column == 0) continue outer;
                if (column == 2) continue;
                total++;
            }
            mark(90 + row);
        }
        return total;
    }

    /** A potentially throwing update stays outside pure-update admission. */
    public static int divide(int limit, int divisor) {
        int row = 4;
        while (row > limit) {
            mark(row);
            if (row != 4) mark(99);
            row /= divisor;
        }
        return row;
    }
}
