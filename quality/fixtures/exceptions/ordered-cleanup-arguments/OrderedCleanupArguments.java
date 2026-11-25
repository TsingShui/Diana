package fixture;

public final class OrderedCleanupArguments {
    public static native int read(int handle);
    public static native void mark(int result);
    public static native void close(int first, int second);

    public static int run(int first, int second) throws Throwable {
        int a;
        try {
            a = read(first);
        } finally {
            close(first, second);
        }

        int b;
        try {
            b = read(second);
        } catch (Throwable thrown) {
            mark(a);
            close(first, second);
            throw thrown;
        }
        mark(a);
        close(second, first);
        return a + b;
    }
}
