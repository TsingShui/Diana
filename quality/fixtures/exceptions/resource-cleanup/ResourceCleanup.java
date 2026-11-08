package fixture;

/** Integer handles stand for resources; native helpers have scenario-defined effects. */
public final class ResourceCleanup {
    public static native int open(int id);
    public static native int read(int handle);
    public static native void close(int handle);

    public static int loop(int count) throws Throwable {
        int total = 0;
        for (int index = 0; index < count; index++) {
            int resource = open(index);
            try {
                total += read(resource);
            } catch (Throwable primary) {
                try {
                    close(resource);
                } catch (Throwable secondary) {
                    primary.addSuppressed(secondary);
                }
                throw primary;
            }
            close(resource);
        }
        return total;
    }

    public static int nested(int extra) throws Throwable {
        int first = open(1);
        int result;
        try {
            int second = open(2);
            try {
                result = read(first) + read(second) + extra;
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
        return result;
    }

    public static int sharedHandler(int count) throws Throwable {
        int resource = open(0);
        int total = 0;
        try {
            for (int index = 0; index < count; index++) {
                total += read(resource);
            }
            total += read(resource);
        } catch (Throwable primary) {
            try {
                close(resource);
            } catch (Throwable secondary) {
                primary.addSuppressed(secondary);
            }
            throw primary;
        }
        close(resource);
        return total;
    }
}
