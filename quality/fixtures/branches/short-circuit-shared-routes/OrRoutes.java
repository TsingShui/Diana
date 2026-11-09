package fixture;

/** Explanation of the independently authored Smali; not a second build input. */
public final class OrRoutes {
    public static int trace;

    public static void mark(int event) { trace = trace * 31 + event; }

    public static boolean probe(int stage, int flags, int fault) {
        mark(10 + stage);
        if (stage == fault) throw new IllegalStateException("probe");
        return (flags & (1 << stage)) != 0;
    }

    public static int ordered(int flags, int fault) {
        int result;
        if (probe(0, flags, fault) || probe(1, flags, fault)
                || probe(2, flags, fault)) {
            mark(40);
            result = 7;
        } else {
            mark(50);
            result = 9;
        }
        mark(60);
        return result;
    }

    public static int effectful(int flags) {
        boolean selected = probe(0, flags, -1);
        if (!selected && probe(1, flags, -1)) {
            mark(30);
            selected = true;
        }
        mark(selected ? 40 : 50);
        return selected ? 7 : 9;
    }

    public static int blocked(int flags) {
        boolean selected = probe(0, flags, -1);
        if (!selected) {
            mark(25);
            selected = probe(1, flags, -1);
        }
        mark(selected ? 40 : 50);
        return selected ? 7 : 9;
    }

    public static int copied(int first, int value, int replacement) {
        if (first != 0) return value;
        value = replacement;
        if (replacement != 0) return value;
        return -1;
    }

    public static int foreign(int flags, int skip) {
        boolean selected = skip != 0 || probe(0, flags, -1)
                || probe(1, flags, -1);
        mark(selected ? 40 : 50);
        return selected ? 7 : 9;
    }

    public static int cyclic(int flags) {
        if (probe(0, flags, -1)) return 7;
        if (probe(1, flags, -1)) {
            while (true) { }
        }
        return 9;
    }
}
