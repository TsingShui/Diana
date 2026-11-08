package fixture;

/**
 * A synchronized block holds the monitor across a read-protection catch that
 * closes the resource and rethrows. The normal path releases the monitor before
 * its own close.
 */
public final class SyncCleanupRethrow {
    public static native int open(int id);
    public static native int read(int handle);
    public static native void close(int handle);

    public static int guarded(Object lock, int mode) throws Throwable {
        int res = open(0);
        int total = 0;
        synchronized (lock) {
            try {
                total = read(res);
                if (mode == 1) {
                    throw new IllegalStateException("body");
                }
            } catch (Throwable primary) {
                try {
                    close(res);
                } catch (Throwable secondary) {
                    primary.addSuppressed(secondary);
                }
                throw primary;
            }
        }
        close(res);
        return total;
    }
}
