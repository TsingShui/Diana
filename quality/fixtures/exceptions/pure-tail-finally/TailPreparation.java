package fixture;

/** Pure normal completion belongs before cleanup, including actual Phi copies. */
public final class TailPreparation {
    public static int trace;
    public static void mark(int event) { trace = trace * 31 + event; }
    public static int read(int code, int mode) {
        mark(10 + code);
        if (mode == 1) throw new NumberFormatException("row");
        if (mode == 2) throw new NullPointerException("row");
        if (mode == 3) throw new IllegalStateException("row");
        if (mode == 4 || mode == 6 || mode == 8) throw new AssertionError("row");
        return code + 1;
    }
    public static void cleanup(int code, int mode) {
        mark(30 + code);
        if (mode == 5 || mode == 6 || ((mode == 7 || mode == 8) && code == 3))
            throw new IllegalArgumentException("close");
    }
    public static int finish(int code, int mode, int flag, int a, int b) {
        int adjusted;
        try {
            int value;
            try { value = read(code, mode); }
            catch (NumberFormatException | NullPointerException error) {
                mark(20);
                value = 100;
            }
            if (flag == 0) value += 1;
            else value ^= 5;
            int delta = flag;
            if (delta >= 0) delta = 2;
            adjusted = value + delta;
        } finally { cleanup(code, mode); }
        if (a != 0 || b != 0) mark(50);
        return adjusted;
    }
    public static int literal(int code, int mode, int flag, int a, int b) {
        int adjusted;
        try {
            int value;
            try { value = read(code, mode); }
            catch (NumberFormatException | NullPointerException error) {
                mark(20);
                value = 100;
            }
            if (flag == 0) value += 1;
            else value ^= 5;
            int delta = flag;
            if (delta >= 0) delta = 2;
            adjusted = (value + delta) ^ 1;
        } finally { cleanup(7, mode); cleanup(code, mode); }
        if (a != 0 || b != 0) mark(50);
        return adjusted;
    }
    public static int bypass(int code, int mode, int flag, int a, int b) {
        int value;
        try {
            body: {
                switch (flag) {
                    case 0: mark(9); value = -1; break body;
                    case 1: mark(1); break;
                    case 2: mark(2); break;
                    default: break;
                }
                try { value = read(code, mode); }
                catch (NumberFormatException | NullPointerException error) {
                    mark(20);
                    value = 100;
                }
                value += 2;
            }
        } finally { cleanup(code, mode); }
        value += 3;
        if (a != 0 || b != 0) mark(50);
        return value;
    }
    public static int foreign(int code, int mode, int flag) throws Throwable {
        int value;
        if (flag == 0) {
            try { value = read(code, mode); }
            catch (Throwable error) { cleanup(code, mode); throw error; }
        } else value = read(code + 1, mode);
        value += 1;
        cleanup(code, mode);
        mark(50);
        return value;
    }
}
