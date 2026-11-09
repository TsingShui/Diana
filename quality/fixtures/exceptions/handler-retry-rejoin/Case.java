package fixture;

/** Independently authored fast-return and ignored-catch retry reduction. */
public final class RetryRejoin {
    static native int fast(int mode);
    static native void reset(int row);
    static native int attempt(int row, int mode);
    static int run(int mode, int target, int limit) {
        try { return fast(mode); }
        catch (IllegalArgumentException ignored) { }
        int row = 0;
        while (row < limit) {
            reset(row);
            try {
                int result = attempt(row, mode);
                if (result == target) { return result; }
            } catch (IllegalStateException ignored) { }
            row++;
        }
        return -1;
    }
}
