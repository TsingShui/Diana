package fixture;

/** Multiple original case predecessors share one body, not cloned effects. */
public final class CaseEntryJoins {
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

    public static int choice(int key, int flag) {
        if (key == 1) mark(1);
        else if (key == 2) mark(2);
        else if (key == 5) mark(5);
        int value;
        switch (key) {
            case 1: case 2: case 3: case 5:
                mark(3);
                if (flag == 0) { mark(6); value = 10; }
                else { mark(7); value = 20; }
                break;
            case 4: mark(4); value = 40; break;
            default: mark(9); value = -1;
        }
        mark(50);
        return value;
    }

    public static boolean matches(int candidate, int match) {
        mark(80 + candidate);
        return candidate == match;
    }

    public static int nonCaseTail(int key, int match) {
        int value;
        switch (key) {
            case 1: value = matches(1, match) ? 10 : -2; break;
            case 2: value = matches(2, match) ? 20 : -2; break;
            case 3: value = matches(3, match) ? 30 : -2; break;
            default: value = -1;
        }
        return value;
    }

    public static int route(int key, int flag, int mode) {
        int value;
        try {
            // Smali encodes these prefixes and their shared body as one switch:
            // case1 and case2 jump to the original case3 entry.
            if (key == 1) mark(1);
            else if (key == 2) mark(2);
            if (key == 1 || key == 2 || key == 3) {
                try { value = read(0, mode); }
                catch (NumberFormatException | NullPointerException error) {
                    mark(20);
                    value = 100;
                }
                switch (flag) {
                    case 0: mark(4); value += 10; break;
                    default: mark(5); value -= 10; break;
                }
            } else {
                mark(9);
                value = -1;
            }
        } finally { cleanup(0, mode); }
        mark(40);
        return value;
    }

    public static int parameterJoin(int key, int seed) {
        switch (key) {
            case 1:
                mark(1); seed += 1;
                mark(3); seed += 3;
                break;
            case 2:
                mark(2); seed += 2;
            case 3:
                mark(3); seed += 3;
                break;
            default:
                mark(9); seed = -1;
        }
        return seed;
    }

    public static int literalCycle(int key) {
        int left = 2;
        if (key != 1 && key != 2) { mark(9); return left; }
        int phase = key;
        while (true) {
            mark(phase);
            if (left == 0) break;
            left--;
            phase = 3 - phase;
        }
        return left;
    }

    public static int cyclic(int key, int left) {
        if (key != 1 && key != 2) { mark(9); return left; }
        // Explanation of alternating original Smali case labels, not DEX state.
        int phase = key;
        while (true) {
            mark(phase);
            if (left == 0) break;
            left--;
            phase = 3 - phase;
        }
        return left;
    }
}
