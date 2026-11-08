package fixture;

import java.io.Closeable;

/** Atomic fixture: try-with-resources over two resources. */
public final class ExceptionTwoResources {
    static final class Res implements Closeable {
        private final int value;
        private boolean closed;

        Res(int value) {
            this.value = value;
        }

        int get() {
            if (closed) {
                throw new IllegalStateException("closed");
            }
            return value;
        }

        @Override
        public void close() {
            closed = true;
        }
    }

    public static int sum(int extra) {
        try (Res first = new Res(1); Res second = new Res(2)) {
            return first.get() + second.get() + extra;
        }
    }
}
