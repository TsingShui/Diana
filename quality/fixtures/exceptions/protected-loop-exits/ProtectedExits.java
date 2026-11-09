package fixture;

public final class ProtectedExits {
    public static int trace;
    public static void mark(int event) { trace = trace * 31 + event; }
    public static int probe(int key, int stop, int mode) {
        mark(100 + key);
        if (mode == 1 && key == 1) throw new NumberFormatException("probe");
        return key == stop ? 1 : 0;
    }
    public static int read(int key, int mode) {
        mark(200 + key);
        if ((mode == 2 || mode == 5) && key == 1) throw new AssertionError("read");
        return key + 1;
    }
    public static void cleanup(int code, int mode) {
        mark(300 + code);
        if ((mode == 3 && code == 7) || ((mode == 4 || mode == 5) && code == 8))
            throw new IllegalArgumentException("cleanup");
    }
    public static int nested(int rows, int cols, int stop, int mode) {
        int row = 0, sum = 0;
        try {
            while (row < rows) {
                int col = 0;
                while (col < cols) {
                    int key = row * 10 + col;
                    if (probe(key, stop, mode) != 0) throw new IllegalStateException("guard");
                    sum += read(key, mode);
                    col += 1;
                }
                row += 1;
            }
        } finally { cleanup(7, mode); cleanup(8, mode); }
        return sum;
    }
    public static int early(int rows, int stop, int mode) {
        int row = 0, sum = 0;
        try {
            while (row < rows) {
                if (row == stop) return sum; // Deliberately skips cleanup.
                sum += read(row, mode);
                row += 1;
            }
        } catch (Throwable pending) { cleanup(7, mode); cleanup(8, mode); throw pending; }
        cleanup(7, mode); cleanup(8, mode);
        return sum;
    }
    public static int switchExit(int rows, int key, int bound, int mode) {
        int row = 0, sum = 0;
        try {
            outer: while (row < rows) {
                int count = 0;
                if (key == 2) { mark(30); sum += 3; }
                else if (key == 0 || key == 1) {
                    // Smali fixes one shared cyclic case-entry graph.
                    boolean first = key == 0;
                    do {
                        if (first) { mark(10); sum += 1; count += 1; }
                        mark(20); sum += 2; count += 1;
                        first = true;
                    } while (count < bound);
                } else { mark(90); break outer; }
                row += 1;
            }
        } finally { cleanup(7, mode); cleanup(8, mode); }
        return sum;
    }
    public static int recover(int rows, int stop, int mode) {
        int row = 0, sum = 0;
        while (row < rows) {
            try {
                if (probe(row, stop, mode) != 0) throw new IllegalStateException("guard");
                sum += read(row, mode);
            } catch (Throwable pending) {
                cleanup(7, mode); cleanup(8, mode);
                row += 2;
                continue;
            }
            row += 1;
        }
        return sum;
    }
    public static int foreign(int rows, int stop, int mode, int foreign) {
        int row = 0, sum = 0;
        // The Smali has one shared throw tail, entered directly before the loop
        // or by its probe branch. This flag explains that same execution order.
        try {
            boolean failed = foreign != 0;
            while (!failed && row < rows) {
                if (probe(row, stop, mode) != 0) { failed = true; break; }
                sum += read(row, mode);
                row += 1;
            }
            if (failed) throw new IllegalStateException("guard");
        } finally { cleanup(7, mode); cleanup(8, mode); }
        return sum;
    }
}
