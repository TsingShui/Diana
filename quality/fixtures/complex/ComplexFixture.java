package fixture;

import java.io.Closeable;
import java.io.IOException;
import java.util.ArrayList;
import java.util.List;

/**
 * Complex quality fixture: the shapes that only appear in real code.
 *
 * <p>Enum switch, string switch with side effects, nested try with finally,
 * multi-catch, two-resource try-with-resources, synchronized method, break and
 * continue in one loop, builder with a conditional skip, varargs, and a try that
 * lives inside a loop.
 *
 * <p>The Java source is the readable reference. The DEX is generated locally with
 * {@code javac -g} + {@code d8} and committed next to it so a review needs no
 * toolchain.
 */
public final class ComplexFixture {
    enum Mode {
        FAST,
        SAFE,
        LEGACY
    }

    static final class Res implements Closeable {
        private final int value;
        private boolean closed;

        Res(int value) {
            this.value = value;
        }

        int get() throws IOException {
            if (closed) {
                throw new IOException("closed");
            }
            return value;
        }

        @Override
        public void close() {
            closed = true;
        }
    }

    private final List<String> log = new ArrayList<>();
    private Mode mode = Mode.SAFE;

    public int dispatch(Mode requested, int value) {
        switch (requested) {
            case FAST:
                return value * 2;
            case SAFE:
                return value;
            case LEGACY:
                return value - 1;
            default:
                return -1;
        }
    }

    public String route(String operation, int value) {
        switch (operation) {
            case "inc":
                log.add("inc");
                return "i" + (value + 1);
            case "dec":
                return "d" + (value - 1);
            default:
                log.add("default");
                return "?";
        }
    }

    public int nestedTry(int divisor) {
        try {
            try {
                return 100 / divisor;
            } catch (ArithmeticException error) {
                return -1;
            }
        } finally {
            log.add("done");
        }
    }

    public int multiCatch(Object input) {
        try {
            return Integer.parseInt(input.toString());
        } catch (NumberFormatException | NullPointerException error) {
            return -2;
        }
    }

    public int twoResources(int extra) throws IOException {
        try (Res first = new Res(1); Res second = new Res(2)) {
            return first.get() + second.get() + extra;
        }
    }

    public synchronized int guarded(int delta) {
        mode = Mode.LEGACY;
        return delta + mode.ordinal();
    }

    public int loopWithBreakContinue(int[] values) {
        int sum = 0;
        for (int value : values) {
            if (value < 0) {
                continue;
            }
            if (value > 100) {
                break;
            }
            sum += value;
        }
        return sum;
    }

    public String build(String[] parts) {
        StringBuilder builder = new StringBuilder();
        for (String part : parts) {
            if (part == null) {
                continue;
            }
            builder.append(part).append(';');
        }
        return builder.toString();
    }

    public int varargs(int base, int... extra) {
        int total = base;
        for (int value : extra) {
            total += value;
        }
        return total;
    }

    public int tryInsideLoop(int count) {
        int total = 0;
        for (int index = 0; index < count; index++) {
            try {
                total += 10 / (index - 2);
            } catch (ArithmeticException error) {
                total -= 1;
            }
        }
        return total;
    }
}
