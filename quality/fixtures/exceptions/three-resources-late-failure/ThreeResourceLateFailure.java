package fixture;

/**
 * Three nested resources. Failures may happen during a later acquisition, in the
 * body, or in any normal/cleanup close; each close can fail independently.
 */
public final class ThreeResourceLateFailure {
    public static native int open(int id);
    public static native int read(int handle);
    public static native void close(int handle);

    public static int openThree(int unused) throws Throwable {
        int total = 0;
        int first = open(0);
        try {
            int second = open(1);
            try {
                int third = open(2);
                try {
                    total = read(first) + read(second) + read(third);
                } catch (Throwable primary) {
                    try {
                        close(third);
                    } catch (Throwable secondary) {
                        primary.addSuppressed(secondary);
                    }
                    throw primary;
                }
                close(third);
            } catch (Throwable primary) {
                try {
                    close(second);
                } catch (Throwable secondary) {
                    primary.addSuppressed(secondary);
                }
                throw primary;
            }
            close(second);
        } catch (Throwable primary) {
            try {
                close(first);
            } catch (Throwable secondary) {
                primary.addSuppressed(secondary);
            }
            throw primary;
        }
        close(first);
        return total;
    }
}
