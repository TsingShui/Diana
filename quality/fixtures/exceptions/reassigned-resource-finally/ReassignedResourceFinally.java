package fixture;

/**
 * Manual cleanup with an acquisition argument, a resource slot reassigned inside
 * the protected region and a conditional finally that closes whichever handle is
 * still owned.
 */
public final class ReassignedResourceFinally {
    public static native int read(int handle);
    public static native int open(int id);
    public static native void close(int handle);

    public static int rotate(int mode) throws Throwable {
        int seed = read(0);
        int current = open(seed);
        boolean acquired = true;
        int total = 0;
        try {
            total = read(current);
            if (mode == 1) {
                acquired = false;
                close(current);
                current = open(seed + 1);
                acquired = true;
                total += read(current);
            }
            return total;
        } finally {
            if (acquired) {
                close(current);
            }
        }
    }
}
