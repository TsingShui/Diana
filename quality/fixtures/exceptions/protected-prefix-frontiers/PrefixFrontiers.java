package fixture;

/** Guarded cores retain their entry when optional predecessor growth diverges. */
public final class PrefixFrontiers {
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

    public static int branch(int lead, int flag, int mode) {
        int result;
        try {
            switch (lead) {
                case 0: mark(1); break;
                case 1: mark(2); break;
                default: mark(3); break;
            }
            try {
                if (flag == 0) result = read(0, mode);
                else result = read(1, mode);
            } catch (NumberFormatException | NullPointerException error) {
                mark(20);
                result = 100;
            }
            mark(40);
        } finally { cleanup(2, mode); }
        mark(50);
        return result;
    }

    public static int split(int flag, int mode) {
        int result;
        try {
            // Smali shares one capture/body for these two source explanations.
            if (flag == 0) {
                mark(60);
                try { result = read(0, mode); }
                catch (NumberFormatException | NullPointerException error) {
                    mark(20); result = 100;
                }
            } else {
                mark(61);
                try { result = read(1, mode); }
                catch (NumberFormatException | NullPointerException error) {
                    mark(20); result = 100;
                }
            }
            mark(40);
        } finally { cleanup(3, mode); }
        mark(50);
        return result;
    }
}
