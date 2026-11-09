package fixture;

/** Independent response-finally and resource-suppression reproduction. */
public final class ResponseClose {
    static native ViewHandle acquire(int mode) throws java.io.IOException;
    static native void log(Throwable failure);

    static int response(int mode) throws java.io.IOException {
        ViewHandle response = null;
        try {
            response = acquire(mode);
            if (response.ready()) { return response.read(); }
        } catch (java.io.InterruptedIOException failure) {
            throw failure;
        } catch (java.io.IOException failure) {
            log(failure);
        } finally {
            if (response != null) { response.close(); }
        }
        return -1;
    }

    static int resource(int mode) throws java.io.IOException {
        try (ViewHandle resource = acquire(mode)) { return resource.read(); }
    }
}

class ViewHandle implements AutoCloseable {
    native boolean ready() throws java.io.IOException;
    native int read() throws java.io.IOException;
    public native void close() throws java.io.IOException;
}
