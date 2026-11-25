package fixture;

public final class SequentialCleanupOrigins {
    public static native int read(int handle);
    public static native void close(int handle);

    public static int readBoth(int first, int second) {
        int a;
        try {
            a = read(first);
        } finally {
            close(first);
        }
        int b;
        try {
            b = read(second);
        } catch (Throwable pending) {
            close(first);
            throw pending;
        }
        close(second);
        return a + b;
    }

    public static int closeBoth(int first, int second) {
        int a;
        try {
            a = read(first);
        } finally {
            close(first);
        }
        int b;
        try {
            b = read(second);
        } finally {
            close(second);
        }
        return a + b;
    }
}
