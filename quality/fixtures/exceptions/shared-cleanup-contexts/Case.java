package fixture;

/** Independently authored explanation of the paired original control flow. */
final class CleanupContexts {
    static native ContextHandle acquire(int mode);
    static native void log(Throwable failure);
    static native void note(int value);

    // Smali has separate pre/post acquisition entries for each clause. Their
    // pure adapters join shared logging, interrupted rethrow and cleanup tails.
    static int split(int mode) throws Throwable {
        ContextHandle handle = null;
        try {
            try {
                handle = acquire(mode);
                if (!handle.ready()) return -1;
                int value = handle.read();
                if (value < 0) return -1;
                return value;
            } catch (java.io.InterruptedIOException stop) {
                throw stop;
            } catch (java.io.IOException recover) {
                log(recover);
                return -1;
            }
        } finally {
            if (handle != null) handle.close();
        }
    }

    static int resource(int mode) throws Throwable {
        try (ContextHandle handle = acquire(mode)) {
            int value = handle.read();
            if (value > 0) { note(value); return value; }
            return 0;
        } catch (java.io.IOException recover) {
            log(recover);
            return -7;
        }
    }
}

final class ContextHandle implements AutoCloseable {
    native boolean ready();
    native int read();
    public native void close();
}
