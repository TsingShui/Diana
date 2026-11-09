package fixture;

public final class OperationRecords {
    static native int first(int seed);
    static native int later();
    static native void gate(int a, int b);

    public static int protectedNew(int seed) {
        int saved = 0;
        try {
            saved = first(seed);
            new RecordBox(saved, later());
            return saved + 1;
        } catch (RuntimeException error) {
            return saved;
        }
    }

    public static int locked(Object lock, int seed) {
        synchronized (lock) {
            int value = first(seed);
            if (value < 0) return value;
            return later();
        }
    }
}

final class RecordBox {
    RecordBox(int a, int b) {
        OperationRecords.gate(a, b);
    }
}
