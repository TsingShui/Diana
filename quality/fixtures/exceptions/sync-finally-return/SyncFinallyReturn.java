package fixture;

/**
 * A finally inside a synchronized block. The close runs while the monitor is
 * held on both the normal path and the exceptional path; the monitor is released
 * before either the return or the rethrow. An early return skips a following
 * effect inside the same synchronized block.
 */
public final class SyncFinallyReturn {
    public static native int open(int id);
    public static native int read(int handle);
    public static native void close(int handle);
    public static native void record(int value);

    public static int syncFinally(Object lock, int mode) throws Throwable {
        int res = open(0);
        int total;
        synchronized (lock) {
            try {
                total = read(res);
                if (mode == 1) {
                    return total;
                }
            } finally {
                close(res);
            }
            record(0);
        }
        return total;
    }
}
