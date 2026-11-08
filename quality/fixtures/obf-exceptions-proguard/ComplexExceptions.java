package fixture;

import java.util.ArrayList;
import java.util.List;

/** Combined fixture: nested try, finally, rethrow with cause, TWR in a loop. */
public final class ComplexExceptions {
    static final class Res implements AutoCloseable {
        private final int id;
        private boolean closed;

        Res(int id) {
            this.id = id;
        }

        int read() {
            if (closed) {
                throw new IllegalStateException("closed " + id);
            }
            return id * 2;
        }

        @Override
        public void close() {
            closed = true;
        }
    }

    private final List<String> log = new ArrayList<>();

    public int parseOrThrow(String text) {
        try {
            return Integer.parseInt(text.trim());
        } catch (NumberFormatException error) {
            log.add("bad");
            throw new IllegalArgumentException("parse:" + text, error);
        }
    }

    public int nestedFinally(int value) {
        int acc = 0;
        try {
            try {
                acc = 10 / value;
            } finally {
                log.add("inner");
                acc = acc + 1;
            }
        } catch (ArithmeticException error) {
            log.add("catch");
            return -1;
        } finally {
            log.add("outer");
        }
        return acc;
    }

    public int resources(int count) {
        int total = 0;
        for (int index = 0; index < count; index++) {
            try (Res res = new Res(index)) {
                total += res.read();
            }
        }
        return total;
    }

    public int multiAndInstance(Object input) {
        try {
            if (input instanceof Number) {
                return ((Number) input).intValue();
            }
            return Integer.parseInt(input.toString());
        } catch (NumberFormatException | NullPointerException error) {
            log.add("multi");
            return -2;
        } catch (RuntimeException error) {
            return -3;
        }
    }
}
