package fixture;

import java.io.IOException;

public final class ExceptionJoinTypes {
    public static void run() throws IOException {
        Exception last = null;
        for (int step = nextStep(); step != 0; step = nextStep()) {
            try {
                write(step);
            } catch (IOException failure) {
                last = failure;
                seenIo();
            } catch (RuntimeException failure) {
                last = failure;
                seenRuntime();
            }
        }
        rethrow(last);
    }

    public static void runShared() throws IOException {
        Exception last = null;
        for (int step = nextStep(); step != 0; step = nextStep()) {
            try {
                write(step);
            } catch (IOException | RuntimeException failure) {
                last = failure;
                seenEither();
            }
        }
        rethrow(last);
    }

    // The Smali places this path directly after the loop in each method.
    private static void rethrow(Exception last) throws IOException {
        if (last == null) return;
        if (last instanceof IOException) throw (IOException) last;
        throw (RuntimeException) last;
    }

    private static native int nextStep();
    private static native void write(int step) throws IOException;
    private static native void seenIo();
    private static native void seenRuntime();
    private static native void seenEither();
}
