package fixture;

/** Already-emitted exits must not acquire a second planned jump. */
public final class TailCompletion {
    public static int trace;
    public static void mark(int event) { trace = trace * 31 + event; }
    public static int read(int value, int mode) {
        mark(100 + value);
        if (mode == 1) throw new IllegalStateException("read");
        return value;
    }

    public static boolean matches(int candidate, int match) {
        mark(80 + candidate);
        return candidate == match;
    }

    public static int decorated(int key, int match, int mode) {
        int selector;
        switch (key) {
            case 0: selector = matches(0, match) ? 0 : -1; break;
            case 1: selector = matches(1, match) ? 1 : -1; break;
            case 2: selector = matches(2, match) ? 2 : -1; break;
            default: selector = -1;
        }
        int value;
        switch (selector) {
            case 0: value = 10; break;
            case 1: value = 20; break;
            case 2: value = 30; break;
            default: value = -1;
        }
        int result;
        try { result = read(value, mode); }
        finally { mark(90); }
        mark(91);
        return result;
    }

    public static int select(int key, int match, int mode) {
        int value;
        try {
            switch (key) {
                case 0:
                    mark(10);
                    value = match == 0 ? read(10, mode) : -1;
                    break;
                case 1:
                    mark(11);
                    value = match == 1 ? read(20, mode) : -1;
                    break;
                case 2:
                    mark(12);
                    value = match == 2 ? read(30, mode) : -1;
                    break;
                default:
                    value = -1;
            }
        } finally { mark(30); }
        mark(40);
        return value;
    }

    public static int caught(int flag) {
        int value;
        try {
            mark(50);
            if (flag == 0) throw new NumberFormatException();
            value = 7;
        } catch (NumberFormatException error) {
            mark(51);
            value = 9;
        }
        mark(52);
        return value;
    }

    public static int breakScope(int key) {
        switch (key) {
            case 0: mark(60); break;
            default: mark(61); break;
        }
        mark(62);
        return 7;
    }
}
