package fixture;

import java.io.Closeable;
import java.io.IOException;

/**
 * Diana quality measurement corpus.
 *
 * <p>Each method exercises one decompilation shape we currently lose or must
 * recover: parameter naming, classic/enhanced/do-while loops, ternary,
 * short-circuit condition, packed switch, string switch, try/finally,
 * try-with-resources, synchronized block, nested loops and string building.
 *
 * <p>The Java source is the readable reference. The DEX used for measurement is
 * generated locally (javac -g + d8) into {@code .cache/}; nothing generated enters
 * the repository.
 */
public final class QualityFixtures {
    private static final Object LOCK = new Object();
    private static int counter;

    private QualityFixtures() {}

    static final class Resource implements Closeable {
        private boolean closed;

        void touch() {
            if (closed) {
                throw new IllegalStateException("closed");
            }
        }

        @Override
        public void close() {
            closed = true;
        }
    }

    public static int paramNaming(int count, String label) {
        int total = 0;
        for (int index = 0; index < count; index++) {
            total += index;
        }
        return total + label.length();
    }

    public static int loopFor(int limit) {
        int sum = 0;
        for (int i = 0; i < limit; i++) {
            sum += i * 2;
        }
        return sum;
    }

    public static int enhancedFor(int[] values) {
        int sum = 0;
        for (int value : values) {
            sum += value;
        }
        return sum;
    }

    public static int doWhile(int n) {
        int i = 0;
        do {
            i++;
        } while (i < n);
        return i;
    }

    public static int ternaryMax(int left, int right) {
        return left > right ? left : right;
    }

    public static boolean shortCircuitOr(boolean first, boolean second) {
        return first || second;
    }

    public static int packedSwitch(int code) {
        switch (code) {
            case 1:
                return 10;
            case 2:
                return 20;
            case 3:
                return 30;
            case 4:
                return 40;
            default:
                return -1;
        }
    }

    public static String stringSwitch(String operation) {
        switch (operation) {
            case "add":
                return "plus";
            case "sub":
                return "minus";
            default:
                return "unknown";
        }
    }

    public static int tryFinally(int value) {
        int acc = 0;
        try {
            acc += value;
        } finally {
            acc += 1;
        }
        return acc;
    }

    public static int tryWithResources() throws IOException {
        try (Resource resource = new Resource()) {
            resource.touch();
            return 42;
        }
    }

    public static int syncBlock(int delta) {
        synchronized (LOCK) {
            counter += delta;
            return counter;
        }
    }

    public static int arraySum2d(int[][] matrix) {
        int sum = 0;
        for (int[] row : matrix) {
            for (int value : row) {
                sum += value;
            }
        }
        return sum;
    }

    public static String stringConcat(String left, String right, int count) {
        return left + right + count;
    }
}
