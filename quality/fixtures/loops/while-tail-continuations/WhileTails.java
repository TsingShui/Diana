package fixture;

/** Explanation of authored Smali, not an alternative assembly input. */
public final class WhileTails {
    public static int trace;
    public static void mark(int event) { trace = trace * 31 + event; }
    public static void touch(int event, int fault) {
        mark(event);
        if (event == fault) throw new IllegalStateException("tail");
    }
    public static int read(int row, int mode) {
        mark(10 + row);
        if (mode == 1) throw new NumberFormatException();
        if (mode == 4) throw new AssertionError();
        return row + 1;
    }
    public static void cleanup(int row, int mode) {
        mark(30 + row);
        if (mode == 2 || mode == 4) throw new IllegalArgumentException();
    }
    public static int walk(int limit, int step, int mode) {
        int row = 0, total = 0;
        while (row < limit) {
            try {
                int value = read(row, mode);
                if (row != 1) total += value;
            } finally {
                cleanup(row, mode);
            }
            if (mode == 3 && row == 2) break;
            row += step;
        }
        return total;
    }
    public static int tail(int limit, int fault) {
        int row = 0, total = 0;
        do {
            touch(60 + row, fault);
            total += row;
            row++;
        } while (row < limit);
        return total;
    }
    public static int nested(int rows, int hit, int fault) {
        int row = 0, total = 0;
        outer: while (row < rows) {
            int middle = 0;
            while (middle < 2) {
                int column = 0;
                while (column < 3) {
                    if (column == hit) {
                        touch(100 + row * 10 + middle, fault);
                        total += 10;
                        row += 2;
                        continue outer;
                    }
                    total += column + 1;
                    column++;
                }
                middle++;
            }
            touch(200 + row, fault);
            row++;
        }
        return total;
    }

    public static int shared(int rows, int hit, int fault) {
        int row = 0, total = 0;
        outer: while (row < rows) {
            int middle = 0;
            while (middle < 2) {
                int column = 0;
                while (column < 3) {
                    if (column == hit || column == 1) {
                        touch(100 + row * 10 + middle, fault);
                        total += 10;
                        row += 2;
                        continue outer;
                    }
                    total += column + 1;
                    column++;
                }
                middle++;
            }
            touch(200 + row, fault);
            row++;
        }
        return total;
    }
}
