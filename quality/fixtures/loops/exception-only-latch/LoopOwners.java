package fixture;

public final class LoopOwners {
    public static native int choose(int remaining, int mode);
    public static native int first(int value);
    public static native int second(int value);
    public static native int alternate(int value);
    public static native int stop(int value);
    public static native void retry(int value);
    public static native int empty(int value);

    public static int dispatch(int remaining, int mode) {
        while (remaining > 0) {
            try {
                switch (choose(remaining, mode)) {
                    case 0:
                    case 1: return second(first(remaining));
                    case 2: return alternate(remaining);
                    default: return stop(remaining);
                }
            } catch (IllegalStateException problem) {
                retry(remaining);
                remaining--;
            }
        }
        return empty(remaining);
    }
}
