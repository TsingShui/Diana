package fixture;

/**
 * A multi-catch handler and a broader typed handler share one finally cleanup
 * with the normal path. The multi-catch handler owns a nested try/finally over a
 * second resource.
 */
public final class TypedCatchSharedCleanup {
    public static native int open(int id);
    public static native int read(int handle);
    public static native void close(int handle);
    public static native int parse(String text);

    public static int classify(String text, int mode) throws Throwable {
        int res = open(0);
        int result = 0;
        try {
            try {
                if (mode == 1) {
                    throw new NumberFormatException("mode");
                }
                if (mode == 2) {
                    throw new NullPointerException("mode");
                }
                if (mode == 3) {
                    throw new IllegalStateException("mode");
                }
                result = parse(text);
            } catch (NumberFormatException | NullPointerException error) {
                int detail = open(1);
                try {
                    result = read(detail) + 100;
                } finally {
                    close(detail);
                }
            } catch (RuntimeException error) {
                result = -2;
            }
        } finally {
            close(res);
        }
        return result;
    }
}
