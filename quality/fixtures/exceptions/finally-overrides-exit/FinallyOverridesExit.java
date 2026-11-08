package fixture;

/**
 * An explicit finally whose own effects replace a pending exit: a close failure
 * replaces the body exception or return, a return in the finally replaces either,
 * and a throw in the finally replaces the body exception.
 */
public final class FinallyOverridesExit {
    public static native int open(int id);
    public static native int read(int handle);
    public static native void close(int handle);

    public static int replaceExit(int mode) throws Throwable {
        int res = open(0);
        try {
            if ((mode & 1) != 0) {
                throw new IllegalStateException("body");
            }
            read(res);
            return 5;
        } finally {
            close(res);
            if ((mode & 2) != 0) {
                return -7;
            }
            if ((mode & 4) != 0) {
                throw new IllegalArgumentException("finally");
            }
        }
    }
}
