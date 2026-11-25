package fixture;

import java.io.IOException;

public final class HandlerCleanupBoundary {
    public static native int acquire(int key) throws IOException;
    public static native void probe();
    public static native int read(int handle) throws IOException;
    public static native void log(Throwable failure);
    public static native void audit(int handle);
    public static native void close(int handle);

    public static int run(int key) throws IOException {
        int handle = acquire(key);
        probe();
        int result;
        try {
            result = read(handle);
        } catch (IOException error) {
            try {
                log(error);
            } catch (Throwable failure) {
                if (handle != 0) close(handle);
                throw failure;
            }
            audit(handle);
            if (handle != 0) close(handle);
            return -2;
        } catch (Throwable failure) {
            if (handle != 0) close(handle);
            throw failure;
        }
        if (handle != 0) close(handle);
        return result;
    }
}
