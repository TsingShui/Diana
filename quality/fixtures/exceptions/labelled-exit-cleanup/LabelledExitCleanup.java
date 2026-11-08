package fixture;

/** Try-with-resources across labelled loop exits. */
public final class LabelledExitCleanup {
    public static native int open(int id);
    public static native int read(int handle);
    public static native void close(int handle);

    static final class Res implements AutoCloseable {
        final int handle;

        Res(int handle) {
            this.handle = handle;
        }

        @Override
        public void close() {
            LabelledExitCleanup.close(handle);
        }
    }

    public static int scan(int rows, int limit) throws Throwable {
        int total = 0;
        outer:
        for (int row = 0; row < rows; row++) {
            try (Res res = new Res(open(row))) {
                for (int col = 0; col < 4; col++) {
                    int value = read(res.handle);
                    if (value == limit) {
                        continue outer;
                    }
                    if (value < 0) {
                        break outer;
                    }
                    total += value;
                    if (total > 100) {
                        return total;
                    }
                }
            }
        }
        return total;
    }
}
