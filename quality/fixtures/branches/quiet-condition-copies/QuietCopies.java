package fixture;

/** Origin-only and executable final-other copies in consecutive conditions. */
public final class QuietCopies {
    public static int trace;
    public static void mark(int event) { trace = trace * 31 + event; }
    public static int seed(int base) { mark(base + 30); return base + 1; }
    public static int quiet(int base, int a, int b, int c) {
        seed(base);
        int selected = 0;
        mark(selected);
        if (a != 0 || b != 0) selected += 7;
        mark(selected);
        int choice = base;
        if (b != 0 || c != 0) choice = 40;
        mark(choice);
        return selected + choice;
    }
    public static int pair(int base, int a, int b, int c) {
        int selected = seed(base);
        mark(selected);
        if (a != 0 || b != 0) selected += 7;
        mark(selected);
        int choice = base;
        if (b != 0 || c != 0) choice = 40;
        mark(choice);
        return selected + choice;
    }
}
