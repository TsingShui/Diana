package fixture;

public final class MemberForest {
    static native int tick(int value);

    public static int interleaved(int flag) {
        int value;
        if (flag != 0) { value = 1; value += 3; }
        else { value = 2; value += 5; }
        return value;
    }

    public static int nested(int kind, int start) {
        if (kind < 0) return -1;
        int value = start;
        while (value < 4) {
            try {
                switch (kind) {
                    case 0: case 1: value = tick(value); break;
                    case 2: value += 2; break;
                    default: value++; break;
                }
            } catch (RuntimeException failure) { value++; }
        }
        return value;
    }

    public static int locked(Object lock, int start) {
        int value = start;
        synchronized (lock) {
            while (value < 3) { value = tick(value); }
        }
        return value;
    }
}
