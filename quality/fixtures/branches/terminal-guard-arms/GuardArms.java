package fixture;

/** Explanation of the authored Smali, not an alternative build input. */
public final class GuardArms {
    public static int trace;
    public static void mark(int event) { trace = trace * 31 + event; }
    public static boolean probe(int code, int mode) {
        mark(10 + code);
        if (mode == 9) throw new IllegalStateException("test");
        return code == 0;
    }
    public static int read(int code, int mode) {
        mark(20 + code);
        if (mode == 1) throw new IllegalStateException();
        if (mode == 3) throw new AssertionError();
        return code + 7;
    }
    public static void cleanup(int code, int mode) {
        mark(40 + code);
        if (mode == 2 || mode == 3) throw new IllegalArgumentException();
    }
    public static int declared(int flag, int mode) {
        if (probe(flag, mode)) return 0;
        int value = read(2, mode);
        mark(value);
        return value + 1;
    }
    public static int after(int code, int mode) {
        int value;
        try { value = read(code, mode); }
        finally { cleanup(code, mode); }
        if (probe(value - 8, 0)) return value;
        mark(50);
        return value + 1;
    }
    public static int throwing(int flag, int mode) {
        if (probe(flag, mode)) throw new IllegalStateException("guard");
        mark(50);
        return read(flag, mode);
    }
    public static int select(int flag, int base, int mode) {
        int saved = base, selected = base;
        if (flag == 0) selected = 9;
        else {
            if (probe(flag - 1, mode)) return read(1, mode);
        }
        mark(50);
        return selected + saved;
    }
    public static int outer(int flag, int which, int mode) {
        int selected;
        if (probe(flag, mode)) {
            mark(30);
            selected = 7;
        } else {
            if (probe(which, mode)) return read(3, mode);
            mark(50);
            selected = 8;
        }
        mark(70);
        return selected;
    }
    public static int recover(int flag, int mode) {
        if (probe(flag, mode)) {
            try {
                return read(2, mode);
            } catch (IllegalStateException failure) {
                mark(60);
            }
        } else {
            mark(50);
        }
        mark(70);
        return 4;
    }
    public static int loop(int limit, int hit, int mode) {
        int row = 0, total = 0;
        try {
            while (row < limit) {
                if (probe(row - hit, mode)) {
                    throw new IllegalStateException("guard");
                } else {
                    total += read(row, mode);
                    row++;
                }
            }
        } finally {
            cleanup(limit, mode);
        }
        return total;
    }
    public static void voided(int flag) {
        if (probe(flag, 0)) {
            mark(30);
            return;
        } else {
            mark(50);
        }
        mark(70);
    }
}
