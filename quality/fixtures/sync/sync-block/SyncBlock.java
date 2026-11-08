package fixture;

/** Atomic fixture: synchronized block, including an early return inside it. */
public final class SyncBlock {
    private static final Object LOCK = new Object();
    private static int counter;

    public static int add(int delta) {
        synchronized (LOCK) {
            counter += delta;
            return counter;
        }
    }

    public static void clearIfPositive() {
        synchronized (LOCK) {
            if (counter > 0) {
                counter = 0;
            }
        }
    }

    public static int nested(int delta) {
        synchronized (LOCK) {
            synchronized (LOCK) {
                counter += delta;
            }
            return counter;
        }
    }
}
